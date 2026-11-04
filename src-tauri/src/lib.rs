//! papers-review: a window per review project (a folder), or one window with a
//! bar to switch between projects (a setting), a start window and a Settings
//! window. The project of a window is found by its label (`windows`).

pub mod export;
mod files;
mod manuscript;
mod menu;
mod pane;
mod refs;
mod review;
mod settings;
mod state;
mod trash;
mod update;
mod windows;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;

use papers_core::Found;
use review_core::{Colour, Datetime, Node, Project, ReviewFile};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, Webview, WindowEvent};

use settings::Settings;
use state::{Layout, Saved};
use update::{Available, Installed, Updates};

pub struct AppState {
    /// the projects open, by their folder
    pub projects: Mutex<HashMap<PathBuf, Project>>,
    /// the project of each window, by the window's label
    pub windows: Mutex<HashMap<String, PathBuf>>,
    pub saved: Mutex<Saved>,
    pub settings: Mutex<Settings>,
    /// the app's data folder, where `saved` and `settings` are written
    data: PathBuf,
    /// the number of the next window
    next: AtomicUsize,
    /// the review panes, as they are now
    pub panes: Mutex<pane::Panes>,
    /// the review of each project open, watched
    pub watchers: Mutex<HashMap<PathBuf, notify::RecommendedWatcher>>,
    /// the files open in the tabs of each window, watched
    pub file_watchers: Mutex<files::Watched>,
    /// the PDF exports waiting for their hidden window, by its label
    pub exports: Mutex<HashMap<String, export::Pending>>,
    /// the references of each project open, once asked for
    pub references: Mutex<refs::Loaded>,
    /// the folders made for a new review from a manuscript, not yet defined, with the PDF copied into each
    pub started: Mutex<HashMap<PathBuf, PathBuf>>,
}

impl AppState {
    pub fn save(&self) {
        self.saved.lock().unwrap().save(&self.data);
    }
}

/// A project as its page shows it.
#[derive(Serialize)]
struct ProjectView {
    root: PathBuf,
    name: String,
    title: Option<String>,
    short: Option<String>,
    journal: Option<String>,
    manuscript: Option<String>,
    /// the heading of the PDF export
    heading: Option<String>,
    due: Option<String>,
    colour: Colour,
    files: Vec<Node>,
    linked: Vec<Node>,
    /// the Markdown file of the review
    review: PathBuf,
    /// not defined yet: the folder had no `review.toml` when it was opened, and the page asks for its name,
    /// due date and review file (`create_project`)
    new: bool,
}

/// A project as the page of its settings shows it: what `review.toml` says, the Markdown files to choose
/// the review from and the files linked, as paths.
#[derive(Serialize)]
struct ProjectInfo {
    root: PathBuf,
    name: String,
    title: Option<String>,
    short: Option<String>,
    journal: Option<String>,
    manuscript: Option<String>,
    heading: Option<String>,
    /// `YYYY-MM-DD`
    due: Option<String>,
    colour: Colour,
    review: PathBuf,
    markdown: Vec<PathBuf>,
    linked: Vec<PathBuf>,
}

/// What the page of a project's settings changes: all of it is written, a missing text or an empty one
/// removes the key.
#[derive(Deserialize)]
struct InfoEdit {
    title: Option<String>,
    short: Option<String>,
    journal: Option<String>,
    manuscript: Option<String>,
    heading: Option<String>,
    /// `YYYY-MM-DD`
    due: Option<String>,
}

/// A project opened before, as the start window lists it.
#[derive(Serialize)]
struct Recent {
    root: PathBuf,
    name: String,
    colour: Colour,
    /// gone, or no longer readable
    missing: bool,
}

/// A project of the project bar, as its tile shows it.
#[derive(Serialize)]
pub struct BarItem {
    pub root: PathBuf,
    pub name: String,
    pub short: Option<String>,
    pub colour: Colour,
    pub due: Option<String>,
    /// gone, or no longer readable
    pub missing: bool,
}

/// The projects of `roots` as their tiles and menu lines show them, read from their folders.
pub fn bar_items(roots: &[PathBuf]) -> Vec<BarItem> {
    roots
        .iter()
        .map(|root| match Project::open(root) {
            Ok(project) => BarItem {
                name: project.name(),
                short: project.manifest.short.clone(),
                colour: project.manifest.colour.unwrap_or(Colour::None),
                due: project.manifest.due.map(|d| d.to_string()),
                root: root.clone(),
                missing: false,
            },
            Err(_) => BarItem {
                name: root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                short: None,
                colour: Colour::None,
                due: None,
                root: root.clone(),
                missing: true,
            },
        })
        .collect()
}

/// The project of the window of a page.
fn root_of(webview: &Webview) -> Result<PathBuf, String> {
    windows::root_of(webview.app_handle(), webview.window().label())
        .ok_or_else(|| format!("{}: no project in this window", webview.label()))
}

/// The settings, for the Settings window and the project pages.
#[tauri::command]
fn settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// The settings changed: kept, and every page told.
#[tauri::command]
fn set_settings(app: AppHandle, settings: Settings) {
    apply_settings(&app, settings);
}

/// The settings kept as `settings`, and every page told.
pub fn apply_settings(app: &AppHandle, settings: Settings) {
    let state = app.state::<AppState>();
    *state.settings.lock().unwrap() = settings.clone();
    settings.save(&state.data);
    windows::settings_changed(app);
    let _ = app.emit("settings-changed", settings);
}

#[tauri::command]
fn open_project(app: AppHandle, path: PathBuf) -> Result<(), String> {
    windows::open_project(&app, &path)
}

#[tauri::command]
fn choose_project(app: AppHandle) {
    menu::choose_project(&app, false);
}

/// A review started from the manuscript `pdf`: its folder, the project open with the title, journal and
/// manuscript ID read from the PDF. Returns when the project is open.
#[tauri::command]
async fn new_review(app: AppHandle, pdf: PathBuf) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || manuscript::start(&app, &pdf))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn recent(state: State<'_, AppState>) -> Vec<Recent> {
    let recent = state.saved.lock().unwrap().recent.clone();
    bar_items(&recent)
        .into_iter()
        .map(|item| Recent {
            root: item.root,
            name: item.name,
            colour: item.colour,
            missing: item.missing,
        })
        .collect()
}

/// The projects of the project bar, in order.
#[tauri::command]
fn bar(state: State<'_, AppState>) -> Vec<BarItem> {
    let bar = state.saved.lock().unwrap().bar.clone();
    bar_items(&bar)
}

/// The window of the calling page switched to the project of `root`.
#[tauri::command]
fn switch_project(app: AppHandle, webview: Webview, root: PathBuf) -> Result<(), String> {
    windows::switch(&app, webview.window().label(), &root)
}

#[tauri::command]
fn remove_from_bar(app: AppHandle, root: PathBuf) {
    windows::remove_from_bar(&app, &root);
}

#[tauri::command]
fn open_in_new_window(app: AppHandle, root: PathBuf) -> Result<(), String> {
    windows::open_in_new_window(&app, &root)
}

#[tauri::command]
fn project(webview: Webview, state: State<'_, AppState>) -> Result<ProjectView, String> {
    let root = root_of(&webview)?;
    let projects = state.projects.lock().unwrap();
    let project = projects.get(&root).ok_or("the project is not open")?;
    let manifest = &project.manifest;
    Ok(ProjectView {
        name: project.name(),
        title: manifest.title.clone(),
        short: manifest.short.clone(),
        journal: manifest.journal.clone(),
        manuscript: manifest.manuscript.clone(),
        heading: manifest.heading.clone(),
        due: manifest.due.map(|d| d.to_string()),
        colour: manifest.colour.unwrap_or(Colour::None),
        files: project.files(),
        linked: project.linked(),
        review: project.review_path(),
        new: project.is_new(),
        root,
    })
}

/// The project as the page of its settings shows it.
#[tauri::command]
fn project_info(webview: Webview, state: State<'_, AppState>) -> Result<ProjectInfo, String> {
    let root = root_of(&webview)?;
    let projects = state.projects.lock().unwrap();
    let project = projects.get(&root).ok_or("the project is not open")?;
    let manifest = &project.manifest;
    Ok(ProjectInfo {
        name: project.name(),
        title: manifest.title.clone(),
        short: manifest.short.clone(),
        journal: manifest.journal.clone(),
        manuscript: manifest.manuscript.clone(),
        heading: manifest.heading.clone(),
        due: manifest.due.map(|d| d.to_string()),
        colour: manifest.colour.unwrap_or(Colour::None),
        review: project.review_path(),
        markdown: project.markdown_files(),
        linked: project.linked().into_iter().map(|n| n.path).collect(),
        root,
    })
}

/// The title, short name, journal, manuscript, heading and due date of the project, for all its windows, written
/// in its `review.toml`.
#[tauri::command]
fn set_project_info(app: AppHandle, webview: Webview, state: State<'_, AppState>, info: InfoEdit) -> Result<(), String> {
    let root = root_of(&webview)?;
    let due = parse_due(info.due.as_deref())?;
    {
        let mut projects = state.projects.lock().unwrap();
        let project = projects.get_mut(&root).ok_or("the project is not open")?;
        set_info(project, info, due)?;
    }
    windows::relabelled(&app, &root);
    windows::changed(&app, &root);
    Ok(())
}

/// A due date of the card: `YYYY-MM-DD`, or none when empty.
fn parse_due(due: Option<&str>) -> Result<Option<Datetime>, String> {
    match due.map(str::trim).filter(|d| !d.is_empty()) {
        Some(due) => Ok(Some(
            due.parse::<Datetime>()
                .ok()
                .filter(|d| d.date.is_some() && d.time.is_none())
                .ok_or_else(|| format!("{due}: not a date (YYYY-MM-DD)"))?,
        )),
        None => Ok(None),
    }
}

/// The fields of the card set in the project.
fn set_info(project: &mut Project, info: InfoEdit, due: Option<Datetime>) -> Result<(), String> {
    project
        .set_title(info.title)
        .and_then(|()| project.set_short(info.short))
        .and_then(|()| project.set_journal(info.journal))
        .and_then(|()| project.set_manuscript(info.manuscript))
        .and_then(|()| project.set_heading(info.heading))
        .and_then(|()| project.set_due(due))
        .map_err(|e| e.to_string())
}

/// The new project defined, from the card that asks for it: the fields as in `set_project_info`, then
/// `review.toml` written and the review file made (`review`, its name in the folder) or chosen (`existing`,
/// a Markdown file of the folder; `review` is then unused). Nothing is written if it fails.
#[tauri::command]
fn create_project(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    info: InfoEdit,
    review: String,
    existing: Option<PathBuf>,
) -> Result<(), String> {
    let root = root_of(&webview)?;
    let due = parse_due(info.due.as_deref())?;
    {
        let mut projects = state.projects.lock().unwrap();
        let project = projects.get_mut(&root).ok_or("the project is not open")?;
        set_info(project, info, due)?;
        let file = match &existing {
            Some(path) => ReviewFile::Existing(path),
            None => ReviewFile::New(&review),
        };
        project.create(file).map_err(|e| e.to_string())?;
    }
    windows::created(&app, &root);
    Ok(())
}

/// Cancel on the card of a new project: the project is closed, as a window or in the bar, and leaves nothing.
#[tauri::command]
fn cancel_project(app: AppHandle, webview: Webview) -> Result<(), String> {
    let root = root_of(&webview)?;
    if !windows::is_new(&app, &root) {
        return Err(format!("{}: already a project", root.display()));
    }
    windows::cancel(&app, &root);
    Ok(())
}

/// Files from elsewhere added to the project: linked, or copied into `folder` (the project's folder
/// without).
#[tauri::command]
fn add_files(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    paths: Vec<PathBuf>,
    copy: bool,
    folder: Option<PathBuf>,
) -> Result<(), String> {
    let root = root_of(&webview)?;
    {
        let mut projects = state.projects.lock().unwrap();
        let project = projects.get_mut(&root).ok_or("the project is not open")?;
        if copy {
            project.copy_in(&paths, &folder.unwrap_or_else(|| root.clone()))
        } else {
            project.link(&paths)
        }
        .map_err(|e| e.to_string())?;
    }
    windows::changed(&app, &root);
    Ok(())
}

/// A file linked from elsewhere no longer linked; the file itself stays where it is.
#[tauri::command]
fn unlink(app: AppHandle, webview: Webview, state: State<'_, AppState>, path: PathBuf) -> Result<(), String> {
    let root = root_of(&webview)?;
    state
        .projects
        .lock()
        .unwrap()
        .get_mut(&root)
        .ok_or("the project is not open")?
        .unlink(&path)
        .map_err(|e| e.to_string())?;
    windows::changed(&app, &root);
    Ok(())
}

/// An empty file `name` in `folder` of the project: its path.
#[tauri::command]
fn new_file(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    folder: PathBuf,
    name: String,
) -> Result<PathBuf, String> {
    let root = root_of(&webview)?;
    let path = state
        .projects
        .lock()
        .unwrap()
        .get(&root)
        .ok_or("the project is not open")?
        .new_file(&folder, &name)
        .map_err(|e| e.to_string())?;
    windows::changed(&app, &root);
    Ok(path)
}

/// The text of a file of the project.
#[tauri::command]
fn read_file(app: AppHandle, webview: Webview, path: PathBuf) -> Result<String, String> {
    files::read(&app, webview.window().label(), &path)
}

/// A file of the project written, atomically.
#[tauri::command]
fn write_file(app: AppHandle, webview: Webview, path: PathBuf, text: String) -> Result<(), String> {
    files::write(&app, webview.window().label(), &path, &text)
}

/// A file of the project open in a tab: a change made to it elsewhere comes to the page as `file-changed`.
#[tauri::command]
fn watch_file(app: AppHandle, webview: Webview, path: PathBuf) -> Result<(), String> {
    files::watch(&app, webview.window().label(), &path)
}

/// The tab of a file closed.
#[tauri::command]
fn unwatch_file(app: AppHandle, webview: Webview, path: PathBuf) {
    files::unwatch(&app, webview.window().label(), &path);
}

/// How the window of the project looked.
#[tauri::command]
fn layout(webview: Webview, state: State<'_, AppState>) -> Result<Layout, String> {
    let root = root_of(&webview)?;
    let saved = state.saved.lock().unwrap();
    Ok(saved.layouts.get(&root).cloned().unwrap_or_default())
}

/// How the window of `root` looks, written. The page names its project: its write comes a moment after the change,
/// when the window may show another project.
#[tauri::command]
fn set_layout(state: State<'_, AppState>, root: PathBuf, layout: Layout) {
    let changed = {
        let mut saved = state.saved.lock().unwrap();
        let changed = saved.layouts.get(&root) != Some(&layout);
        saved.layouts.insert(root, layout);
        changed
    };
    if changed {
        state.save();
    }
}

/// The colour of the project, for all its windows, written in its `review.toml`.
#[tauri::command]
fn set_colour(app: AppHandle, webview: Webview, state: State<'_, AppState>, colour: Colour) -> Result<(), String> {
    let root = root_of(&webview)?;
    state
        .projects
        .lock()
        .unwrap()
        .get_mut(&root)
        .ok_or("the project is not open")?
        .set_colour(colour)
        .map_err(|e| e.to_string())?;
    windows::relabelled(&app, &root);
    windows::changed(&app, &root);
    Ok(())
}

/// The viewer of a project's page moved or changed size: the pane follows.
#[tauri::command]
fn pane_area(app: AppHandle, webview: Webview, area: pane::Area) {
    pane::set_area(&app, webview.window().label(), area);
}

/// The pane's hide button.
#[tauri::command]
fn pane_hide(app: AppHandle, webview: Webview) {
    pane::hide(&app, webview.window().label());
}

/// The pane's side button.
#[tauri::command]
fn pane_side(app: AppHandle, webview: Webview) {
    pane::switch_side(&app, webview.window().label());
}

/// The pane's collapse button: `collapsed` to its header bar, or expanded.
#[tauri::command]
fn pane_collapse(app: AppHandle, webview: Webview, collapsed: bool) {
    pane::collapse(&app, webview.window().label(), collapsed);
}

/// The pane shown and expanded in this window, with the keys (the page's References…).
#[tauri::command]
fn pane_show(app: AppHandle, webview: Webview) {
    pane::show(&app, webview.window().label());
}

/// Escape in the pane: the keys back to the PDF, the pane left as it is.
#[tauri::command]
fn pane_release(app: AppHandle, webview: Webview) {
    pane::release(&app, webview.window().label());
}

/// ⌘' and ⌘;: a quote of the PDF into the review, at the pane's cursor.
#[tauri::command]
fn quote(app: AppHandle, webview: Webview, text: String) {
    pane::insert(&app, webview.window().label(), &text);
}

/// A card of the page shown or gone: the pane, above the page, hides meanwhile.
#[tauri::command]
fn page_covered(app: AppHandle, webview: Webview, covered: bool) {
    pane::set_covered(&app, webview.window().label(), covered);
}

/// The pane's edge dragged to `size` (points); `done` when let go.
#[tauri::command]
fn pane_size(app: AppHandle, webview: Webview, size: f64, done: bool) {
    pane::resize(&app, webview.window().label(), size, done);
}

/// The review of the project, for its pane.
#[tauri::command]
fn review(app: AppHandle, webview: Webview) -> Result<review::ReviewView, String> {
    review::read(&app, webview.window().label())
}

/// The review as typed, written to the file the pane read, if it is there.
#[tauri::command]
fn save_review(path: PathBuf, text: String) -> Result<(), String> {
    review::write(&path, &text)
}

/// The review file, empty, where the project says it is: the pane's button when it is not there.
#[tauri::command]
fn create_review(app: AppHandle, webview: Webview) -> Result<(), String> {
    review::create(&app, webview.window().label())
}

/// A Markdown file of the project as its review, shown in the pane.
#[tauri::command]
fn set_review(app: AppHandle, webview: Webview, path: PathBuf) -> Result<(), String> {
    let window = webview.window().label().to_string();
    review::choose(&app, &window, &path)?;
    pane::show(&app, &window);
    Ok(())
}

/// Where the pane is, for its page as it loads.
#[tauri::command]
fn pane_placement(app: AppHandle, webview: Webview) -> Option<pane::Placed> {
    pane::placement(&app, webview.window().label())
}

/// The pane took the keys, or gave them up.
#[tauri::command]
fn pane_focus(app: AppHandle, webview: Webview, focused: bool) {
    pane::set_focused(&app, webview.window().label(), focused);
}

/// The review as a PDF in the project's `export/` folder, `name` the stem of the file; `html` is the
/// document. The path written.
#[tauri::command]
async fn export_pdf(app: AppHandle, webview: Webview, name: String, html: String) -> Result<PathBuf, String> {
    let root = root_of(&webview)?;
    export::pdf(&app, &root, &name, html).await
}

/// The HTML of the export for the page of its hidden window.
#[tauri::command]
fn export_html(app: AppHandle, webview: Webview) -> Result<String, String> {
    export::html(&app, webview.window().label())
}

/// The page of the hidden window rendered the HTML: its web view is printed to the PDF.
#[tauri::command]
fn export_rendered(webview: Webview) -> Result<(), String> {
    export::rendered(&webview)
}

/// The review as text in the project's `export/` folder and on the pasteboard. The path written.
#[tauri::command]
fn export_text(app: AppHandle, webview: Webview, name: String, text: String) -> Result<PathBuf, String> {
    let root = root_of(&webview)?;
    export::text(&app, &root, &name, &text)
}

/// The references of the project (its `.bib` files and its papers library) that match `query`: the first
/// `limit` of them, with the words that matched marked.
#[tauri::command]
async fn references(app: AppHandle, webview: Webview, query: String, limit: usize) -> Result<Found, String> {
    let root = root_of(&webview)?;
    let library = refs::library(&app, &root)?;
    Ok(refs::search(&library, &query, limit))
}

/// The references closest to a reference as it is written (one selected in the PDF), the best first.
#[tauri::command]
async fn closest_references(app: AppHandle, webview: Webview, text: String, limit: usize) -> Result<Found, String> {
    let root = root_of(&webview)?;
    Ok(refs::library(&app, &root)?.closest(&text, limit))
}

/// The `.bib` files the references of the project are read from.
#[tauri::command]
async fn reference_sources(app: AppHandle, webview: Webview) -> Result<Vec<PathBuf>, String> {
    let root = root_of(&webview)?;
    Ok(refs::sources_of(&app, &root))
}

/// The PDF of a reference opened in a tab of the page of the calling window (`open-file`, the path).
#[tauri::command]
fn open_reference_pdf(app: AppHandle, webview: Webview, path: PathBuf) -> Result<(), String> {
    refs::open_pdf(&app, webview.window().label(), &path)
}

/// A file or folder of the project (not a linked one) moved to the Trash.
#[tauri::command]
fn trash(app: AppHandle, webview: Webview, path: PathBuf) -> Result<(), String> {
    let root = root_of(&webview)?;
    let path = path.canonicalize().map_err(|e| format!("{}: {e}", path.display()))?;
    if path == root || !path.starts_with(&root) {
        return Err(format!("{}: not a file of the project", path.display()));
    }
    trash::trash(&path)?;
    windows::changed(&app, &root);
    Ok(())
}

/// Look for an update now.
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<Available>, String> {
    update::check(&app).await
}

/// The update found by the last look, if any.
#[tauri::command]
fn found_update(updates: State<'_, Updates>) -> Option<Available> {
    updates.available()
}

/// The update installed just before this start, if any.
#[tauri::command]
fn installed_update(updates: State<'_, Updates>) -> Option<Installed> {
    updates.installed()
}

/// Install the update found, and restart.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    update::install(&app).await
}

/// The app's configuration, assets and permissions. Made once: the example that checks the PDF export
/// builds an app of its own with it.
pub fn context() -> tauri::Context {
    tauri::generate_context!()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_pdfkit::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            app.manage(AppState {
                projects: Mutex::new(HashMap::new()),
                windows: Mutex::new(HashMap::new()),
                saved: Mutex::new(Saved::load(&data)),
                settings: Mutex::new(Settings::load(&data)),
                data,
                next: AtomicUsize::new(1),
                panes: Mutex::new(pane::Panes::default()),
                watchers: Mutex::new(HashMap::new()),
                file_watchers: Mutex::new(HashMap::new()),
                exports: Mutex::new(HashMap::new()),
                references: Mutex::new(HashMap::new()),
                started: Mutex::new(HashMap::new()),
            });
            app.manage(Updates::new(app.handle()));
            app.set_menu(menu::build(app.handle())?)?;
            app.on_menu_event(menu::on_event);
            windows::reopen(app.handle());
            update::watch(app.handle());
            // after an update, the Settings window tells of it
            if app.state::<Updates>().installed().is_some() {
                windows::show_settings(app.handle(), Some("updates"));
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { .. } => windows::closing(window.app_handle(), window.label()),
            WindowEvent::Destroyed => windows::closed(window.app_handle(), window.label()),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            settings,
            set_settings,
            open_project,
            choose_project,
            new_review,
            recent,
            bar,
            switch_project,
            remove_from_bar,
            open_in_new_window,
            project,
            project_info,
            set_project_info,
            create_project,
            cancel_project,
            add_files,
            unlink,
            new_file,
            read_file,
            write_file,
            watch_file,
            unwatch_file,
            layout,
            set_layout,
            set_colour,
            pane_area,
            pane_hide,
            pane_side,
            pane_collapse,
            pane_release,
            pane_show,
            quote,
            page_covered,
            pane_size,
            pane_focus,
            pane_placement,
            review,
            save_review,
            create_review,
            set_review,
            export_pdf,
            export_html,
            export_rendered,
            export_text,
            references,
            closest_references,
            reference_sources,
            open_reference_pdf,
            trash,
            check_update,
            found_update,
            installed_update,
            install_update
        ])
        .build(context())
        .expect("error while building papers-review")
        .run(|app, event| match event {
            // the Dock icon clicked, no window open
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => windows::show_start(app),
            // folders dropped on the Dock icon, or opened with the app, are projects; PDFs are manuscripts to review
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                for url in urls {
                    let Ok(path) = url.to_file_path() else {
                        continue;
                    };
                    if manuscript::is_pdf(&path) {
                        manuscript::begin(app, path);
                    } else if let Err(e) = windows::open_project(app, &path) {
                        eprintln!("{e}");
                    }
                }
            }
            _ => {}
        });
}
