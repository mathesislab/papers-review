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
    /// the user's own sequences typed that become symbols, each with what it
    /// becomes; an empty replacement turns off a default sequence
    pub symbols: Vec<(String, String)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { project_bar: true, symbols: Vec::new() }
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
    fn the_symbols_are_pairs_and_a_file_without_them_has_none() {
        let settings: Settings = serde_json::from_str(r#"{"symbols": [["(c)", "©"], ["->", ""]]}"#).unwrap();
        assert_eq!(settings.symbols, [("(c)".to_string(), "©".to_string()), ("->".to_string(), String::new())]);
        assert!(serde_json::from_str::<Settings>(r#"{"project_bar": false}"#).unwrap().symbols.is_empty());
    }

    #[test]
    fn saved_and_loaded_back_a_missing_or_broken_file_is_the_defaults() {
        let dir = std::env::temp_dir().join(format!("papers-review-settings-{}", std::process::id()));
        assert_eq!(Settings::load(&dir), Settings::default());
        let settings = Settings { project_bar: false, symbols: vec![("(c)".into(), "©".into())] };
        settings.save(&dir);
        assert_eq!(Settings::load(&dir), settings);
        std::fs::write(dir.join(FILE), "{").unwrap();
        assert_eq!(Settings::load(&dir), Settings::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
