//! The menu bar. The commands of a project's page go to the page of the
//! window in front (event `menu`, the id of the item); the others are the
//! app's.

use std::path::PathBuf;

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, EventTarget, Manager, Wry};
use tauri_plugin_dialog::DialogExt;

use crate::{AppState, pane, windows};

/// The commands of the page, by the id of their item.
const PAGE: &[&str] = &[
    "close-tab",
    "files",
    "zoom-in",
    "zoom-out",
    "zoom-fit",
    "find",
    "find-next",
    "find-previous",
    "quote",
];

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, text: &str, keys: Option<&str>| MenuItem::with_id(app, id, text, true, keys);
    let separator = || PredefinedMenuItem::separator(app);
    let recent = app.state::<AppState>().saved.lock().unwrap().recent.clone();
    let mut recent_items = Vec::new();
    for (i, root) in recent.iter().enumerate() {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned());
        recent_items.push(item(&format!("recent:{i}"), &name, None)?);
    }
    let open_recent = Submenu::with_id(app, "open-recent", "Open Recent", !recent.is_empty())?;
    for i in &recent_items {
        open_recent.append(i)?;
    }
    if !recent.is_empty() {
        open_recent.append(&separator()?)?;
        open_recent.append(&item("clear-recent", "Clear Menu", None)?)?;
    }
    let app_menu = Submenu::with_items(
        app,
        "Papers Review",
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &separator()?,
            &item("settings", "Settings…", Some("CmdOrCtrl+,"))?,
            &separator()?,
            &PredefinedMenuItem::services(app, None)?,
            &separator()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &separator()?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &item("open", "Open Project…", Some("CmdOrCtrl+O"))?,
            &item("open-new", "Open Project in New Window…", Some("Shift+CmdOrCtrl+O"))?,
            &open_recent,
            &separator()?,
            &item("close-tab", "Close Tab", Some("CmdOrCtrl+W"))?,
            &item("close-window", "Close Window", Some("Shift+CmdOrCtrl+W"))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &separator()?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &separator()?,
            &item("find", "Find…", Some("CmdOrCtrl+F"))?,
            &item("find-next", "Find Next", Some("CmdOrCtrl+G"))?,
            &item("find-previous", "Find Previous", Some("Shift+CmdOrCtrl+G"))?,
            &separator()?,
            &item("quote", "Quote Selection", Some("CmdOrCtrl+'"))?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &item("files", "Show or Hide Files", Some("Alt+CmdOrCtrl+S"))?,
            &item("review", "Show or Hide Review", Some("CmdOrCtrl+J"))?,
            &item("review-side", "Review at the Bottom or Right", Some("Shift+CmdOrCtrl+J"))?,
            &separator()?,
            &item("zoom-in", "Zoom In", Some("CmdOrCtrl+="))?,
            &item("zoom-out", "Zoom Out", Some("CmdOrCtrl+-"))?,
            &item("zoom-fit", "Fit Width", Some("CmdOrCtrl+0"))?,
            &separator()?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
        ],
    )?;
    // the projects of the bar, ⌘1 to ⌘9 for the first nine
    let state = app.state::<AppState>();
    let bar = state.saved.lock().unwrap().bar.clone();
    if state.settings.lock().unwrap().project_bar && !bar.is_empty() {
        window.append(&separator()?)?;
        for (i, project) in crate::bar_items(&bar).iter().enumerate() {
            let keys = (i < 9).then(|| format!("CmdOrCtrl+{}", i + 1));
            window.append(&item(&format!("bar:{i}"), &project.name, keys.as_deref())?)?;
        }
    }
    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window])
}

/// The menu made again: Open Recent or the bar changed.
pub fn update(app: &AppHandle) {
    match build(app) {
        Ok(menu) => {
            let _ = app.set_menu(menu);
        }
        Err(e) => eprintln!("menu: {e}"),
    }
}

pub fn on_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "open" => choose_project(app, false),
        "open-new" => choose_project(app, true),
        "settings" => windows::show_settings(app),
        "clear-recent" => {
            let state = app.state::<AppState>();
            state.saved.lock().unwrap().recent.clear();
            state.save();
            update(app);
        }
        "review" | "review-side" => {
            if let Some(window) = front(app).filter(|w| w.label() != windows::START) {
                if id == "review" {
                    pane::toggle(app, window.label());
                } else {
                    pane::switch_side(app, window.label());
                }
            }
        }
        "close-window" => {
            if let Some(window) = front(app) {
                let _ = window.close();
            }
        }
        _ if id.starts_with("recent:") => {
            let root: Option<PathBuf> = id["recent:".len()..].parse::<usize>().ok().and_then(|i| {
                app.state::<AppState>().saved.lock().unwrap().recent.get(i).cloned()
            });
            if let Some(root) = root
                && let Err(e) = windows::open_project(app, &root)
            {
                app.dialog().message(e).title("Open Recent").show(|_| {});
            }
        }
        _ if id.starts_with("bar:") => {
            let root: Option<PathBuf> = id["bar:".len()..]
                .parse::<usize>()
                .ok()
                .and_then(|i| app.state::<AppState>().saved.lock().unwrap().bar.get(i).cloned());
            let window = windows::project_window(app);
            if let (Some(root), Some(window)) = (root, window)
                && let Err(e) = windows::switch(app, &window, &root)
            {
                app.dialog().message(e).title("Switch Project").show(|_| {});
            }
        }
        _ if PAGE.contains(&id) => match front(app) {
            Some(window) if window.label() == windows::START || window.label() == windows::SETTINGS => {
                if id == "close-tab" {
                    let _ = window.close();
                }
            }
            Some(window) => {
                let _ = app.emit_to(EventTarget::webview(windows::page(window.label())), "menu", id);
            }
            None => {}
        },
        _ => {}
    }
}

/// The window in front.
fn front(app: &AppHandle) -> Option<tauri::Window> {
    app.windows()
        .into_values()
        .find(|w| w.is_focused().unwrap_or(false))
}

/// Open Project…: a folder to choose; in a window of its own for Open Project in New Window….
pub fn choose_project(app: &AppHandle, new_window: bool) {
    let handle = app.clone();
    app.dialog()
        .file()
        .set_title("Open a project folder")
        .pick_folder(move |folder| {
            let Some(path) = folder.and_then(|f| f.into_path().ok()) else {
                return;
            };
            let opened = if new_window {
                windows::open_in_new_window(&handle, &path)
            } else {
                windows::open_project(&handle, &path)
            };
            if let Err(e) = opened {
                handle.dialog().message(e).title("Open Project").show(|_| {});
            }
        });
}
