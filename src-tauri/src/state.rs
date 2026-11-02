//! What the app keeps between sessions, in its data folder (`state.json`):
//! the projects opened last, the windows open as it quit, and how each
//! project's window looked. Not in `review.toml`: how the app looked is not
//! the project's.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE: &str = "state.json";
/// The most projects Open Recent lists.
const RECENT: usize = 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    /// the projects opened, the last first
    pub recent: Vec<PathBuf>,
    /// the projects whose window was open as the app quit
    pub open: Vec<PathBuf>,
    pub layouts: HashMap<PathBuf, Layout>,
    /// the review pane of each project
    pub panes: HashMap<PathBuf, PaneLayout>,
}

/// How a project's window looked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    /// the files panel shown, and its width (points)
    pub files: bool,
    pub files_width: f64,
    /// the files open in tabs, and the one in front
    pub tabs: Vec<PathBuf>,
    pub front: Option<PathBuf>,
    /// the page in view of each PDF open (from 1)
    pub pages: HashMap<PathBuf, usize>,
}

/// The side of the viewer the review pane lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Bottom,
    Right,
}

/// The review pane of a project: shown or not, its side, its height at the
/// bottom and its width on the right (points).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneLayout {
    pub shown: bool,
    pub side: Side,
    pub bottom: f64,
    pub right: f64,
}

impl Default for PaneLayout {
    fn default() -> Self {
        PaneLayout {
            shown: false,
            side: Side::Bottom,
            bottom: 280.0,
            right: 420.0,
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            files: true,
            files_width: 240.0,
            tabs: Vec::new(),
            front: None,
            pages: HashMap::new(),
        }
    }
}

impl Saved {
    /// What `dir` keeps; nothing when the file is missing or broken.
    pub fn load(dir: &Path) -> Saved {
        fs::read_to_string(dir.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Written atomically: a file beside it, renamed.
    pub fn save(&self, dir: &Path) {
        let written = fs::create_dir_all(dir)
            .and_then(|()| {
                let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
                let temporary = dir.join(format!(".{FILE}.tmp"));
                fs::write(&temporary, text)?;
                fs::rename(&temporary, dir.join(FILE))
            });
        if let Err(e) = written {
            eprintln!("{}: {e}", dir.join(FILE).display());
        }
    }

    /// A project opened: first of the recent ones, and open.
    pub fn opened(&mut self, root: &Path) {
        self.recent.retain(|r| r != root);
        self.recent.insert(0, root.to_path_buf());
        self.recent.truncate(RECENT);
        if !self.open.iter().any(|o| o == root) {
            self.open.push(root.to_path_buf());
        }
    }

    /// A project's window closed by hand: it does not open at the next start.
    pub fn closed(&mut self, root: &Path) {
        self.open.retain(|o| o != root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opened_projects_come_first_once_and_closed_ones_stay_recent() {
        let mut saved = Saved::default();
        let (a, b) = (Path::new("/a"), Path::new("/b"));
        saved.opened(a);
        saved.opened(b);
        saved.opened(a);
        assert_eq!(saved.recent, [a, b]);
        assert_eq!(saved.open, [a, b]);
        saved.closed(a);
        assert_eq!(saved.open, [b]);
        assert_eq!(saved.recent, [a, b]);
    }

    #[test]
    fn saved_and_loaded_back_a_broken_file_is_nothing() {
        let dir = std::env::temp_dir().join(format!("papers-review-state-{}", std::process::id()));
        let mut saved = Saved::default();
        saved.opened(Path::new("/p"));
        saved.layouts.insert(PathBuf::from("/p"), Layout { files: false, ..Layout::default() });
        saved.save(&dir);
        let loaded = Saved::load(&dir);
        assert_eq!(loaded.recent, saved.recent);
        assert!(!loaded.layouts[Path::new("/p")].files);
        fs::write(dir.join(FILE), "{").unwrap();
        assert!(Saved::load(&dir).recent.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
