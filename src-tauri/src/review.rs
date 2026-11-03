//! The review of a project in its pane: read, written as it is typed,
//! watched for the changes made elsewhere (an editor, a sync client); any
//! Markdown file of the project as the review.

use std::path::{Path, PathBuf};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, EventTarget, Manager};

use crate::{AppState, pane, windows};

/// The review as the pane shows it.
#[derive(Debug, Serialize)]
pub struct ReviewView {
    pub path: PathBuf,
    pub name: String,
    pub text: String,
}

pub fn read(app: &AppHandle, window: &str) -> Result<ReviewView, String> {
    let root = windows::root_of(app, window).ok_or("no project in this window")?;
    let state = app.state::<AppState>();
    let projects = state.projects.lock().unwrap();
    let project = projects.get(&root).ok_or("the project is not open")?;
    let path = project.review_path();
    Ok(ReviewView {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        text: project.read_review().map_err(|e| e.to_string())?,
        path,
    })
}

/// The review written to the file the pane read (it may have changed to
/// another since: what was typed goes where it was read from). A file that
/// was not there yet shows in the files.
pub fn write(app: &AppHandle, window: &str, path: &Path, text: &str) -> Result<(), String> {
    let root = windows::root_of(app, window).ok_or("no project in this window")?;
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown")) {
        return Err(format!("{}: not a Markdown file", path.display()));
    }
    let new = !path.exists();
    review_core::project::write_atomically(path, text).map_err(|e| e.to_string())?;
    if new {
        windows::changed(app, &root);
    }
    Ok(())
}

/// Another Markdown file as the review of the project (in its review.toml):
/// watched instead, and the panes told.
pub fn choose(app: &AppHandle, window: &str, path: &Path) -> Result<(), String> {
    let root = windows::root_of(app, window).ok_or("no project in this window")?;
    {
        let state = app.state::<AppState>();
        let mut projects = state.projects.lock().unwrap();
        let project = projects.get_mut(&root).ok_or("the project is not open")?;
        project.set_review(path).map_err(|e| e.to_string())?;
    }
    moved(app, &root);
    Ok(())
}

/// The review of the project of `root` is another file than the panes read, or was made: watched instead,
/// and the panes told.
pub fn moved(app: &AppHandle, root: &Path) {
    watch(app, root);
    tell(app, root, "review-switched");
}

/// The panes of the project's windows told `event`.
fn tell(app: &AppHandle, root: &Path, event: &str) {
    for window in windows::of_project(app, root) {
        let _ = app.emit_to(EventTarget::webview(pane::label(&window)), event, ());
    }
}

/// The review of the project of `root` watched (its folder, as a file
/// written atomically is a new one): a change made to it elsewhere goes to
/// the panes, which read it again unless it is what they wrote.
pub fn watch(app: &AppHandle, root: &Path) {
    let state = app.state::<AppState>();
    let Some(path) = state.projects.lock().unwrap().get(root).map(|p| p.review_path()) else {
        return;
    };
    let Some(folder) = path.parent().map(Path::to_path_buf) else {
        return;
    };
    let (handle, project, file) = (app.clone(), root.to_path_buf(), path.clone());
    let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_))
            && event.paths.contains(&file)
        {
            tell(&handle, &project, "review-changed");
        }
    });
    let watcher: Option<RecommendedWatcher> = watcher
        .and_then(|mut w| w.watch(&folder, RecursiveMode::NonRecursive).map(|()| w))
        .map_err(|e| eprintln!("{}: not watched ({e})", folder.display()))
        .ok();
    let mut watchers = state.watchers.lock().unwrap();
    match watcher {
        Some(watcher) => {
            watchers.insert(root.to_path_buf(), watcher);
        }
        None => {
            watchers.remove(root);
        }
    }
}

/// The project of `root` closed: its review no longer watched.
pub fn unwatch(app: &AppHandle, root: &Path) {
    app.state::<AppState>().watchers.lock().unwrap().remove(root);
}
