//! A project: a folder, and what `review.toml` in it adds (the title, the
//! journal, the due date, the colour, the files linked from elsewhere). The
//! file is read as it is; a change writes only its own key, so the comments
//! and the keys the app does not know stay.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::{Array, Datetime, DocumentMut, value};

use crate::colour::Colour;
use crate::files::{self, Node};

/// The file of the app in a project folder.
pub const MANIFEST: &str = "review.toml";

/// The review, when `review.toml` names none.
pub const REVIEW: &str = "review.md";

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
    #[error("{0}: not in the project folder")]
    Outside(PathBuf),
    #[error("{0}: already there")]
    Exists(PathBuf),
    #[error("{0}: not a name for a file")]
    Name(String),
    #[error("{0}: a folder cannot be copied into itself")]
    Itself(PathBuf),
}

/// What `review.toml` says; all of it optional.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub title: Option<String>,
    /// a few letters for the project, where the name is too long (the project bar)
    pub short: Option<String>,
    pub journal: Option<String>,
    /// the manuscript's identifier, as the journal gives it
    pub manuscript: Option<String>,
    pub due: Option<Datetime>,
    /// none yet: the app gives one when the project first opens
    pub colour: Option<Colour>,
    /// files elsewhere, part of the project
    #[serde(default)]
    pub linked: Vec<PathBuf>,
    /// the Markdown file of the review (`review.md` without)
    pub review: Option<PathBuf>,
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

    /// The title (none, or empty: the folder names the project), `review.toml` written.
    pub fn set_title(&mut self, title: Option<String>) -> Result<(), Error> {
        let title = clean(title);
        if title != self.manifest.title {
            self.write_text("title", title.as_deref())?;
            self.manifest.title = title;
        }
        Ok(())
    }

    /// The short name, `review.toml` written.
    pub fn set_short(&mut self, short: Option<String>) -> Result<(), Error> {
        let short = clean(short);
        if short != self.manifest.short {
            self.write_text("short", short.as_deref())?;
            self.manifest.short = short;
        }
        Ok(())
    }

    /// The journal, `review.toml` written.
    pub fn set_journal(&mut self, journal: Option<String>) -> Result<(), Error> {
        let journal = clean(journal);
        if journal != self.manifest.journal {
            self.write_text("journal", journal.as_deref())?;
            self.manifest.journal = journal;
        }
        Ok(())
    }

    /// The manuscript's identifier, `review.toml` written.
    pub fn set_manuscript(&mut self, manuscript: Option<String>) -> Result<(), Error> {
        let manuscript = clean(manuscript);
        if manuscript != self.manifest.manuscript {
            self.write_text("manuscript", manuscript.as_deref())?;
            self.manifest.manuscript = manuscript;
        }
        Ok(())
    }

    /// The due date, `review.toml` written.
    pub fn set_due(&mut self, due: Option<Datetime>) -> Result<(), Error> {
        if due != self.manifest.due {
            match due {
                Some(due) => self.document["due"] = value(due),
                None => {
                    self.document.remove("due");
                }
            }
            self.save()?;
            self.manifest.due = due;
        }
        Ok(())
    }

    /// A key of `review.toml` set to `text`, or removed when there is none.
    fn write_text(&mut self, key: &str, text: Option<&str>) -> Result<(), Error> {
        match text {
            Some(text) => self.document[key] = value(text),
            None => {
                self.document.remove(key);
            }
        }
        self.save()
    }

    /// Files from elsewhere linked to the project, after those linked already; a file in the folder, or
    /// linked already, is left out.
    pub fn link(&mut self, paths: &[PathBuf]) -> Result<(), Error> {
        let mut added: Vec<PathBuf> = Vec::new();
        for path in paths {
            if !self.within(path) && !self.is_linked(path) && !added.contains(path) {
                added.push(path.clone());
            }
        }
        if added.is_empty() {
            return Ok(());
        }
        let mut linked = self.manifest.linked.clone();
        linked.extend(added);
        self.write_linked(linked)
    }

    /// A file no longer linked, by its path as `linked` gives it or as `review.toml` has it.
    pub fn unlink(&mut self, path: &Path) -> Result<(), Error> {
        let kept: Vec<PathBuf> = self
            .manifest
            .linked
            .iter()
            .filter(|l| l.as_path() != path && self.resolve(l) != path)
            .cloned()
            .collect();
        if kept.len() == self.manifest.linked.len() {
            return Ok(());
        }
        self.write_linked(kept)
    }

    /// `linked` of `review.toml` set to `linked`, the key removed when none is left.
    fn write_linked(&mut self, linked: Vec<PathBuf>) -> Result<(), Error> {
        if linked.is_empty() {
            self.document.remove("linked");
        } else {
            let paths: Array = linked.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            self.document["linked"] = value(paths);
        }
        self.save()?;
        self.manifest.linked = linked;
        Ok(())
    }

    fn is_linked(&self, path: &Path) -> bool {
        self.manifest.linked.iter().any(|l| self.resolve(l) == path)
    }

    /// Files and folders (with all they hold) copied into `folder` of the project. Nothing is copied
    /// when one of the names is there already.
    pub fn copy_in(&self, paths: &[PathBuf], folder: &Path) -> Result<(), Error> {
        self.check_folder(folder)?;
        let mut targets: Vec<PathBuf> = Vec::new();
        for path in paths {
            let name = path.file_name().ok_or_else(|| Error::Name(path.display().to_string()))?;
            let target = folder.join(name);
            if target.symlink_metadata().is_ok() || targets.contains(&target) {
                return Err(Error::Exists(target));
            }
            if path.is_dir() && canonical(folder).zip(canonical(path)).is_some_and(|(f, p)| f.starts_with(p)) {
                return Err(Error::Itself(path.clone()));
            }
            targets.push(target);
        }
        for (path, target) in paths.iter().zip(&targets) {
            files::copy(path, target).map_err(|e| Error::Write(target.clone(), e))?;
        }
        Ok(())
    }

    /// An empty file `name` in `folder` of the project.
    pub fn new_file(&self, folder: &Path, name: &str) -> Result<PathBuf, Error> {
        if name.is_empty() || name == "." || name == ".." || name.contains(std::path::is_separator) {
            return Err(Error::Name(name.to_string()));
        }
        self.check_folder(folder)?;
        let path = folder.join(name);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| match e.kind() {
                io::ErrorKind::AlreadyExists => Error::Exists(path.clone()),
                _ => Error::Write(path.clone(), e),
            })?;
        Ok(path)
    }

    /// `folder` is a folder of the project.
    fn check_folder(&self, folder: &Path) -> Result<(), Error> {
        if !folder.is_dir() {
            return Err(Error::NotAFolder(folder.to_path_buf()));
        }
        if !self.within(folder) {
            return Err(Error::Outside(folder.to_path_buf()));
        }
        Ok(())
    }

    /// The Markdown files of the project: those of the folder, then the linked ones (and those in linked
    /// folders).
    pub fn markdown_files(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        files::markdown(&self.files(), &mut found);
        for node in self.linked() {
            if node.kind == files::Kind::Folder {
                files::markdown(&files::tree(&node.path), &mut found);
            } else {
                files::markdown(&[node], &mut found);
            }
        }
        found
    }

    /// Whether a file is the project's, to be read and written: in the folder (`review.toml` apart, the
    /// app's own), the review, a linked file or in a linked folder. Links and `..` are resolved first.
    pub fn contains(&self, path: &Path) -> bool {
        let Some(path) = canonical(path) else {
            return false;
        };
        if let Some(root) = canonical(&self.root)
            && path.starts_with(&root)
        {
            return path != root.join(MANIFEST);
        }
        if canonical(&self.review_path()).is_some_and(|review| review == path) {
            return true;
        }
        self.manifest
            .linked
            .iter()
            .filter_map(|l| canonical(&self.resolve(l)))
            .any(|l| path == l || (l.is_dir() && path.starts_with(&l)))
    }

    /// Whether a path is in the folder of the project, links and `..` resolved.
    fn within(&self, path: &Path) -> bool {
        canonical(path)
            .zip(canonical(&self.root))
            .is_some_and(|(path, root)| path.starts_with(root))
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

    /// The file of the review: `review` of `review.toml`, else `review.md`
    /// in the folder.
    pub fn review_path(&self) -> PathBuf {
        self.resolve(self.manifest.review.as_deref().unwrap_or(Path::new(REVIEW)))
    }

    /// The text of the review; empty while it is not written yet.
    pub fn read_review(&self) -> Result<String, Error> {
        let path = self.review_path();
        match fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(Error::Read(path, e)),
        }
    }

    /// The review written, atomically.
    pub fn write_review(&self, text: &str) -> Result<(), Error> {
        write_atomically(&self.review_path(), text)
    }

    /// Another Markdown file as the review, in `review.toml`: by its path
    /// from the folder when it is in it.
    pub fn set_review(&mut self, path: &Path) -> Result<(), Error> {
        let kept = path.strip_prefix(&self.root).unwrap_or(path).to_path_buf();
        self.document["review"] = value(kept.to_string_lossy().as_ref());
        self.save()?;
        self.manifest.review = Some(kept);
        Ok(())
    }

    /// `review.toml` written whole, atomically.
    fn save(&self) -> Result<(), Error> {
        write_atomically(&self.root.join(MANIFEST), &self.document.to_string())
    }
}

/// A text without the spaces round it; none when nothing is left.
fn clean(text: Option<String>) -> Option<String> {
    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

/// A path with its links and `..` resolved; a file not there yet, by its folder.
fn canonical(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = path.canonicalize() {
        return Some(path);
    }
    let name = path.file_name()?;
    Some(path.parent()?.canonicalize().ok()?.join(name))
}

/// A file written whole, atomically: a hidden file beside it, renamed.
pub fn write_atomically(path: &Path, text: &str) -> Result<(), Error> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let temporary = path.with_file_name(format!(".{name}.tmp"));
    fs::write(&temporary, text)
        .and_then(|()| fs::rename(&temporary, path))
        .map_err(|e| {
            let _ = fs::remove_file(&temporary);
            Error::Write(path.to_path_buf(), e)
        })
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
    fn the_review_is_review_md_until_another_file_is_chosen() {
        let dir = tempfile::tempdir().unwrap();
        let mut project = Project::open(dir.path()).unwrap();
        assert_eq!(project.review_path(), dir.path().join(REVIEW));
        // not written yet: empty
        assert_eq!(project.read_review().unwrap(), "");
        project.write_review("## Round 1\n").unwrap();
        assert_eq!(fs::read_to_string(dir.path().join(REVIEW)).unwrap(), "## Round 1\n");
        assert!(!dir.path().join(".review.md.tmp").exists());

        fs::create_dir(dir.path().join("round-2")).unwrap();
        let notes = dir.path().join("round-2/notes.md");
        fs::write(&notes, "notes").unwrap();
        project.set_review(&notes).unwrap();
        assert_eq!(project.read_review().unwrap(), "notes");
        // kept from the folder, read again so
        let written = fs::read_to_string(dir.path().join(MANIFEST)).unwrap();
        assert_eq!(written, "review = \"round-2/notes.md\"\n");
        assert_eq!(Project::open(dir.path()).unwrap().review_path(), notes);
    }

    #[test]
    fn a_review_elsewhere_is_kept_by_its_whole_path() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let notes = elsewhere.path().join("notes.md");
        let mut project = Project::open(dir.path()).unwrap();
        project.set_review(&notes).unwrap();
        assert_eq!(project.review_path(), notes);
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

    #[test]
    fn the_title_short_journal_manuscript_and_due_date_are_written_key_by_key() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MANIFEST), "# my notes\ntitle = \"T\"  # the short title\ncolour = \"blue\"\n").unwrap();
        let mut project = Project::open(dir.path()).unwrap();
        project.set_title(Some("Overshoot thresholds".into())).unwrap();
        project.set_short(Some("OT".into())).unwrap();
        project.set_journal(Some("The Cryosphere".into())).unwrap();
        project.set_manuscript(Some("egusphere-2026-1234".into())).unwrap();
        project.set_due(Some("2026-11-14".parse().unwrap())).unwrap();
        let written = fs::read_to_string(dir.path().join(MANIFEST)).unwrap();
        assert!(written.starts_with("# my notes\ntitle = \"Overshoot thresholds\""), "{written}");
        assert!(written.contains("colour = \"blue\""), "{written}");
        let reopened = Project::open(dir.path()).unwrap();
        assert_eq!(reopened.manifest, project.manifest);
        assert_eq!(reopened.manifest.short.as_deref(), Some("OT"));
        assert_eq!(reopened.manifest.due.unwrap().to_string(), "2026-11-14");
        assert_eq!(reopened.name(), "Overshoot thresholds");
    }

    #[test]
    fn nothing_or_an_empty_text_removes_the_key() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(MANIFEST),
            "title = \"T\"\nshort = \"T\"\njournal = \"J\"\nmanuscript = \"M\"\ndue = 2026-11-14\n",
        )
        .unwrap();
        let mut project = Project::open(dir.path()).unwrap();
        project.set_title(Some("  ".into())).unwrap();
        project.set_short(None).unwrap();
        project.set_journal(Some(String::new())).unwrap();
        project.set_manuscript(None).unwrap();
        project.set_due(None).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join(MANIFEST)).unwrap(), "");
        assert_eq!(project.manifest, Manifest::default());
        assert_eq!(project.name(), dir.path().file_name().unwrap().to_string_lossy());
    }

    #[test]
    fn a_value_unchanged_is_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let mut project = Project::open(dir.path()).unwrap();
        project.set_journal(None).unwrap();
        project.set_due(None).unwrap();
        assert!(!dir.path().join(MANIFEST).exists());
    }

    /// A project folder and a folder elsewhere, with a few files.
    fn two_folders() -> (tempfile::TempDir, tempfile::TempDir) {
        let (dir, elsewhere) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fs::create_dir(dir.path().join("round-1")).unwrap();
        fs::write(dir.path().join("round-1/manuscript.pdf"), "pdf").unwrap();
        fs::write(elsewhere.path().join("guidelines.pdf"), "pdf").unwrap();
        fs::write(elsewhere.path().join("notes.md"), "notes").unwrap();
        fs::create_dir(elsewhere.path().join("code")).unwrap();
        fs::write(elsewhere.path().join("code/README.md"), "readme").unwrap();
        fs::write(elsewhere.path().join("code/run.jl"), "").unwrap();
        (dir, elsewhere)
    }

    #[test]
    fn files_are_linked_after_those_linked_and_unlinked() {
        let (dir, elsewhere) = two_folders();
        fs::write(dir.path().join(MANIFEST), "title = \"T\"  # kept\nlinked = [\"~/a.pdf\"]\n").unwrap();
        let mut project = Project::open(dir.path()).unwrap();
        let (guidelines, notes) = (elsewhere.path().join("guidelines.pdf"), elsewhere.path().join("notes.md"));
        project.link(&[guidelines.clone(), notes.clone()]).unwrap();
        let reopened = Project::open(dir.path()).unwrap();
        assert_eq!(reopened.manifest.linked, [PathBuf::from("~/a.pdf"), guidelines.clone(), notes.clone()]);
        assert!(fs::read_to_string(dir.path().join(MANIFEST)).unwrap().starts_with("title = \"T\"  # kept\n"));

        project.unlink(&guidelines).unwrap();
        assert_eq!(Project::open(dir.path()).unwrap().manifest.linked, [PathBuf::from("~/a.pdf"), notes]);
        // as review.toml has it
        project.unlink(Path::new("~/a.pdf")).unwrap();
        project.unlink(&elsewhere.path().join("notes.md")).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join(MANIFEST)).unwrap(), "title = \"T\"  # kept\n");
        assert!(project.manifest.linked.is_empty());
    }

    #[test]
    fn a_file_linked_already_or_in_the_folder_is_not_linked_again() {
        let (dir, elsewhere) = two_folders();
        let mut project = Project::open(dir.path()).unwrap();
        let guidelines = elsewhere.path().join("guidelines.pdf");
        project.link(&[guidelines.clone(), guidelines.clone()]).unwrap();
        project.link(&[guidelines.clone(), dir.path().join("round-1/manuscript.pdf")]).unwrap();
        assert_eq!(project.manifest.linked, [guidelines]);
        // nothing to add: nothing written
        let other = tempfile::tempdir().unwrap();
        let mut bare = Project::open(other.path()).unwrap();
        bare.link(&[other.path().join("x.pdf")]).unwrap();
        bare.unlink(Path::new("/nowhere")).unwrap();
        assert!(!other.path().join(MANIFEST).exists());
    }

    #[test]
    fn files_and_folders_are_copied_in() {
        let (dir, elsewhere) = two_folders();
        let project = Project::open(dir.path()).unwrap();
        let folder = dir.path().join("round-1");
        let paths = [elsewhere.path().join("guidelines.pdf"), elsewhere.path().join("code")];
        project.copy_in(&paths, &folder).unwrap();
        assert_eq!(fs::read_to_string(folder.join("guidelines.pdf")).unwrap(), "pdf");
        assert_eq!(fs::read_to_string(folder.join("code/README.md")).unwrap(), "readme");
        assert!(folder.join("code/run.jl").is_file());
        assert!(elsewhere.path().join("code/run.jl").is_file());
    }

    #[test]
    fn a_name_already_there_is_not_overwritten_and_nothing_is_copied() {
        let (dir, elsewhere) = two_folders();
        let project = Project::open(dir.path()).unwrap();
        let folder = dir.path().join("round-1");
        fs::write(folder.join("notes.md"), "mine").unwrap();
        let paths = [elsewhere.path().join("guidelines.pdf"), elsewhere.path().join("notes.md")];
        let error = project.copy_in(&paths, &folder).unwrap_err();
        assert!(matches!(error, Error::Exists(_)));
        assert!(error.to_string().contains("notes.md"), "{error}");
        assert_eq!(fs::read_to_string(folder.join("notes.md")).unwrap(), "mine");
        assert!(!folder.join("guidelines.pdf").exists());
    }

    #[test]
    fn files_are_copied_into_a_folder_of_the_project_only_and_a_folder_not_into_itself() {
        let (dir, elsewhere) = two_folders();
        let project = Project::open(dir.path()).unwrap();
        let paths = [elsewhere.path().join("notes.md")];
        assert!(matches!(project.copy_in(&paths, elsewhere.path()), Err(Error::Outside(_))));
        let dodging = dir.path().join("round-1/../../");
        assert!(matches!(project.copy_in(&paths, &dodging), Err(Error::Outside(_))));
        assert!(matches!(project.copy_in(&paths, &dir.path().join("nowhere")), Err(Error::NotAFolder(_))));
        let root = [dir.path().to_path_buf()];
        let error = project.copy_in(&root, &dir.path().join("round-1")).unwrap_err();
        assert!(matches!(error, Error::Itself(_)), "{error}");
    }

    #[test]
    fn a_new_file_is_empty_and_never_replaces_one() {
        let (dir, _elsewhere) = two_folders();
        let project = Project::open(dir.path()).unwrap();
        let folder = dir.path().join("round-1");
        let path = project.new_file(&folder, "notes.md").unwrap();
        assert_eq!(path, folder.join("notes.md"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "");
        fs::write(&path, "typed").unwrap();
        assert!(matches!(project.new_file(&folder, "notes.md"), Err(Error::Exists(_))));
        assert_eq!(fs::read_to_string(&path).unwrap(), "typed");
    }

    #[test]
    fn a_new_file_has_a_plain_name_in_a_folder_of_the_project() {
        let (dir, elsewhere) = two_folders();
        let project = Project::open(dir.path()).unwrap();
        let folder = dir.path().join("round-1");
        for name in ["", ".", "..", "a/b.md", "../b.md"] {
            assert!(matches!(project.new_file(&folder, name), Err(Error::Name(_))), "{name}");
        }
        assert!(matches!(project.new_file(elsewhere.path(), "a.md"), Err(Error::Outside(_))));
        assert!(!elsewhere.path().join("a.md").exists());
        assert!(matches!(project.new_file(&dir.path().join("none"), "a.md"), Err(Error::NotAFolder(_))));
    }

    #[test]
    fn the_markdown_files_are_those_of_the_folder_and_the_linked_ones() {
        let (dir, elsewhere) = two_folders();
        fs::write(dir.path().join("review.md"), "").unwrap();
        fs::write(dir.path().join("round-1/notes.markdown"), "").unwrap();
        fs::write(
            dir.path().join(MANIFEST),
            format!(
                "linked = [{:?}, {:?}, {:?}, {:?}]\n",
                elsewhere.path().join("notes.md"),
                elsewhere.path().join("code"),
                elsewhere.path().join("guidelines.pdf"),
                elsewhere.path().join("gone.md"),
            ),
        )
        .unwrap();
        let project = Project::open(dir.path()).unwrap();
        assert_eq!(
            project.markdown_files(),
            [
                dir.path().join("round-1/notes.markdown"),
                dir.path().join("review.md"),
                elsewhere.path().join("notes.md"),
                elsewhere.path().join("code/README.md"),
            ]
        );
    }

    #[test]
    fn the_project_contains_its_folder_the_review_and_what_is_linked() {
        let (dir, elsewhere) = two_folders();
        let other = tempfile::tempdir().unwrap();
        fs::write(other.path().join("secret.txt"), "").unwrap();
        fs::write(
            dir.path().join(MANIFEST),
            format!(
                "review = {:?}\nlinked = [{:?}, {:?}]\n",
                elsewhere.path().join("notes.md"),
                elsewhere.path().join("guidelines.pdf"),
                elsewhere.path().join("code"),
            ),
        )
        .unwrap();
        let project = Project::open(dir.path()).unwrap();
        let root = dir.path();
        assert!(project.contains(&root.join("round-1/manuscript.pdf")));
        // not there yet
        assert!(project.contains(&root.join("round-1/new.md")));
        assert!(project.contains(&root.join("review.md")));
        assert!(project.contains(&elsewhere.path().join("notes.md")));
        assert!(project.contains(&elsewhere.path().join("guidelines.pdf")));
        assert!(project.contains(&elsewhere.path().join("code/README.md")));
        assert!(project.contains(&elsewhere.path().join("code/new.md")));

        assert!(!project.contains(&root.join(MANIFEST)));
        assert!(!project.contains(&other.path().join("secret.txt")));
        assert!(!project.contains(&elsewhere.path().join("guidelines.pdf.bak")));
        assert!(!project.contains(&root.join("round-1/../../secret.txt")));
        assert!(!project.contains(&root.join("nowhere/new.md")));
        assert!(!project.contains(Path::new("")));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_file_elsewhere_does_not_make_the_file_it_points_to_the_projects() {
        let (dir, elsewhere) = two_folders();
        std::os::unix::fs::symlink(elsewhere.path().join("notes.md"), dir.path().join("link.md")).unwrap();
        let project = Project::open(dir.path()).unwrap();
        assert!(!project.contains(&dir.path().join("link.md")));
    }
}
