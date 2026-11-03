//! The windows: the start window, the Settings window, and the windows of the
//! projects, which hold web views (the page, the review pane over it). A
//! window's label names its project in `AppState::windows`.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use review_core::{Colour, Project};
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, WebviewBuilder,
    WebviewUrl, WebviewWindowBuilder, Window,
};

use crate::{AppState, menu, pane, review};

/// The start window: the projects opened last, and Open.
pub const START: &str = "start";

/// The Settings window.
pub const SETTINGS: &str = "settings";

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

/// The Settings window, brought forward or made.
pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings".into()))
        .title("Settings")
        .inner_size(460.0, 220.0)
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

/// The project of `root` loaded, if it is not: opened, given a colour if it
/// has none (the next one not in use, written in its `review.toml`), its
/// review watched.
pub fn load(app: &AppHandle, root: &Path) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.projects.lock().unwrap().contains_key(root) {
        return Ok(());
    }
    let mut project = Project::open(root).map_err(|e| e.to_string())?;
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
    state.projects.lock().unwrap().insert(root.to_path_buf(), project);
    review::watch(app, root);
    Ok(())
}

/// The project of `root` unloaded, when no window shows it.
fn release(app: &AppHandle, root: &Path) {
    let state = app.state::<AppState>();
    if state.windows.lock().unwrap().values().any(|r| r == root) {
        return;
    }
    state.projects.lock().unwrap().remove(root);
    review::unwatch(app, root);
}

/// Open the project of a folder in a window of its own, or bring forward the
/// window that shows it.
pub fn open_project(app: &AppHandle, folder: &Path) -> Result<(), String> {
    let root = canonical(folder)?;
    if let Some(window) = window_of(app, &root) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }
    new_window(app, &root)
}

fn canonical(folder: &Path) -> Result<PathBuf, String> {
    folder
        .canonicalize()
        .map_err(|e| format!("{}: {e}", folder.display()))
}

/// A window for the project of `root`, loaded.
fn new_window(app: &AppHandle, root: &Path) -> Result<(), String> {
    let state = app.state::<AppState>();
    load(app, root)?;
    let label = format!("project-{}", state.next.fetch_add(1, Ordering::Relaxed));
    let title = state.projects.lock().unwrap()[root].name();
    state.windows.lock().unwrap().insert(label.clone(), root.to_path_buf());
    if let Err(e) = make(app, &label, &title) {
        state.windows.lock().unwrap().remove(&label);
        release(app, root);
        return Err(e.to_string());
    }
    state.saved.lock().unwrap().opened(root);
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
    // the review pane, above the page; placed and shown once the page tells where its viewer is
    let pane = window.add_child(
        WebviewBuilder::new(pane::label(label), WebviewUrl::App("pane".into())),
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(1.0, 1.0),
    )?;
    pane.hide()?;
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
    pane::closed(app, window);
    let root = app.state::<AppState>().windows.lock().unwrap().remove(window);
    if let Some(root) = root {
        release(app, &root);
    }
}

/// The windows that show the project of `root`, by their labels.
pub fn of_project(app: &AppHandle, root: &Path) -> Vec<String> {
    app.state::<AppState>()
        .windows
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, r)| *r == root)
        .map(|(label, _)| label.clone())
        .collect()
}

/// The project of `root` changed (its colour, its files): its windows are told.
pub fn changed(app: &AppHandle, root: &Path) {
    for label in of_project(app, root) {
        let _ = app.emit_to(EventTarget::webview(page(&label)), "project-changed", ());
    }
}
