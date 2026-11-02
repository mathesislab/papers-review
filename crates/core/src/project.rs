//! A project: a folder, and what `review.toml` in it adds (the title, the
//! journal, the due date, the colour, the files linked from elsewhere). The
//! file is read as it is; a change writes only its own key, so the comments
//! and the keys the app does not know stay.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::{Datetime, DocumentMut, value};

use crate::colour::Colour;
use crate::files::{self, Node};

/// The file of the app in a project folder.
pub const MANIFEST: &str = "review.toml";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}: not a folder")]
    NotAFolder(PathBuf),
    #[error("{0}: {1}")]
    Read(PathBuf, io::Error),
    #[error("{0}: {1}")]
    Manifest(PathBuf, String),
    #[error("{0}: {1}")]
    Write(PathBuf, io::Error),
}

/// What `review.toml` says; all of it optional.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub title: Option<String>,
    pub journal: Option<String>,
    /// the manuscript's identifier, as the journal gives it
    pub manuscript: Option<String>,
    pub due: Option<Datetime>,
    /// none yet: the app gives one when the project first opens
    pub colour: Option<Colour>,
    /// files elsewhere, part of the project
    #[serde(default)]
    pub linked: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    /// `review.toml` as written, changed key by key
    document: DocumentMut,
}

impl Project {
    /// The project of a folder, with its `review.toml` if there is one.
    pub fn open(root: &Path) -> Result<Project, Error> {
        if !root.is_dir() {
            return Err(Error::NotAFolder(root.to_path_buf()));
        }
        let path = root.join(MANIFEST);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(Error::Read(path, e)),
        };
        let document: DocumentMut = text
            .parse()
            .map_err(|e: toml_edit::TomlError| Error::Manifest(path.clone(), e.to_string()))?;
        let manifest = toml_edit::de::from_str(&text).map_err(|e| Error::Manifest(path, e.to_string()))?;
        Ok(Project {
            root: root.to_path_buf(),
            manifest,
            document,
        })
    }

    /// The title, else the name of the folder.
    pub fn name(&self) -> String {
        match &self.manifest.title {
            Some(title) if !title.trim().is_empty() => title.clone(),
            _ => self
                .root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.root.to_string_lossy().into_owned()),
        }
    }

    /// The colour, `review.toml` written.
    pub fn set_colour(&mut self, colour: Colour) -> Result<(), Error> {
        self.document["colour"] = value(colour.name());
        self.save()?;
        self.manifest.colour = Some(colour);
        Ok(())
    }

    /// The files of the folder, as a tree.
    pub fn files(&self) -> Vec<Node> {
        files::tree(&self.root)
    }

    /// The files linked from elsewhere; a relative path is from the folder,
    /// `~` the home folder.
    pub fn linked(&self) -> Vec<Node> {
        self.manifest
            .linked
            .iter()
            .map(|path| files::linked(&self.resolve(path)))
            .collect()
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        if let Ok(rest) = path.strip_prefix("~")
            && let Some(home) = std::env::var_os("HOME")
        {
            return PathBuf::from(home).join(rest);
        }
        self.root.join(path)
    }

    /// `review.toml` written whole, atomically: a file beside it, renamed.
    fn save(&self) -> Result<(), Error> {
        let path = self.root.join(MANIFEST);
        let temporary = self.root.join(format!(".{MANIFEST}.tmp"));
        fs::write(&temporary, self.document.to_string())
            .and_then(|()| fs::rename(&temporary, &path))
            .map_err(|e| {
                let _ = fs::remove_file(&temporary);
                Error::Write(path, e)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_without_review_toml_opens_and_is_named_after_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("egusphere-2026-1234");
        fs::create_dir(&root).unwrap();
        let project = Project::open(&root).unwrap();
        assert_eq!(project.manifest, Manifest::default());
        assert_eq!(project.name(), "egusphere-2026-1234");
        assert!(!root.join(MANIFEST).exists());
    }

    #[test]
    fn review_toml_is_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(MANIFEST),
            "title = \"Overshoot thresholds\"\njournal = \"The Cryosphere\"\n\
             due = 2026-11-14\ncolour = \"orange\"\nlinked = [\"~/guidelines.pdf\"]\n",
        )
        .unwrap();
        let project = Project::open(dir.path()).unwrap();
        assert_eq!(project.name(), "Overshoot thresholds");
        assert_eq!(project.manifest.journal.as_deref(), Some("The Cryosphere"));
        assert_eq!(project.manifest.due.unwrap().to_string(), "2026-11-14");
        assert_eq!(project.manifest.colour, Some(Colour::Orange));
        assert_eq!(project.manifest.linked, [PathBuf::from("~/guidelines.pdf")]);
    }

    #[test]
    fn the_colour_is_written_and_the_rest_of_the_file_kept() {
        let dir = tempfile::tempdir().unwrap();
        let text = "# my notes\ntitle = \"T\"  # the short title\nreviewer = 2\ncolour = \"blue\"\n";
        fs::write(dir.path().join(MANIFEST), text).unwrap();
        let mut project = Project::open(dir.path()).unwrap();
        project.set_colour(Colour::Pink).unwrap();
        let written = fs::read_to_string(dir.path().join(MANIFEST)).unwrap();
        assert_eq!(written, text.replace("\"blue\"", "\"pink\""));
        assert_eq!(Project::open(dir.path()).unwrap().manifest.colour, Some(Colour::Pink));
        assert!(!dir.path().join(".review.toml.tmp").exists());
    }

    #[test]
    fn a_first_colour_makes_review_toml() {
        let dir = tempfile::tempdir().unwrap();
        let mut project = Project::open(dir.path()).unwrap();
        project.set_colour(Colour::Teal).unwrap();
        let written = fs::read_to_string(dir.path().join(MANIFEST)).unwrap();
        assert_eq!(written, "colour = \"teal\"\n");
    }

    #[test]
    fn a_broken_review_toml_is_an_error_that_names_it() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MANIFEST), "title = \n").unwrap();
        let error = Project::open(dir.path()).unwrap_err().to_string();
        assert!(error.contains("review.toml"), "{error}");
        fs::write(dir.path().join(MANIFEST), "colour = \"mauve\"\n").unwrap();
        assert!(matches!(Project::open(dir.path()), Err(Error::Manifest(..))));
    }

    #[test]
    fn a_file_is_not_a_project() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("paper.pdf");
        fs::write(&file, "").unwrap();
        assert!(matches!(Project::open(&file), Err(Error::NotAFolder(_))));
    }

    #[test]
    fn linked_files_are_from_the_folder_or_home() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MANIFEST), "linked = [\"../elsewhere/a.pdf\"]\n").unwrap();
        let project = Project::open(dir.path()).unwrap();
        let linked = project.linked();
        assert_eq!(linked[0].path, dir.path().join("../elsewhere/a.pdf"));
        assert!(linked[0].missing);
    }
}
