//! The app's settings, in its data folder (`settings.json`), set in the
//! Settings window.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::state;

const FILE: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// the projects in one window, with a bar to switch between them; off,
    /// each project opens in a window of its own
    pub project_bar: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { project_bar: true }
    }
}

impl Settings {
    /// What `dir` keeps; the defaults when the file is missing or broken.
    pub fn load(dir: &Path) -> Settings {
        state::read(dir, FILE)
    }

    pub fn save(&self, dir: &Path) {
        state::write(dir, FILE, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_is_on_by_default_and_a_file_without_it_leaves_it_on() {
        assert!(Settings::default().project_bar);
        assert!(serde_json::from_str::<Settings>("{}").unwrap().project_bar);
        assert!(!serde_json::from_str::<Settings>(r#"{"project_bar": false}"#).unwrap().project_bar);
    }

    #[test]
    fn saved_and_loaded_back_a_missing_or_broken_file_is_the_defaults() {
        let dir = std::env::temp_dir().join(format!("papers-review-settings-{}", std::process::id()));
        assert_eq!(Settings::load(&dir), Settings::default());
        Settings { project_bar: false }.save(&dir);
        assert!(!Settings::load(&dir).project_bar);
        std::fs::write(dir.join(FILE), "{").unwrap();
        assert_eq!(Settings::load(&dir), Settings::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
