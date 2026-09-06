//! Sending a generated PDF to a printer.
//!
//! Every platform raises the system print UI, so the operator can choose the
//! printer, the page range and the copy count. Reaching it differs by platform:
//!
//! - macOS goes through PDFKit: a `PDFDocument` vends an `NSPrintOperation`
//!   that knows how to paginate the PDF, and that operation shows the standard
//!   panel. Handing the file to `lp` instead does print, but silently and
//!   entirely on the default printer, with no way to say "just page 3".
//! - iPadOS has no such path. AirPrint is only reachable through
//!   `UIPrintInteractionController`, which always shows Apple's sheet, and on
//!   iPad that sheet is a popover that must be anchored to a rect - presenting
//!   it the iPhone way raises an exception.
//! - Other desktops keep the CUPS `lp` route, which is all they have here.

use tauri::AppHandle;

/// Writes the PDF somewhere the platform's print path can reach it.
#[cfg(all(desktop, not(target_os = "macos")))]
fn spool_to_temp(app: &AppHandle, file_name: &str, data: &[u8]) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;

    let dir = app
        .path()
        .temp_dir()
        .map_err(|e| format!("No temporary directory: {}", e))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(file_name);
    std::fs::write(&path, data).map_err(|e| format!("Could not stage the PDF: {}", e))?;
    Ok(path)
}

#[cfg(all(desktop, not(target_os = "macos")))]
#[tauri::command]
pub fn print_pdf(app: AppHandle, file_name: String, data: Vec<u8>) -> Result<(), String> {
    use std::process::Command;

    // Fail with something an operator can act on, rather than letting `lp`
    // swallow the job when no printer has been set up.
    let default_printer = Command::new("lpstat")
        .arg("-d")
        .output()
        .map_err(|e| format!("Could not query printers: {}", e))?;
    let listing = String::from_utf8_lossy(&default_printer.stdout);
    if listing.contains("no system default destination") || listing.trim().is_empty() {
        return Err("No default printer is set. Add one in System Settings > Printers & Scanners.".into());
    }

    let path = spool_to_temp(&app, &file_name, &data)?;

    let output = Command::new("lp")
        .arg("-t")
        .arg(&file_name)
        .arg(&path)
        .output()
        .map_err(|e| format!("Could not run lp: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Printing failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn print_pdf(app: AppHandle, file_name: String, data: Vec<u8>) -> Result<(), String> {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::NSPrintInfo;
    use objc2_foundation::{NSData, NSString};
    use objc2_pdf_kit::{PDFDocument, PDFPrintScalingMode};
    use std::sync::mpsc;

    // AppKit is main-thread only, and a Tauri command runs on a worker. The
    // channel carries back whether the job could be set up at all, so a
    // malformed document surfaces as an error instead of nothing happening.
    let (tx, rx) = mpsc::channel::<Result<(), String>>();

    app.run_on_main_thread(move || {
        let Some(mtm) = MainThreadMarker::new() else {
            let _ = tx.send(Err("Printing has to start on the main thread.".into()));
            return;
        };

        let pdf = NSData::with_bytes(&data);
        let Some(document) = (unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &pdf) })
        else {
            let _ = tx.send(Err("The generated PDF could not be read back for printing.".into()));
            return;
        };

        let operation = unsafe {
            document.printOperationForPrintInfo_scalingMode_autoRotate(
                Some(&NSPrintInfo::sharedPrintInfo()),
                // Court sheets and brackets are laid out to the page already;
                // shrinking an oversized one beats cropping it.
                PDFPrintScalingMode::PageScaleDownToFit,
                true,
                mtm,
            )
        };
        let Some(operation) = operation else {
            let _ = tx.send(Err("Could not start a print job for this document.".into()));
            return;
        };

        // Names the job in the print queue and pre-fills Save as PDF.
        operation.setJobTitle(Some(&NSString::from_str(&file_name)));
        operation.setShowsPrintPanel(true);
        operation.setShowsProgressPanel(true);

        // Answer before raising the panel, not after: runOperation blocks until
        // the operator dismisses it, and the caller should not sit in a pending
        // invoke - with its button stuck in a loading state - for that long.
        let _ = tx.send(Ok(()));
        operation.runOperation();
    })
    .map_err(|e| format!("Could not reach the UI thread: {}", e))?;

    rx.recv()
        .map_err(|_| "The print panel did not open.".to_string())?
}

#[cfg(target_os = "ios")]
#[tauri::command]
pub fn print_pdf(app: AppHandle, file_name: String, data: Vec<u8>) -> Result<(), String> {
    use objc2::MainThreadMarker;
    use objc2_foundation::{NSData, NSString};
    use objc2_ui_kit::{
        UIApplication, UIPrintInfo, UIPrintInfoOutputType, UIPrintInteractionController,
        UIWindowScene,
    };

    // UIKit is main-thread only, and a Tauri command runs on a worker.
    app.run_on_main_thread(move || {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };

        if !UIPrintInteractionController::isPrintingAvailable(mtm) {
            return;
        }

        let controller = UIPrintInteractionController::sharedPrintController(mtm);

        let info = UIPrintInfo::printInfo(mtm);
        info.setJobName(&NSString::from_str(&file_name));
        info.setOutputType(UIPrintInfoOutputType::General);
        controller.setPrintInfo(Some(&info));

        let pdf = NSData::with_bytes(&data);
        unsafe { controller.setPrintingItem(Some(&pdf)) };

        // On iPad the print sheet is a popover and must be anchored. Anchoring
        // it to the centre of the root view puts it over the page the operator
        // just pressed Print on.
        let app_ui = UIApplication::sharedApplication(mtm);
        let view = app_ui
            .connectedScenes()
            .iter()
            .filter_map(|scene| scene.downcast::<UIWindowScene>().ok())
            .flat_map(|scene| scene.windows().iter().collect::<Vec<_>>())
            .find_map(|window| window.rootViewController())
            .and_then(|root| root.view());

        if let Some(view) = view {
            let bounds = view.bounds();
            let anchor = objc2_core_foundation::CGRect {
                origin: objc2_core_foundation::CGPoint {
                    x: bounds.size.width / 2.0,
                    y: bounds.size.height / 2.0,
                },
                size: objc2_core_foundation::CGSize {
                    width: 1.0,
                    height: 1.0,
                },
            };
            unsafe {
                controller.presentFromRect_inView_animated_completionHandler(
                    anchor,
                    &view,
                    true,
                    std::ptr::null_mut(),
                )
            };
        }
    })
    .map_err(|e| format!("Could not reach the UI thread: {}", e))?;

    Ok(())
}

/// Whether this build can print at all. The UI uses it to decide which of
/// Export / Print to offer.
#[tauri::command]
pub fn printing_available() -> bool {
    cfg!(any(desktop, target_os = "ios"))
}

/// Whether this build can write a PDF to a location the user chooses.
///
/// iOS has no such concept: its save dialog exports a copy through the document
/// picker, so the app offers printing there instead.
#[tauri::command]
pub fn file_export_available() -> bool {
    cfg!(desktop)
}
