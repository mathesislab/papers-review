//! papers-review: a window per review project (a folder), and a start
//! window. The project of a window is found by its label (`windows`).

mod menu;
mod state;
mod windows;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;

use review_core::{Colour, Node, Project};
use serde::Serialize;
use tauri::{AppHandle, Manager, RunEvent, State, Webview, WindowEvent};

use state::{Layout, Saved};

pub struct AppState {
    /// the projects open, by their folder
    pub projects: Mutex<HashMap<PathBuf, Project>>,
    /// the project of each window, by the window's label
    pub windows: Mutex<HashMap<String, PathBuf>>,
    pub saved: Mutex<Saved>,
    /// the app's data folder, where `saved` is written
    data: PathBuf,
    /// the number of the next window
    next: AtomicUsize,
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
                data,
                next: AtomicUsize::new(1),
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
            open_project,
            choose_project,
            recent,
            project,
            layout,
            set_layout,
            set_colour
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
