//! papers-review: a window per review project (a folder), or one window with a
//! bar to switch between projects (a setting), a start window and a Settings
//! window. The project of a window is found by its label (`windows`).

mod menu;
mod pane;
mod review;
mod settings;
mod state;
mod windows;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;

use review_core::{Colour, Node, Project};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, Webview, WindowEvent};

use settings::Settings;
use state::{Layout, Saved};

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
    journal: Option<String>,
    manuscript: Option<String>,
    due: Option<String>,
    colour: Colour,
    files: Vec<Node>,
    linked: Vec<Node>,
    /// the Markdown file of the review
    review: PathBuf,
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
fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) {
    *state.settings.lock().unwrap() = settings.clone();
    settings.save(&state.data);
    let _ = app.emit("settings-changed", settings);
}

#[tauri::command]
fn open_project(app: AppHandle, path: PathBuf) -> Result<(), String> {
    windows::open_project(&app, &path)
}

#[tauri::command]
fn choose_project(app: AppHandle) {
    menu::choose_project(&app);
}

#[tauri::command]
fn recent(state: State<'_, AppState>) -> Vec<Recent> {
    let recent = state.saved.lock().unwrap().recent.clone();
    recent
        .into_iter()
        .map(|root| match Project::open(&root) {
            Ok(project) => Recent {
                name: project.name(),
                colour: project.manifest.colour.unwrap_or(Colour::None),
                root,
                missing: false,
            },
            Err(_) => Recent {
                name: root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                colour: Colour::None,
                root,
                missing: true,
            },
        })
        .collect()
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
        journal: manifest.journal.clone(),
        manuscript: manifest.manuscript.clone(),
        due: manifest.due.map(|d| d.to_string()),
        colour: manifest.colour.unwrap_or(Colour::None),
        files: project.files(),
        linked: project.linked(),
        review: project.review_path(),
        root,
    })
}

/// How the window of the project looked.
#[tauri::command]
fn layout(webview: Webview, state: State<'_, AppState>) -> Result<Layout, String> {
    let root = root_of(&webview)?;
    let saved = state.saved.lock().unwrap();
    Ok(saved.layouts.get(&root).cloned().unwrap_or_default())
}

#[tauri::command]
fn set_layout(webview: Webview, state: State<'_, AppState>, layout: Layout) -> Result<(), String> {
    let root = root_of(&webview)?;
    let changed = {
        let mut saved = state.saved.lock().unwrap();
        let changed = saved.layouts.get(&root) != Some(&layout);
        saved.layouts.insert(root, layout);
        changed
    };
    if changed {
        state.save();
    }
    Ok(())
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

/// Escape in the pane: the keys back to the PDF, the pane left as it is.
#[tauri::command]
fn pane_release(app: AppHandle, webview: Webview) {
    pane::release(&app, webview.window().label());
}

/// ⌘': a quote of the PDF into the review, at the pane's cursor.
#[tauri::command]
fn quote(app: AppHandle, webview: Webview, text: String) {
    pane::insert(&app, webview.window().label(), &text);
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

/// The review as typed, written to the file the pane read.
#[tauri::command]
fn save_review(app: AppHandle, webview: Webview, path: PathBuf, text: String) -> Result<(), String> {
    review::write(&app, webview.window().label(), &path, &text)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_pdfkit::init())
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
            });
            app.set_menu(menu::build(app.handle())?)?;
            app.on_menu_event(menu::on_event);
            windows::reopen(app.handle());
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
            recent,
            project,
            layout,
            set_layout,
            set_colour,
            pane_area,
            pane_hide,
            pane_side,
            pane_collapse,
            pane_release,
            quote,
            pane_size,
            pane_focus,
            pane_placement,
            review,
            save_review,
            set_review
        ])
        .build(tauri::generate_context!())
        .expect("error while building papers-review")
        .run(|app, event| match event {
            // the Dock icon clicked, no window open
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => windows::show_start(app),
            // folders dropped on the Dock icon, or opened with the app
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                for url in urls {
                    if let Ok(path) = url.to_file_path()
                        && let Err(e) = windows::open_project(app, &path)
                    {
                        eprintln!("{e}");
                    }
                }
            }
            _ => {}
        });
}
