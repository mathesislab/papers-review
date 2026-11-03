//! The windows: the start window, the Settings window, and the windows of the
//! projects, which hold web views (the page, the review pane over it). A
//! window's label names its project in `AppState::windows`. With the project
//! bar on (a setting) the projects share a window: opening one switches the
//! window in front to it, and the bar lists the projects to switch between;
//! off, each project has a window of its own.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use review_core::{Colour, Project};
use tauri::window::WindowBuilder;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, WebviewBuilder,
    WebviewUrl, WebviewWindowBuilder, Window,
};

use crate::{AppState, files, menu, pane, review};

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
        .inner_size(520.0, 640.0)
        .min_inner_size(440.0, 300.0)
        .center()
        .build();
}

/// The windows open as the app last quit (each shows the project it was
/// switched to), else the start window.
pub fn reopen(app: &AppHandle) {
    let open = app.state::<AppState>().saved.lock().unwrap().open.clone();
    let mut any = false;
    for root in open {
        match open_in_new_window(app, &root) {
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

/// Whether the projects share a window, with a bar to switch between them.
fn bar_on(app: &AppHandle) -> bool {
    app.state::<AppState>().settings.lock().unwrap().project_bar
}

/// The window of a project to switch: the one in front, else the first opened.
pub fn project_window(app: &AppHandle) -> Option<String> {
    let state = app.state::<AppState>();
    let labels: Vec<String> = state.windows.lock().unwrap().keys().cloned().collect();
    let front = app
        .windows()
        .into_values()
        .find(|w| w.is_focused().unwrap_or(false) && labels.iter().any(|l| l == w.label()));
    match front {
        Some(window) => Some(window.label().to_string()),
        None => labels.into_iter().min_by_key(|l| (l.len(), l.clone())),
    }
}

/// Open the project of a folder: in the window in front when the project bar
/// is on (and added to the bar), else in a window of its own. Or bring forward
/// the window that shows it.
pub fn open_project(app: &AppHandle, folder: &Path) -> Result<(), String> {
    let root = canonical(folder)?;
    if let Some(window) = window_of(app, &root) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }
    if bar_on(app)
        && let Some(window) = project_window(app)
    {
        load(app, &root)?;
        app.state::<AppState>().saved.lock().unwrap().barred(&root);
        return switch(app, &window, &root);
    }
    new_window(app, &root)
}

/// The project of a folder in a window of its own, though another window shows it.
pub fn open_in_new_window(app: &AppHandle, folder: &Path) -> Result<(), String> {
    new_window(app, &canonical(folder)?)
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
    {
        let mut saved = state.saved.lock().unwrap();
        saved.opened(root);
        if bar_on(app) {
            saved.barred(root);
        }
    }
    state.save();
    menu::update(app);
    tell_bar(app);
    if let Some(start) = app.get_webview_window(START) {
        let _ = start.close();
    }
    Ok(())
}

/// The window `window` switched to the project of `root`: loaded, the old
/// project unloaded when no other window shows it; the title, the pane and
/// the page follow. Where another window shows `root`, that one comes forward.
pub fn switch(app: &AppHandle, window: &str, root: &Path) -> Result<(), String> {
    let old = root_of(app, window).ok_or("no project in this window")?;
    if old == root {
        return Ok(());
    }
    if let Some(other) = window_of(app, root) {
        let _ = other.unminimize();
        let _ = other.set_focus();
        return Ok(());
    }
    load(app, root)?;
    let state = app.state::<AppState>();
    state.windows.lock().unwrap().insert(window.to_string(), root.to_path_buf());
    release(app, &old);
    state.saved.lock().unwrap().switched(&old, root);
    state.save();
    retitle(app, window);
    files::unwatch_all(app, window);
    pane::switched(app, window, &old);
    // the pane writes what was typed to the file it read, then reads the new review; the page loads again
    let _ = app.emit_to(EventTarget::webview(pane::label(window)), "review-switched", ());
    let _ = app.emit_to(EventTarget::webview(page(window)), "project-switched", ());
    menu::update(app);
    tell_bar(app);
    Ok(())
}

/// The project of `root` taken out of the bar. Its window, with the bar on,
/// switches to a neighbour in the bar, or closes when none is left.
pub fn remove_from_bar(app: &AppHandle, root: &Path) {
    let state = app.state::<AppState>();
    let bar = state.saved.lock().unwrap().bar.clone();
    if !bar.iter().any(|b| b == root) {
        return;
    }
    state.saved.lock().unwrap().bar.retain(|b| b != root);
    state.save();
    if bar_on(app) {
        for window in of_project(app, root) {
            match neighbour(&bar, root, |p| window_of(app, p).is_none()) {
                Some(next) => {
                    if let Err(e) = switch(app, &window, &next) {
                        eprintln!("{e}");
                    }
                }
                None => {
                    if let Some(window) = app.get_window(&window) {
                        let _ = window.close();
                    }
                }
            }
        }
    }
    menu::update(app);
    tell_bar(app);
}

/// The project of `bar` to switch to when `root` leaves it: the next one that
/// `free` allows, else the nearest before it.
fn neighbour(bar: &[PathBuf], root: &Path, free: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let at = bar.iter().position(|b| b == root)?;
    bar[at + 1..]
        .iter()
        .chain(bar[..at].iter().rev())
        .find(|b| free(b))
        .cloned()
}

/// The settings changed: with the bar on, the projects open in windows are
/// in it; the menu follows.
pub fn settings_changed(app: &AppHandle) {
    let state = app.state::<AppState>();
    if bar_on(app) {
        let mut labels: Vec<(String, PathBuf)> = state.windows.lock().unwrap().clone().into_iter().collect();
        labels.sort_by_key(|(label, _)| (label.len(), label.clone()));
        let mut saved = state.saved.lock().unwrap();
        for (_, root) in labels {
            saved.barred(&root);
        }
        drop(saved);
        state.save();
    }
    menu::update(app);
    tell_bar(app);
}

/// The window titled with the name of its project.
fn retitle(app: &AppHandle, window: &str) {
    let name = root_of(app, window).and_then(|root| {
        let state = app.state::<AppState>();
        let projects = state.projects.lock().unwrap();
        projects.get(&root).map(|p| p.name())
    });
    if let (Some(window), Some(name)) = (app.get_window(window), name) {
        let _ = window.set_title(&name);
    }
}

/// The name, the short name or the due date of the project of `root` changed: the titles of its windows,
/// the menu and the project bars follow.
pub fn renamed(app: &AppHandle, root: &Path) {
    for window in of_project(app, root) {
        retitle(app, &window);
    }
    menu::update(app);
    tell_bar(app);
}

/// The project pages told that the bar changed: what it lists, or which is current.
fn tell_bar(app: &AppHandle) {
    let labels: Vec<String> = app.state::<AppState>().windows.lock().unwrap().keys().cloned().collect();
    for label in labels {
        let _ = app.emit_to(EventTarget::webview(page(&label)), "bar-changed", ());
    }
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
    files::unwatch_all(app, window);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bar() -> Vec<PathBuf> {
        ["/a", "/b", "/c", "/d"].map(PathBuf::from).to_vec()
    }

    #[test]
    fn a_project_leaving_the_bar_gives_its_window_to_the_next_one_else_the_one_before() {
        let (b, d) = (Path::new("/b"), Path::new("/d"));
        assert_eq!(neighbour(&bar(), b, |_| true), Some(PathBuf::from("/c")));
        assert_eq!(neighbour(&bar(), d, |_| true), Some(PathBuf::from("/c")));
    }

    #[test]
    fn a_project_shown_in_another_window_is_passed_over() {
        let shown = [Path::new("/c"), Path::new("/d")];
        let neighbour = neighbour(&bar(), Path::new("/b"), |p| !shown.contains(&p));
        assert_eq!(neighbour, Some(PathBuf::from("/a")));
    }

    #[test]
    fn nothing_is_left_when_the_bar_has_no_other_project_or_none_free() {
        let alone = [PathBuf::from("/a")];
        assert_eq!(neighbour(&alone, Path::new("/a"), |_| true), None);
        assert_eq!(neighbour(&bar(), Path::new("/b"), |_| false), None);
        assert_eq!(neighbour(&bar(), Path::new("/x"), |_| true), None);
    }
}
