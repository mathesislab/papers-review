//! The windows: the start window, and one window per project, which holds
//! web views (its page now; the review pane over it later). A window's label
//! names its project in `AppState::windows`.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use review_core::{Colour, Project};
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, Manager, WebviewBuilder, WebviewUrl,
    WebviewWindowBuilder, Window,
};

use crate::{AppState, menu};

/// The start window: the projects opened last, and Open.
pub const START: &str = "start";

/// The label of the page of a project's window.
pub fn page(window: &str) -> String {
    format!("{window}-page")
}

pub fn show_start(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(START) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, START, WebviewUrl::App("/".into()))
        .title("Papers Review")
        .inner_size(560.0, 460.0)
        .resizable(false)
        .center()
        .build();
}

/// The windows open as the app last quit, else the start window.
pub fn reopen(app: &AppHandle) {
    let open = app.state::<AppState>().saved.lock().unwrap().open.clone();
    let mut any = false;
    for root in open {
        match open_project(app, &root) {
            Ok(()) => any = true,
            Err(e) => {
                eprintln!("{e}");
                let state = app.state::<AppState>();
                state.saved.lock().unwrap().closed(&root);
                state.save();
            }
        }
    }
    if !any {
        show_start(app);
    }
}

/// Open the project of a folder in a window of its own, or bring forward the
/// window that shows it. A project without a colour takes the next one not in
/// use (written in its `review.toml`).
pub fn open_project(app: &AppHandle, folder: &Path) -> Result<(), String> {
    let root = folder
        .canonicalize()
        .map_err(|e| format!("{}: {e}", folder.display()))?;
    let state = app.state::<AppState>();
    if let Some(window) = window_of(app, &root) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }
    let mut project = Project::open(&root).map_err(|e| e.to_string())?;
    if project.manifest.colour.is_none() {
        let in_use: Vec<Colour> = state
            .projects
            .lock()
            .unwrap()
            .values()
            .filter_map(|p| p.manifest.colour)
            .collect();
        project
            .set_colour(Colour::next(&in_use))
            .map_err(|e| e.to_string())?;
    }
    let label = format!("project-{}", state.next.fetch_add(1, Ordering::Relaxed));
    let title = project.name();
    state.projects.lock().unwrap().insert(root.clone(), project);
    state.windows.lock().unwrap().insert(label.clone(), root.clone());
    if let Err(e) = make(app, &label, &title) {
        state.windows.lock().unwrap().remove(&label);
        state.projects.lock().unwrap().remove(&root);
        return Err(e.to_string());
    }
    state.saved.lock().unwrap().opened(&root);
    state.save();
    menu::update(app);
    if let Some(start) = app.get_webview_window(START) {
        let _ = start.close();
    }
    Ok(())
}

fn make(app: &AppHandle, label: &str, title: &str) -> tauri::Result<()> {
    let window = WindowBuilder::new(app, label)
        .title(title)
        .inner_size(1200.0, 800.0)
        .min_inner_size(640.0, 400.0)
        .center()
        .build()?;
    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    window.add_child(
        WebviewBuilder::new(page(label), WebviewUrl::App("project".into())).auto_resize(),
        LogicalPosition::new(0.0, 0.0),
        size,
    )?;
    Ok(())
}

/// The window that shows the project of `root`.
fn window_of(app: &AppHandle, root: &Path) -> Option<Window> {
    let state = app.state::<AppState>();
    let windows = state.windows.lock().unwrap();
    let label = windows.iter().find(|(_, r)| *r == root)?.0;
    app.get_window(label)
}

/// The project of a window, by its label.
pub fn root_of(app: &AppHandle, window: &str) -> Option<PathBuf> {
    app.state::<AppState>().windows.lock().unwrap().get(window).cloned()
}

/// A project's window closed by hand: it does not open at the next start.
pub fn closing(app: &AppHandle, window: &str) {
    if let Some(root) = root_of(app, window) {
        let state = app.state::<AppState>();
        state.saved.lock().unwrap().closed(&root);
        state.save();
    }
}

/// A project's window gone (closed, or the app quitting): its project goes
/// when no other window shows it.
pub fn closed(app: &AppHandle, window: &str) {
    let state = app.state::<AppState>();
    let mut windows = state.windows.lock().unwrap();
    if let Some(root) = windows.remove(window)
        && !windows.values().any(|r| *r == root)
    {
        state.projects.lock().unwrap().remove(&root);
    }
}

/// The project of `root` changed (its colour, its files): its windows are told.
pub fn changed(app: &AppHandle, root: &Path) {
    let labels: Vec<String> = app
        .state::<AppState>()
        .windows
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, r)| *r == root)
        .map(|(label, _)| label.clone())
        .collect();
    for label in labels {
        let _ = app.emit_to(EventTarget::webview(page(&label)), "project-changed", ());
    }
}
