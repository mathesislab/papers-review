//! The export of the review: a PDF in the `export/` folder of the project, printed by WebKit from the
//! HTML the review's pane made, and plain text there and on the pasteboard. The PDF is printed from a
//! hidden window of its own (`export-<n>`, the route `/export`): its page asks for the HTML
//! (`export_html`), renders it, and says so (`export_rendered`), and the web view is printed as AppKit
//! paginates it, A4 with margins of 2 cm, to a file.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use tauri::{AppHandle, Manager, Webview, WebviewUrl, WebviewWindowBuilder};

use crate::{AppState, windows};

/// The folder of a project the exports are written to.
pub const FOLDER: &str = "export";

/// How long the page of the hidden window has to render and the web view to be printed.
const TIMEOUT: Duration = Duration::from_secs(20);

/// The paper (points): A4.
const PAPER: (f64, f64) = (595.28, 841.89);
/// The margins of the paper (points): 2 cm.
const MARGIN: f64 = 56.7;

/// An export waiting for its hidden window: the HTML to print, the file to print it to, and where to tell
/// how it went.
pub struct Pending {
    html: String,
    path: PathBuf,
    done: Sender<Result<(), String>>,
}

/// `name` as the stem of a file: not empty, no separator, not hidden.
pub fn check_name(name: &str) -> Result<&str, String> {
    let bad = name.trim().is_empty()
        || name.starts_with('.')
        || name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'));
    if bad { Err(format!("{name:?}: not a name for a file")) } else { Ok(name) }
}

/// The file `<name>.<extension>` in the `export/` folder of the project, which is made.
fn target(root: &Path, name: &str, extension: &str) -> Result<PathBuf, String> {
    let file = format!("{}.{extension}", check_name(name)?);
    let folder = root.join(FOLDER);
    fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    Ok(folder.join(file))
}

/// The review as a PDF in the project's `export/` folder: the path written. `html` is the document, which
/// the page of a hidden window renders and WebKit prints.
pub async fn pdf(app: &AppHandle, root: &Path, name: &str, html: String) -> Result<PathBuf, String> {
    if !cfg!(target_os = "macos") {
        return Err("the PDF export is for macOS only".into());
    }
    let path = target(root, name, "pdf")?;
    let state = app.state::<AppState>();
    let label = format!("export-{}", state.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let (done, wait) = channel();
    // printed to a hidden file beside the target, renamed when it is done: the files panel never lists a half
    let part = path.with_file_name(format!(".{}.part", path.file_name().unwrap_or_default().to_string_lossy()));
    state.exports.lock().unwrap().insert(label.clone(), Pending { html, path: part.clone(), done });
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("export".into()))
        .title("Export")
        .inner_size(PAPER.0, PAPER.1)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| e.to_string());
    let result = match window {
        Ok(_) => tauri::async_runtime::spawn_blocking(move || {
            wait.recv_timeout(TIMEOUT).map_err(|_| "the PDF was not printed in time".to_string())?
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r),
        Err(e) => Err(e),
    };
    state.exports.lock().unwrap().remove(&label);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.destroy();
    }
    let result = result.and_then(|()| fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display())));
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result?;
    windows::changed(app, root);
    Ok(path)
}

/// The HTML for the page of the hidden window `label` to render.
pub fn html(app: &AppHandle, label: &str) -> Result<String, String> {
    let state = app.state::<AppState>();
    let exports = state.exports.lock().unwrap();
    exports.get(label).map(|p| p.html.clone()).ok_or_else(|| format!("{label}: nothing to export"))
}

/// The page of the hidden window rendered the HTML: its web view is printed, and the export told how it went.
pub fn rendered(webview: &Webview) -> Result<(), String> {
    let app = webview.app_handle();
    let label = webview.window().label().to_string();
    let (path, done) = {
        let state = app.state::<AppState>();
        let exports = state.exports.lock().unwrap();
        let pending = exports.get(&label).ok_or_else(|| format!("{label}: nothing to export"))?;
        (pending.path.clone(), pending.done.clone())
    };
    print_to_pdf(webview, &path, move |result| {
        let _ = done.send(result);
    });
    Ok(())
}

/// The web view printed as the pages of a PDF at `path` (A4, margins of 2 cm); `done` is told how it went.
/// The print operation is made on the main thread and run on a thread of its own: WebKit's printing view
/// paginates and draws from the thread that runs it, waiting for the web process, which the main thread
/// serves; run on the main thread it finds no pages, or blank ones.
#[cfg(target_os = "macos")]
pub fn print_to_pdf(webview: &Webview, path: &Path, done: impl FnOnce(Result<(), String>) + Send + 'static) {
    let path = path.to_path_buf();
    let failed = webview.with_webview(move |platform| {
        let operation = operation(platform.inner(), &path);
        std::thread::spawn(move || {
            let ran = operation();
            done(if ran { Ok(()) } else { Err(format!("{}: the print operation failed", path.display())) });
        });
    });
    // the closure did not run: the waiting export times out
    if let Err(e) = failed {
        eprintln!("export: {e}");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn print_to_pdf(_: &Webview, _: &Path, done: impl FnOnce(Result<(), String>) + Send + 'static) {
    done(Err("the PDF export is for macOS only".into()));
}

/// A print operation, which may go to the thread that runs it.
#[cfg(target_os = "macos")]
struct Operation<T>(T);

// SAFETY: made on the main thread and then used by the one thread that runs it, as AppKit allows of a print
// operation with no panels
#[cfg(target_os = "macos")]
unsafe impl<T> Send for Operation<T> {}

#[cfg(target_os = "macos")]
impl<T> Operation<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

/// The print operation of the WKWebView, to run: A4 with margins, the job saved to `path`.
#[cfg(target_os = "macos")]
fn operation(web_view: *mut std::ffi::c_void, path: &Path) -> impl FnOnce() -> bool + Send + use<> {
    use objc2_app_kit::{NSPrintInfo, NSPrintJobDisposition, NSPrintJobSavingURL, NSPrintSaveJob};
    use objc2_foundation::{NSCopying, NSDictionary, NSPoint, NSRect, NSSize, NSString, NSURL};
    use objc2_web_kit::WKWebView;

    // SAFETY: the pointer is the web view of the webview, alive while this runs on the main thread
    let web_view: &WKWebView = unsafe { &*web_view.cast() };
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    // SAFETY: the keys and the disposition are the constants AppKit defines
    let info = unsafe {
        let attributes =
            NSDictionary::from_slices(&[NSPrintJobDisposition, NSPrintJobSavingURL], &[&***NSPrintSaveJob, &***url]);
        // a copy: the shared print info is the app's, and what the user prints with
        let info = NSPrintInfo::sharedPrintInfo().copy();
        info.dictionary().addEntriesFromDictionary(&attributes);
        info
    };
    info.setPaperSize(NSSize::new(PAPER.0, PAPER.1));
    info.setLeftMargin(MARGIN);
    info.setRightMargin(MARGIN);
    info.setTopMargin(MARGIN);
    info.setBottomMargin(MARGIN);
    // SAFETY: called on the main thread, with the print info made above
    let operation = unsafe { web_view.printOperationWithPrintInfo(&info) };
    operation.setShowsPrintPanel(false);
    operation.setShowsProgressPanel(false);
    // AppKit logs an error and finds no pages unless the view is framed before it asks for them; WebKit then
    // sizes it to the pages
    if let Some(view) = operation.view() {
        let size = NSSize::new(PAPER.0 - 2.0 * MARGIN, PAPER.1 - 2.0 * MARGIN);
        view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), size));
    }
    let operation = Operation(operation);
    move || operation.into_inner().runOperation()
}

/// The review as text: written to `<name>.txt` in the project's `export/` folder and copied to the
/// pasteboard; the path written.
pub fn text(app: &AppHandle, root: &Path, name: &str, text: &str) -> Result<PathBuf, String> {
    let path = target(root, name, "txt")?;
    review_core::project::write_atomically(&path, text).map_err(|e| e.to_string())?;
    copy(text);
    windows::changed(app, root);
    Ok(path)
}

/// `text` put on the general pasteboard.
#[cfg(target_os = "macos")]
fn copy(text: &str) {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;

    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    // SAFETY: the type is the constant AppKit defines
    pasteboard.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
}

#[cfg(not(target_os = "macos"))]
fn copy(_: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_a_stem_without_separators() {
        for name in ["review", "Review of egusphere-2026-1234", "v1.2 (draft)", "Überprüfung"] {
            assert_eq!(check_name(name), Ok(name));
        }
        for name in ["", "  ", "a/b", "../review", "a\\b", ".hidden", "a:b", "a\nb", "a\0b"] {
            assert!(check_name(name).is_err(), "{name:?}");
        }
    }

    #[test]
    fn the_files_are_in_the_export_folder_of_the_project() {
        let dir = std::env::temp_dir().join(format!("papers-review-export-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = target(&dir, "review", "pdf").unwrap();
        assert_eq!(path, dir.join("export").join("review.pdf"));
        assert!(dir.join("export").is_dir());
        assert!(target(&dir, "../review", "pdf").is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
