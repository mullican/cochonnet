//! Sending a generated PDF to a printer.
//!
//! The two platforms are genuinely different, not just cosmetically:
//!
//! - macOS hands the file to CUPS and it prints. There is no dialog, which is
//!   what a scorekeeper wants when the same standings sheet goes out after
//!   every round.
//! - iPadOS has no such path. AirPrint is only reachable through
//!   `UIPrintInteractionController`, which always shows Apple's sheet, and on
//!   iPad that sheet is a popover that must be anchored to a rect - presenting
//!   it the iPhone way raises an exception.

use tauri::AppHandle;

/// Writes the PDF somewhere the platform's print path can reach it.
#[cfg(desktop)]
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

#[cfg(desktop)]
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
