//! The files open in the tabs of a project's page: read, written, and watched for the changes made
//! elsewhere (an editor, a sync client). Only the files the project contains (`Project::contains`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, EventTarget, Manager};

use crate::{AppState, windows};

/// The files watched for the pages of the windows, by window and path: how many times the page asked
/// (a file open in two tabs), and the watcher of its folder.
pub type Watched = HashMap<(String, PathBuf), (usize, RecommendedWatcher)>;

/// The text of a file of the project of `window`.
pub fn read(app: &AppHandle, window: &str, path: &Path) -> Result<String, String> {
    contained(app, window, path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    String::from_utf8(bytes).map_err(|_| format!("{}: not a text file", path.display()))
}

/// `text` written to a file of the project of `window`, atomically. A file that was not there yet
/// shows in the files.
pub fn write(app: &AppHandle, window: &str, path: &Path, text: &str) -> Result<(), String> {
    let root = contained(app, window, path)?;
    let new = !path.exists();
    review_core::project::write_atomically(path, text).map_err(|e| e.to_string())?;
    if new {
        windows::changed(app, &root);
    }
    Ok(())
}

/// The file watched for the page of `window`: a change made to it goes to the page as `file-changed`,
/// with the path. Watched once more each time it is asked, until as many `unwatch`.
pub fn watch(app: &AppHandle, window: &str, path: &Path) -> Result<(), String> {
    contained(app, window, path)?;
    let state = app.state::<AppState>();
    let mut watched = state.file_watchers.lock().unwrap();
    let key = (window.to_string(), path.to_path_buf());
    if let Some((count, _)) = watched.get_mut(&key) {
        *count += 1;
        return Ok(());
    }
    let folder = path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .ok_or_else(|| format!("{}: no folder to watch", path.display()))?;
    // events name the file by its resolved folder
    let file = folder.join(path.file_name().unwrap_or_default());
    let (handle, page, named) = (app.clone(), windows::page(window), path.to_string_lossy().into_owned());
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_))
            && event.paths.contains(&file)
        {
            let _ = handle.emit_to(EventTarget::webview(page.clone()), "file-changed", named.clone());
        }
    })
    .map_err(|e| format!("{}: not watched ({e})", path.display()))?;
    watcher
        .watch(&folder, RecursiveMode::NonRecursive)
        .map_err(|e| format!("{}: not watched ({e})", path.display()))?;
    watched.insert(key, (1, watcher));
    Ok(())
}

/// One watch of the file for the page of `window` given up; the file is no longer watched when none is left.
pub fn unwatch(app: &AppHandle, window: &str, path: &Path) {
    let state = app.state::<AppState>();
    let mut watched = state.file_watchers.lock().unwrap();
    let key = (window.to_string(), path.to_path_buf());
    match watched.get_mut(&key) {
        Some((count, _)) if *count > 1 => *count -= 1,
        Some(_) => {
            watched.remove(&key);
        }
        None => {}
    }
}

/// Every file watched for the page of `window` given up: the window is closed, or shows another project.
pub fn unwatch_all(app: &AppHandle, window: &str) {
    app.state::<AppState>()
        .file_watchers
        .lock()
        .unwrap()
        .retain(|(label, _), _| label != window);
}

/// The project of `window`, when it contains `path`.
fn contained(app: &AppHandle, window: &str, path: &Path) -> Result<PathBuf, String> {
    let root = windows::root_of(app, window).ok_or("no project in this window")?;
    let state = app.state::<AppState>();
    let projects = state.projects.lock().unwrap();
    let project = projects.get(&root).ok_or("the project is not open")?;
    if !project.contains(path) {
        return Err(format!("{}: not a file of the project", path.display()));
    }
    Ok(root)
}
