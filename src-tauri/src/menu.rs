//! The menu bar. The commands of the tabs are papers-tabs's; those of a project's page go to the page in front of
//! the window in front (event `menu`, the id of the item); the export commands go to its review pane (event
//! `export`); the others are the app's.

use std::path::PathBuf;

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, EventTarget, Manager, Wry};
use tauri_plugin_dialog::DialogExt;

use crate::{AppState, manuscript, pane, windows};

/// The commands of the page, by the id of their item.
const PAGE: &[&str] = &[
    "files",
    "zoom-in",
    "zoom-out",
    "zoom-fit",
    "find",
    "find-next",
    "find-previous",
    "quote",
    "insert",
    "references",
    "info",
    "previous-page",
    "next-page",
    "first-page",
    "last-page",
    "go-to-page",
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
            &item("check-updates", "Check for Updates…", None)?,
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
            &item("new-review", "New Review from Manuscript…", Some("CmdOrCtrl+N"))?,
            &item("open", "Open Project…", Some("CmdOrCtrl+O"))?,
            &item("open-new", "Open Project in New Window…", Some("Shift+CmdOrCtrl+O"))?,
            &open_recent,
            &separator()?,
            &item("info", "Project Info…", Some("CmdOrCtrl+I"))?,
            &separator()?,
            &item("close-tab", "Close Tab", Some("CmdOrCtrl+W"))?,
            &item("close-window", "Close Window", Some("Shift+CmdOrCtrl+W"))?,
            &separator()?,
            &item("export-pdf", "Export Review as PDF…", Some("CmdOrCtrl+E"))?,
            &item("export-text", "Copy Review as Text", Some("Shift+CmdOrCtrl+E"))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &item("undo", "Undo", Some("CmdOrCtrl+Z"))?,
            &item("redo", "Redo", Some("Shift+CmdOrCtrl+Z"))?,
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
            &item("insert", "Insert Selection", Some("CmdOrCtrl+U"))?,
            &item("quote", "Quote Selection", Some("CmdOrCtrl+Y"))?,
            &separator()?,
            &item("symbols", "Symbols…", Some("CmdOrCtrl+/"))?,
            &item("references", "References…", Some("Shift+CmdOrCtrl+R"))?,
            &item("cite-key", "Cite as [@key]", Some("CmdOrCtrl+."))?,
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
    let go = Submenu::with_items(
        app,
        "Go",
        true,
        &[
            &item("back", "Back", Some("CmdOrCtrl+["))?,
            &item("forward", "Forward", Some("CmdOrCtrl+]"))?,
            &separator()?,
            &item("previous-page", "Previous Page", Some("Alt+CmdOrCtrl+Up"))?,
            &item("next-page", "Next Page", Some("Alt+CmdOrCtrl+Down"))?,
            &item("first-page", "First Page", None)?,
            &item("last-page", "Last Page", None)?,
            &separator()?,
            &item("go-to-page", "Go to Page…", Some("Alt+CmdOrCtrl+G"))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &separator()?,
            &item("previous-tab", "Show Previous Tab", Some("Shift+CmdOrCtrl+["))?,
            &item("next-tab", "Show Next Tab", Some("Shift+CmdOrCtrl+]"))?,
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
    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &go, &window])
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
        "new-review" => choose_manuscript(app),
        "open" => choose_project(app, false),
        "open-new" => choose_project(app, true),
        "settings" => windows::show_settings(app, None),
        "check-updates" => windows::show_settings(app, Some("updates")),
        "clear-recent" => {
            let state = app.state::<AppState>();
            state.saved.lock().unwrap().recent.clear();
            state.save();
            update(app);
        }
        "undo" | "redo" | "symbols" | "back" | "forward" => edit(app, id),
        "review" | "review-side" => {
            if let Some(window) = front(app).filter(|w| w.label() != windows::START) {
                if id == "review" {
                    pane::toggle(app, window.label());
                } else {
                    pane::switch_side(app, window.label());
                }
            }
        }
        // the pane knows which section of the review is shown, and has its editor loaded even when hidden
        "export-pdf" | "export-text" => {
            if let Some(window) = front(app).filter(|w| windows::root_of(app, w.label()).is_some()) {
                let kind = if id == "export-pdf" { "pdf" } else { "text" };
                let _ = app.emit_to(EventTarget::webview(pane::label(window.label())), "export", kind);
            }
        }
        // ⌘. in the references list: macOS takes it for Cancel before the page sees the key, so it comes as a menu
        // item, to the pane, where the list is
        "cite-key" => {
            if let Some(window) = front(app).filter(|w| windows::root_of(app, w.label()).is_some()) {
                let _ = app.emit_to(EventTarget::webview(pane::label(window.label())), "cite-key", ());
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
            let switched = match (root, papers_tabs::front_window(app)) {
                (Some(root), Some(window)) => windows::switch(app, &window, &root),
                (Some(root), None) => windows::open_project(app, &root),
                _ => Ok(()),
            };
            if let Err(e) = switched {
                app.dialog().message(e).title("Switch Project").show(|_| {});
            }
        }
        // the tabs are papers-tabs's: ⌘W closes the tab in front (a window without tabs, the start or Settings)
        "close-tab" => papers_tabs::close_front(app),
        "previous-tab" => papers_tabs::step(app, -1),
        "next-tab" => papers_tabs::step(app, 1),
        // the rest to the page in front, when a project's window is
        _ if PAGE.contains(&id) => {
            let ours = papers_tabs::key_window(app).is_some_and(|w| windows::root_of(app, &w).is_some());
            if ours && let Some(page) = papers_tabs::front_page(app) {
                let _ = app.emit_to(EventTarget::webview(page.label()), "menu", id);
            }
        }
        _ => {}
    }
}

/// Undo, Redo, Symbols…, Back and Forward: the review's own history, list of
/// symbols or indent (⌘[ and ⌘] are indent less and more in the editor) when
/// its pane has the keys (the editor keeps them, not WebKit; the pane and the
/// page say which has them as they are clicked or focused), else the page of
/// a project's window, which gives it to the file edited in front or, for
/// Undo and Redo, a field of the page with the keys; Back and Forward go to
/// the PDF when it is in front.
fn edit(app: &AppHandle, name: &str) {
    let Some(window) = front(app) else {
        return;
    };
    let label = window.label();
    if let Some(page) = papers_tabs::front_of(app, label) {
        let pane_has_keys = app.state::<AppState>().panes.lock().unwrap().focused.contains(label);
        let target = if pane_has_keys { pane::label(label) } else { page };
        let _ = app.emit_to(EventTarget::webview(target), "edit", name);
    } else if matches!(name, "undo" | "redo")
        && let Some(webview) = app.get_webview(label)
    {
        // a window of one web view (start, settings)
        let _ = webview.eval(format!("document.execCommand('{name}')"));
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

/// New Review from Manuscript…: a PDF to choose.
pub fn choose_manuscript(app: &AppHandle) {
    let handle = app.clone();
    app.dialog()
        .file()
        .set_title("Choose the manuscript to review")
        .add_filter("PDF", &["pdf"])
        .pick_file(move |file| {
            if let Some(path) = file.and_then(|f| f.into_path().ok()) {
                manuscript::begin(&handle, path);
            }
        });
}
