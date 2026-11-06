//! The windows: the start window, the Settings window, and the windows of the projects, papers-tabs's (shared with
//! Papers Write): a window holds a page per tab (`tab-N`, the route `project`), each a file of a project (a PDF, a
//! text edited) or none (the project with nothing open: its files), grouped in a set per project, the project shown
//! first; and the review pane over them (pane.rs), of the project shown. With the project bar on (a setting) the
//! projects share a window: a tile switches it to that project, its tabs as left; off, each project has a window
//! of its own. The windows open as the app quit open again, each project with its tabs.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use papers_tabs::{Host, Layout, SavedWindow, Sidebars, Target, WindowStyle, Windows};
use review_core::{Colour, Project};
use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewUrl, WebviewWindowBuilder, Window};

use crate::{AppState, files, menu, pane, refs, review};

/// The start window: the projects opened last, and Open.
pub const START: &str = "start";

/// The Settings window.
pub const SETTINGS: &str = "settings";

/// What a page shows: its project, and its file (none: the project's files alone).
#[derive(Debug, Clone)]
pub struct Page {
    pub root: PathBuf,
    pub path: Option<PathBuf>,
}

pub fn show_start(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(START) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, START, WebviewUrl::App("/".into()))
        .title("Papers Review")
        .inner_size(560.0, 540.0)
        .resizable(false)
        .center()
        .build();
}

/// The Settings window, brought forward or made; at a section of it (its id: "updates") if named.
pub fn show_settings(app: &AppHandle, section: Option<&str>) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        if let Some(section) = section {
            let _ = app.emit_to(SETTINGS, "section", section);
        }
        return;
    }
    let url = match section {
        Some(section) => format!("settings#{section}"),
        None => "settings".to_string(),
    };
    let _ = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(url.into()))
        .title("Settings")
        .inner_size(560.0, 720.0)
        .min_inner_size(480.0, 360.0)
        .center()
        .build();
}

/// The windows open as the app last quit, each project with its tabs; else the start window.
pub fn reopen(app: &AppHandle) {
    if !papers_tabs::restore(app) {
        show_start(app);
    }
}

/// The project of `root` loaded, if it is not: opened, given a colour if it
/// has none (the next one not in use, written in its `review.toml`, or kept
/// until the project is defined when it has none), its review watched.
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

/// Whether the project of `root` is loaded and yet to be defined (it has no `review.toml`).
pub fn is_new(app: &AppHandle, root: &Path) -> bool {
    app.state::<AppState>().projects.lock().unwrap().get(root).is_some_and(Project::is_new)
}

/// The project of `root` unloaded, when no page shows it.
pub fn release(app: &AppHandle, root: &Path) {
    let state = app.state::<AppState>();
    if state.pages.lock().unwrap().values().any(|p| p.root == root) {
        return;
    }
    state.projects.lock().unwrap().remove(root);
    review::unwatch(app, root);
    refs::unload(app, root);
}

/// Whether the projects share a window, with a bar to switch between them.
fn bar_on(app: &AppHandle) -> bool {
    app.state::<AppState>().settings.lock().unwrap().project_bar
}

/// Open the project of a folder: in the window in front when the project bar is on (and added to the bar), else in
/// a window of its own. Or bring forward the window that shows it.
pub fn open_project(app: &AppHandle, folder: &Path) -> Result<(), String> {
    show(app, &canonical(folder)?, Target::Front)
}

/// The project of a folder in a window of its own, unless a window shows it already: that one comes forward.
pub fn open_in_new_window(app: &AppHandle, folder: &Path) -> Result<(), String> {
    show(app, &canonical(folder)?, Target::New)
}

/// The window `window` switched to the project of `root`, its tabs as left; or the window that shows it brought
/// forward.
pub fn switch(app: &AppHandle, window: &str, root: &Path) -> Result<(), String> {
    show(app, root, Target::Window(window.to_string()))
}

/// The project of `root` loaded and shown (`into`): a recent project once defined, in the bar when it is on.
fn show(app: &AppHandle, root: &Path, into: Target) -> Result<(), String> {
    load(app, root)?;
    if let Err(e) = papers_tabs::show_project(app, root, into) {
        release(app, root);
        return Err(e);
    }
    let state = app.state::<AppState>();
    {
        let mut saved = state.saved.lock().unwrap();
        // a new project is a recent one once it is defined
        if !is_new(app, root) {
            saved.remembered(root);
        }
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

fn canonical(folder: &Path) -> Result<PathBuf, String> {
    folder
        .canonicalize()
        .map_err(|e| format!("{}: {e}", folder.display()))
}

/// The tabs of the project of `root` closed, all of them: its window shows the project before it, or closes when
/// none is left.
fn close_project(app: &AppHandle, root: &Path) {
    for page in papers_tabs::pages_of_project(app, root) {
        papers_tabs::remove(app, &page);
    }
}

/// The project of `root` taken out of the bar. With the bar on, its tabs close: its window shows the project
/// before it, or closes when none is left.
pub fn remove_from_bar(app: &AppHandle, root: &Path) {
    let state = app.state::<AppState>();
    if !state.saved.lock().unwrap().bar.iter().any(|b| b == root) {
        return;
    }
    state.saved.lock().unwrap().bar.retain(|b| b != root);
    state.save();
    if bar_on(app) {
        close_project(app, root);
    }
    menu::update(app);
    tell_bar(app);
}

/// A new project given up, from the card that asks for its name: out of the bar, its tabs closed. Nothing was
/// written, and it was never a recent project.
pub fn cancel(app: &AppHandle, root: &Path) {
    let state = app.state::<AppState>();
    state.saved.lock().unwrap().bar.retain(|b| b != root);
    state.save();
    close_project(app, root);
    // a folder made from a manuscript for it: not kept either
    let started = state.started.lock().unwrap().remove(root);
    if let Some(pdf) = started {
        crate::manuscript::undo(app, root, &pdf);
        state.save();
    }
    menu::update(app);
    tell_bar(app);
}

/// The new project of `root` defined: first of the recent ones, its review watched (it may be another file
/// than before), and its pages told.
pub fn created(app: &AppHandle, root: &Path) {
    let state = app.state::<AppState>();
    state.saved.lock().unwrap().remembered(root);
    state.started.lock().unwrap().remove(root);
    state.save();
    review::moved(app, root);
    relabelled(app, root);
    changed(app, root);
}

/// The settings changed: with the bar on, the projects open in windows are in it; the menu follows.
pub fn settings_changed(app: &AppHandle) {
    let state = app.state::<AppState>();
    if bar_on(app) {
        let mut saved = state.saved.lock().unwrap();
        for root in papers_tabs::projects(app) {
            saved.barred(&root);
        }
        drop(saved);
        state.save();
    }
    menu::update(app);
    tell_bar(app);
}

/// What the windows and the project bars show of the project of `root` changed (its name, short name, due date or
/// colour): the title of its window, the menu and the project bars follow.
pub fn relabelled(app: &AppHandle, root: &Path) {
    if let Some(window) = papers_tabs::window_of_project(app, root) {
        papers_tabs::retitle(app, &window);
    }
    menu::update(app);
    tell_bar(app);
}

/// The pages told that the bar changed: what it lists, or which is current.
fn tell_bar(app: &AppHandle) {
    for window in papers_tabs::windows(app) {
        for page in papers_tabs::pages_of(app, &window) {
            let _ = app.emit_to(EventTarget::webview(&page), "bar-changed", ());
        }
    }
}

/// The project a window shows.
pub fn root_of(app: &AppHandle, window: &str) -> Option<PathBuf> {
    papers_tabs::project_of_window(app, window)
}

/// What a page shows.
pub fn page(app: &AppHandle, page: &str) -> Option<Page> {
    app.state::<AppState>().pages.lock().unwrap().get(page).cloned()
}

/// The window that holds the project of `root`, as a list (none or one).
pub fn of_project(app: &AppHandle, root: &Path) -> Vec<String> {
    papers_tabs::window_of_project(app, root).into_iter().collect()
}

/// The project of `root` changed (its colour, its files): its pages are told.
pub fn changed(app: &AppHandle, root: &Path) {
    for page in papers_tabs::pages_of_project(app, root) {
        let _ = app.emit_to(EventTarget::webview(&page), "project-changed", ());
    }
    refs::refresh(app, root);
}

/// A file of the project of `root` opened (from its files, the references): its tab brought to the front if one
/// shows it; else the page `from` takes it when it shows no file; else a tab of its own.
pub fn open_file(app: &AppHandle, root: &Path, path: &Path, from: Option<&str>) -> Result<(), String> {
    if let Some(page) = papers_tabs::page_of(app, path) {
        papers_tabs::select(app, &page);
        return Ok(());
    }
    let blank = from.filter(|f| page(app, f).is_some_and(|p| p.root == root && p.path.is_none()));
    match blank {
        Some(from) => {
            shows(app, from, Some(path));
            papers_tabs::select(app, from);
        }
        None => {
            make_page(app, root, Some(path))?;
        }
    }
    Ok(())
}

/// The page shows another file from now on (none: its project's files alone): its tab, its watch, and the page
/// told (`opened`) to show it.
pub fn shows(app: &AppHandle, label: &str, path: Option<&Path>) {
    if let Some(page) = app.state::<AppState>().pages.lock().unwrap().get_mut(label) {
        page.path = path.map(Path::to_path_buf);
    }
    files::unwatch_all(app, label);
    papers_tabs::renamed(app, label);
    let _ = app.emit_to(EventTarget::webview(label), "opened", ());
}

/// The pages that show a file or folder gone from the project (`path`, or what is under it) closed.
pub fn close_under(app: &AppHandle, root: &Path, path: &Path) {
    for label in papers_tabs::pages_of_project(app, root) {
        if page(app, &label).and_then(|p| p.path).is_some_and(|p| p.starts_with(path)) {
            finish_closing(app, &label);
        }
    }
}

/// A page made for `file` of the project of `root` (none: its files alone), as a tab of the project. Its label.
fn make_page(app: &AppHandle, root: &Path, file: Option<&Path>) -> Result<String, String> {
    load(app, root)?;
    let state = app.state::<AppState>();
    let label = format!("tab-{}", state.next.fetch_add(1, Ordering::Relaxed));
    state
        .pages
        .lock()
        .unwrap()
        .insert(label.clone(), Page { root: root.to_path_buf(), path: file.map(Path::to_path_buf) });
    if let Err(e) = papers_tabs::add(app, &label, "project", Some(root)) {
        state.pages.lock().unwrap().remove(&label);
        release(app, root);
        return Err(e.to_string());
    }
    Ok(label)
}

/// A tab to close, its editor's text written (the page asked, `closing`): the last of its project shows the
/// project's files instead, unless it shows them already; any other goes.
pub fn finish_closing(app: &AppHandle, label: &str) {
    let last = page(app, label).is_some_and(|p| p.path.is_some() && papers_tabs::pages_of_project(app, &p.root).len() == 1);
    if last {
        shows(app, label, None);
    } else {
        papers_tabs::remove(app, label);
    }
}

/// The kinds of file a page edits, whose text may wait to be written when it closes.
fn edited(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ["md", "markdown", "tex", "txt", "bib"].iter().any(|k| e.eq_ignore_ascii_case(k)))
}

/// What Papers Review says of its pages to papers-tabs.
pub struct Review;

impl Host for Review {
    fn style(&self) -> WindowStyle {
        WindowStyle { title: "Papers Review", size: (1200.0, 800.0), least: (640.0, 400.0) }
    }

    fn path(&self, app: &AppHandle, label: &str) -> Option<PathBuf> {
        page(app, label)?.path
    }

    fn title(&self, app: &AppHandle, label: &str) -> String {
        match page(app, label).and_then(|p| p.path) {
            Some(path) => path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            None => "Files".to_string(),
        }
    }

    /// The project's name.
    fn window_title(&self, app: &AppHandle, label: &str, tab: &str) -> String {
        let root = page(app, label).map(|p| p.root);
        let state = app.state::<AppState>();
        let projects = state.projects.lock().unwrap();
        root.and_then(|r| projects.get(&r).map(Project::name)).unwrap_or_else(|| tab.to_string())
    }

    /// A page's project is the one it was made for, never one found from its file.
    fn project_of(&self, _app: &AppHandle, _file: &Path) -> Option<PathBuf> {
        None
    }

    fn windows(&self, app: &AppHandle) -> Windows {
        if bar_on(app) { Windows::Shared } else { Windows::PerProject }
    }

    fn open(&self, app: &AppHandle, root: &Path, file: Option<&Path>) -> Result<String, String> {
        make_page(app, root, file)
    }

    /// A project with no tabs: its files alone.
    fn first_file(&self, _app: &AppHandle, _root: &Path) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }

    /// A page that edits a file writes what waits first (`closing`; it then calls `close_page`); the last page of a
    /// project shows its files instead of going.
    fn asks_before_closing(&self, app: &AppHandle, label: &str) -> bool {
        let Some(page) = page(app, label) else {
            return false;
        };
        if page.path.as_deref().is_some_and(edited) {
            let _ = app.emit_to(EventTarget::webview(label), "closing", ());
            return true;
        }
        let last = page.path.is_some() && papers_tabs::pages_of_project(app, &page.root).len() == 1;
        if last {
            shows(app, label, None);
        }
        last
    }

    fn added(&self, app: &AppHandle, window: &Window, _page: &str) {
        pane::over_pages(app, window);
    }

    fn closed(&self, app: &AppHandle, label: &str) {
        let page = app.state::<AppState>().pages.lock().unwrap().remove(label);
        files::unwatch_all(app, label);
        pane::page_closed(app, label);
        if let Some(page) = page {
            release(app, &page.root);
        }
    }

    /// The pane follows: what was typed written to the file it read, the new review read.
    fn shown(&self, app: &AppHandle, window: &str, _root: Option<&Path>) {
        pane::switched(app, window);
        let _ = app.emit_to(EventTarget::webview(pane::label(window)), "review-switched", ());
        menu::update(app);
        tell_bar(app);
    }

    fn window_closed(&self, app: &AppHandle, window: &str) {
        pane::closed(app, window);
    }

    fn layout(&self, app: &AppHandle, root: &Path) -> Layout {
        app.state::<AppState>().saved.lock().unwrap().tabs.get(root).cloned().unwrap_or_default()
    }

    fn save_layouts(&self, app: &AppHandle, layouts: Vec<(PathBuf, Layout)>) {
        let state = app.state::<AppState>();
        let changed = {
            let mut saved = state.saved.lock().unwrap();
            let mut changed = false;
            for (root, layout) in layouts {
                changed |= saved.tabs.get(&root) != Some(&layout);
                saved.tabs.insert(root, layout);
            }
            changed
        };
        if changed {
            state.save();
        }
    }

    fn saved_windows(&self, app: &AppHandle) -> Vec<SavedWindow> {
        app.state::<AppState>().saved.lock().unwrap().windows.clone()
    }

    fn save_windows(&self, app: &AppHandle, windows: Vec<SavedWindow>) {
        let state = app.state::<AppState>();
        let changed = {
            let mut saved = state.saved.lock().unwrap();
            let changed = saved.windows != windows;
            saved.windows = windows;
            changed
        };
        if changed {
            state.save();
        }
    }

    fn with_sidebars(&self, app: &AppHandle, f: &mut dyn FnMut(&mut Sidebars)) {
        let state = app.state::<AppState>();
        let changed = {
            let mut saved = state.saved.lock().unwrap();
            let before = saved.sidebars.clone();
            f(&mut saved.sidebars);
            saved.sidebars != before
        };
        if changed {
            state.save();
        }
    }
}
