//! What the app keeps between sessions, in its data folder (`state.json`): the projects opened last, the projects in
//! the project bar, the windows open as it quit and each project's tabs (papers-tabs), its sidebar, its review pane,
//! and the page in view of each PDF. Not in `review.toml`: how the app looked is not the project's.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use papers_tabs::{Layout, SavedWindow, Sidebar, Sidebars};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const FILE: &str = "state.json";
/// The most projects Open Recent lists.
const RECENT: usize = 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    /// the projects opened, the last first
    pub recent: Vec<PathBuf>,
    /// the projects in the project bar, in order
    pub bar: Vec<PathBuf>,
    /// the windows open as the app quit, each with its projects, to open again at the next start
    pub windows: Vec<SavedWindow>,
    /// the tabs of each project as last left
    pub tabs: BTreeMap<PathBuf, Layout>,
    /// the sidebar (the files) of each project, and the last one changed (`sidebars` and `sidebar` in the file)
    #[serde(flatten)]
    pub sidebars: Sidebars,
    /// the page in view of each PDF (from 1)
    pub pages: BTreeMap<PathBuf, usize>,
    /// the review pane of each project
    pub panes: HashMap<PathBuf, PaneLayout>,
    /// from before the tabs were pages (papers-tabs): the projects whose window was open as the app quit, and how
    /// each project's window looked; read once, into the above
    #[serde(skip_serializing)]
    open: Vec<PathBuf>,
    #[serde(skip_serializing)]
    layouts: HashMap<PathBuf, OldLayout>,
}

/// How a project's window looked before the tabs were pages.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct OldLayout {
    files: Option<bool>,
    files_width: Option<f64>,
    tabs: Vec<PathBuf>,
    front: Option<PathBuf>,
    pages: HashMap<PathBuf, usize>,
}

/// The side of the viewer the review pane lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Bottom,
    Right,
}

/// The review pane of a project: shown or not, collapsed to its header bar or
/// not, its side, its height at the bottom and its width on the right (points).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneLayout {
    pub shown: bool,
    pub collapsed: bool,
    pub side: Side,
    pub bottom: f64,
    pub right: f64,
}

impl Default for PaneLayout {
    fn default() -> Self {
        PaneLayout {
            shown: false,
            collapsed: false,
            side: Side::Bottom,
            bottom: 280.0,
            right: 420.0,
        }
    }
}

/// What `dir` keeps in `file`; the default when it is missing or broken.
pub fn read<T: DeserializeOwned + Default>(dir: &Path, file: &str) -> T {
    fs::read_to_string(dir.join(file))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// `value` written to `file` of `dir` atomically: a file beside it, renamed.
pub fn write<T: Serialize>(dir: &Path, file: &str, value: &T) {
    let written = fs::create_dir_all(dir).and_then(|()| {
        let text = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
        let temporary = dir.join(format!(".{file}.tmp"));
        fs::write(&temporary, text)?;
        fs::rename(&temporary, dir.join(file))
    });
    if let Err(e) = written {
        eprintln!("{}: {e}", dir.join(file).display());
    }
}

impl Saved {
    /// What `dir` keeps; nothing when the file is missing or broken.
    pub fn load(dir: &Path) -> Saved {
        let mut saved: Saved = read(dir, FILE);
        saved.migrate();
        saved
    }

    pub fn save(&self, dir: &Path) {
        write(dir, FILE, self);
    }

    /// What `dir` kept before the tabs were pages, turned into what it keeps now.
    fn migrate(&mut self) {
        if self.windows.is_empty() {
            self.windows = self.open.drain(..).map(|root| SavedWindow { projects: vec![root] }).collect();
        }
        for (root, old) in self.layouts.drain() {
            self.tabs.entry(root.clone()).or_insert(Layout { files: old.tabs, front: old.front });
            let sidebar = Sidebar { shown: old.files.unwrap_or(true), width: old.files_width.unwrap_or(240.0), tab: None };
            self.sidebars.sidebars.entry(root).or_insert(sidebar);
            for (pdf, page) in old.pages {
                self.pages.entry(pdf).or_insert(page);
            }
        }
    }

    /// A project made first of the recent ones. A new project is not, until it is defined.
    pub fn remembered(&mut self, root: &Path) {
        self.recent.retain(|r| r != root);
        self.recent.insert(0, root.to_path_buf());
        self.recent.truncate(RECENT);
    }

    /// A project put in the project bar, at the end, unless it is there.
    pub fn barred(&mut self, root: &Path) {
        if !self.bar.iter().any(|b| b == root) {
            self.bar.push(root.to_path_buf());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembered_projects_come_first_once() {
        let mut saved = Saved::default();
        let (a, b) = (Path::new("/a"), Path::new("/b"));
        saved.remembered(a);
        saved.remembered(b);
        saved.remembered(a);
        assert_eq!(saved.recent, [a, b]);
    }

    #[test]
    fn the_bar_takes_a_project_once_at_the_end() {
        let mut saved = Saved::default();
        let (a, b) = (Path::new("/a"), Path::new("/b"));
        saved.barred(a);
        saved.barred(b);
        saved.barred(a);
        assert_eq!(saved.bar, [a, b]);
    }

    #[test]
    fn a_state_file_from_before_the_bar_has_an_empty_one() {
        let saved: Saved = serde_json::from_str(r#"{"recent": ["/a"]}"#).unwrap();
        assert!(saved.bar.is_empty());
    }

    #[test]
    fn a_state_file_from_before_the_tabs_were_pages_keeps_its_windows_tabs_files_and_pages() {
        let mut saved: Saved = serde_json::from_str(
            r#"{"open": ["/a", "/b"], "layouts": {"/a": {"files": false, "files_width": 300, "tabs": ["/a/x.pdf"],
                "front": "/a/x.pdf", "pages": {"/a/x.pdf": 4}}}}"#,
        )
        .unwrap();
        saved.migrate();
        let window = |p: &str| SavedWindow { projects: vec![PathBuf::from(p)] };
        assert_eq!(saved.windows, [window("/a"), window("/b")]);
        let a = Path::new("/a");
        assert_eq!(saved.tabs[a], Layout { files: vec!["/a/x.pdf".into()], front: Some("/a/x.pdf".into()) });
        assert_eq!(saved.sidebars.sidebars[a], Sidebar { shown: false, width: 300.0, tab: None });
        assert_eq!(saved.pages[Path::new("/a/x.pdf")], 4);
        let text = serde_json::to_string(&saved).unwrap();
        assert!(!text.contains("layouts") && !text.contains("\"open\""));
    }

    #[test]
    fn saved_and_loaded_back_a_broken_file_is_nothing() {
        let dir = std::env::temp_dir().join(format!("papers-review-state-{}", std::process::id()));
        let mut saved = Saved::default();
        saved.remembered(Path::new("/p"));
        saved.sidebars.sidebars.insert(PathBuf::from("/p"), Sidebar { shown: false, ..Sidebar::default() });
        saved.save(&dir);
        let loaded = Saved::load(&dir);
        assert_eq!(loaded.recent, saved.recent);
        assert!(!loaded.sidebars.sidebars[Path::new("/p")].shown);
        fs::write(dir.join(FILE), "{").unwrap();
        assert!(Saved::load(&dir).recent.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
