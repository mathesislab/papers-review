//! The files open in the tabs of a project (each a page): read, written, and watched for the changes made
//! elsewhere (an editor, a sync client). Only the files the project contains (`Project::contains`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, EventTarget, Manager};

use crate::{AppState, windows};

/// The files watched for the pages, by page and path: how many times the page asked, and the watcher of its
/// folder.
pub type Watched = HashMap<(String, PathBuf), (usize, RecommendedWatcher)>;

/// The text of a file of the project of `root`.
pub fn read(app: &AppHandle, root: &Path, path: &Path) -> Result<String, String> {
    contained(app, root, path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    String::from_utf8(bytes).map_err(|_| format!("{}: not a text file", path.display()))
}

/// `text` written to a file of the project of `root`, atomically. A file that was not there yet
/// shows in the files.
pub fn write(app: &AppHandle, root: &Path, path: &Path, text: &str) -> Result<(), String> {
    contained(app, root, path)?;
    let new = !path.exists();
    review_core::project::write_atomically(path, text).map_err(|e| e.to_string())?;
    if new {
        windows::changed(app, root);
    }
    Ok(())
}

/// The file of the project of `root` watched for the page `page`: a change made to it goes to the page as
/// `file-changed`, with the path. Watched once more each time it is asked, until as many `unwatch`.
pub fn watch(app: &AppHandle, page: &str, root: &Path, path: &Path) -> Result<(), String> {
    contained(app, root, path)?;
    let state = app.state::<AppState>();
    let mut watched = state.file_watchers.lock().unwrap();
    let key = (page.to_string(), path.to_path_buf());
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
    let (handle, page, named) = (app.clone(), page.to_string(), path.to_string_lossy().into_owned());
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

/// One watch of the file for the page `page` given up; the file is no longer watched when none is left.
pub fn unwatch(app: &AppHandle, page: &str, path: &Path) {
    let state = app.state::<AppState>();
    let mut watched = state.file_watchers.lock().unwrap();
    let key = (page.to_string(), path.to_path_buf());
    match watched.get_mut(&key) {
        Some((count, _)) if *count > 1 => *count -= 1,
        Some(_) => {
            watched.remove(&key);
        }
        None => {}
    }
}

/// Every file watched for the page `page` given up: it is closed, or shows another file.
pub fn unwatch_all(app: &AppHandle, page: &str) {
    app.state::<AppState>()
        .file_watchers
        .lock()
        .unwrap()
        .retain(|(label, _), _| label != page);
}

/// Whether the project of `root` contains `path`.
fn contained(app: &AppHandle, root: &Path, path: &Path) -> Result<(), String> {
    let state = app.state::<AppState>();
    let projects = state.projects.lock().unwrap();
    let project = projects.get(root).ok_or("the project is not open")?;
    if !project.contains(path) {
        return Err(format!("{}: not a file of the project", path.display()));
    }
    Ok(())
}
