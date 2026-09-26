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
//! - Windows has no CUPS at all, so `lpstat`/`lp` are simply absent there. The
//!   shell is the route instead: `ShellExecuteW` with the `print` verb hands
//!   the file to whatever program owns PDFs, which is Edge on a stock Windows
//!   11 and shows its print preview. Handlers are free to interpret the verb
//!   their own way and some print straight to the default printer, so the
//!   promise on Windows is "it reaches a printer", not "a panel always opens".
//!   A handler that has no `print` verb registered falls back to `open`, which
//!   puts the PDF on screen for the operator to print from.
//! - Other desktops keep the CUPS `lp` route, which is all they have here.

use tauri::AppHandle;

/// Turns a document title into something the filesystem will take.
///
/// The name is built from the tournament's own, and an operator can call a
/// tournament anything: `Doubles 2026: Spring/Fall` is an ordinary title and an
/// illegal file name on Windows, where `\ / : * ? " < > |` are all reserved, as
/// are trailing dots and spaces. Linux only objects to `/`, but applying the
/// stricter rules everywhere costs nothing and keeps the spooled name the same
/// on both.
///
/// Compiled on macOS as well, where nothing spools to a file, because it is
/// pure string logic and this is the only platform the tests actually run on.
#[cfg(desktop)]
#[cfg_attr(all(target_os = "macos", not(test)), allow(dead_code))]
fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if (c as u32) < 0x20 => '-',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if trimmed.is_empty() {
        "document.pdf".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Writes the PDF somewhere the platform's print path can reach it.
#[cfg(all(desktop, not(target_os = "macos")))]
fn spool_to_temp(app: &AppHandle, file_name: &str, data: &[u8]) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;

    let dir = app
        .path()
        .temp_dir()
        .map_err(|e| format!("No temporary directory: {}", e))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(safe_file_name(file_name));
    std::fs::write(&path, data).map_err(|e| format!("Could not stage the PDF: {}", e))?;
    Ok(path)
}

#[cfg(all(desktop, not(target_os = "macos"), not(target_os = "windows")))]
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

/// The printer Windows would use if asked right now, if there is one.
///
/// `GetDefaultPrinterW` is called twice on purpose: the first call is given no
/// buffer and fills in the length it wants, the second fills the buffer. A
/// machine with no printers at all fails the first call and leaves the length
/// at zero, which is the case worth catching - the shell would otherwise open
/// the "add a printer" flow and the operator would never learn why.
#[cfg(target_os = "windows")]
fn default_printer() -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Graphics::Printing::GetDefaultPrinterW;

    let mut len: u32 = 0;
    unsafe {
        let _ = GetDefaultPrinterW(None, &mut len);
    }
    if len == 0 {
        return None;
    }

    let mut buf = vec![0u16; len as usize];
    let got = unsafe { GetDefaultPrinterW(Some(PWSTR(buf.as_mut_ptr())), &mut len) };
    if !got.as_bool() {
        return None;
    }

    // len comes back as the character count including the trailing NUL.
    let end = (len as usize).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn print_pdf(app: AppHandle, file_name: String, data: Vec<u8>) -> Result<(), String> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    // Same courtesy as the CUPS path: say so plainly rather than letting the
    // shell open its own dialog about it.
    if default_printer().is_none() {
        return Err(
            "No default printer is set. Add one in Settings > Bluetooth & devices > \
             Printers & scanners."
                .into(),
        );
    }

    let path = spool_to_temp(&app, &file_name, &data)?;
    let file = HSTRING::from(path.as_os_str());

    // `print` is what the shell offers for documents; `open` is the fallback for
    // a handler that never registered one, and puts the PDF in front of the
    // operator to print by hand. Never SW_HIDE: a handler that answers `print`
    // by showing a print dialog would have it hidden along with everything else.
    let mut last = 0isize;
    for verb in ["print", "open"] {
        let verb = HSTRING::from(verb);
        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        // ShellExecuteW is the old API that returns a fake HINSTANCE: anything
        // above 32 means it launched, anything at or below is an error code.
        last = result.0 as isize;
        if last > 32 {
            return Ok(());
        }
    }

    Err(format!(
        "Windows could not open the PDF to print it (shell error {}). Check that a PDF \
         reader is installed.",
        last
    ))
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

#[cfg(all(test, desktop))]
mod tests {
    use super::safe_file_name;

    /// The spooled name is built from the tournament's, which is free text.
    /// `Doubles 2026: Spring/Fall` is an ordinary thing to call an event and
    /// cannot be written to disk on Windows - the write fails with a bare OS
    /// error and the operator is told only that printing failed.
    #[test]
    fn a_title_windows_would_reject_still_spools() {
        assert_eq!(
            safe_file_name("Doubles 2026: Spring/Fall_court_assignments.pdf"),
            "Doubles 2026- Spring-Fall_court_assignments.pdf"
        );
        assert_eq!(
            safe_file_name(r#"a\b<c>d"e|f*g?h_standings.pdf"#),
            "a-b-c-d-e-f-g-h_standings.pdf"
        );
    }

    /// Windows silently drops trailing dots and spaces from a file name, so a
    /// path built with them does not name the file that ends up on disk.
    #[test]
    fn trailing_dots_and_spaces_come_off() {
        assert_eq!(safe_file_name("Spring Open.  "), "Spring Open");
        assert_eq!(safe_file_name("  Spring Open"), "Spring Open");
    }

    /// A name that sanitises away entirely still has to be something.
    #[test]
    fn a_name_with_nothing_left_falls_back() {
        assert_eq!(safe_file_name("..."), "document.pdf");
        assert_eq!(safe_file_name("   "), "document.pdf");
    }

    /// The ordinary case must pass through untouched, dots in the extension
    /// and all.
    #[test]
    fn an_ordinary_name_is_left_alone() {
        assert_eq!(
            safe_file_name("AIO 2024 Validation_brackets.pdf"),
            "AIO 2024 Validation_brackets.pdf"
        );
    }
}
