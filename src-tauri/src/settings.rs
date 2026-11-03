//! The app's settings, in its data folder (`settings.json`), set in the
//! Settings window.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::state;

const FILE: &str = "settings.json";

/// How a date is written and read in the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DateFormat {
    /// 2026-11-14
    #[default]
    Ymd,
    /// 14.11.2026
    Dmy,
    /// 11/14/2026
    Mdy,
}

/// The font of the editors (the review pane, the files edited in tabs).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorFont {
    #[default]
    System,
    Serif,
    Mono,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// the projects in one window, with a bar to switch between them; off,
    /// each project opens in a window of its own
    pub project_bar: bool,
    /// the user's own sequences typed that become symbols, each with what it
    /// becomes; an empty replacement turns off a default sequence
    pub symbols: Vec<(String, String)>,
    pub date_format: DateFormat,
    /// the font of the editors and its size (points)
    pub editor_font: EditorFont,
    pub editor_size: f64,
    /// the reviewer's name, for the header of the PDF export
    pub reviewer_name: String,
    /// the review signed with `reviewer_name`; off, it is anonymous
    pub sign: bool,
    /// the header of the PDF has the journal, the manuscript, the title and the date; off, it is the heading alone
    pub header_details: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            project_bar: true,
            symbols: Vec::new(),
            date_format: DateFormat::default(),
            editor_font: EditorFont::default(),
            editor_size: 13.5,
            reviewer_name: String::new(),
            sign: false,
            header_details: false,
        }
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
    fn dates_year_first_and_the_system_font_by_default() {
        let settings: Settings = serde_json::from_str(r#"{"date_format": "dmy"}"#).unwrap();
        assert_eq!(settings.date_format, DateFormat::Dmy);
        assert_eq!(Settings::default().date_format, DateFormat::Ymd);
        assert_eq!(Settings::default().editor_font, EditorFont::System);
    }

    #[test]
    fn the_review_is_anonymous_with_a_plain_header_unless_the_settings_say_otherwise() {
        let settings = Settings::default();
        assert_eq!((settings.reviewer_name.as_str(), settings.sign, settings.header_details), ("", false, false));
        let old: Settings = serde_json::from_str(r#"{"project_bar": false}"#).unwrap();
        assert_eq!((old.reviewer_name.as_str(), old.sign, old.header_details), ("", false, false));
        let set: Settings = serde_json::from_str(r#"{"reviewer_name": "A. Reviewer", "sign": true, "header_details": true}"#).unwrap();
        assert_eq!((set.reviewer_name.as_str(), set.sign, set.header_details), ("A. Reviewer", true, true));
    }

    #[test]
    fn saved_and_loaded_back_a_missing_or_broken_file_is_the_defaults() {
        let dir = std::env::temp_dir().join(format!("papers-review-settings-{}", std::process::id()));
        assert_eq!(Settings::load(&dir), Settings::default());
        let settings = Settings {
            project_bar: false,
            symbols: vec![("(c)".into(), "©".into())],
            date_format: DateFormat::Dmy,
            editor_font: EditorFont::Serif,
            editor_size: 16.0,
            reviewer_name: "A. Reviewer".into(),
            sign: true,
            header_details: true,
        };
        settings.save(&dir);
        assert_eq!(Settings::load(&dir), settings);
        std::fs::write(dir.join(FILE), "{").unwrap();
        assert_eq!(Settings::load(&dir), Settings::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
