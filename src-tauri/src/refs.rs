//! The references of a project: the `.bib` files of the project (its folder and the files linked), and the
//! `.bib` of the papers library (a `.papers/config.json` in the folder or above it, else papers' default
//! library), loaded as one library (papers-core) and searched as Papers Desktop searches its own. Loaded when
//! first asked, kept while the project is open, and read again when one of the files changes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use papers_core::{BibWatch, Found, Library, Matching};
use papers_rs::config::{MachineConfig, find_config};
use tauri::{AppHandle, Emitter, EventTarget, Manager};

use crate::{AppState, pane, windows};

/// The references of each project open, by its folder. A project's slot is made when its references are first
/// asked and loaded under its own lock, so a second search waits for the load instead of repeating it.
pub type Loaded = HashMap<PathBuf, Arc<Mutex<Option<Refs>>>>;

/// The references of a project, as they were read.
pub struct Refs {
    /// the `.bib` files read
    sources: Vec<PathBuf>,
    /// the library of the files, or what went wrong reading them (a file that does not parse): the page shows
    /// it, and the files are watched all the same, for the one that is mended
    library: Result<Arc<Library>, String>,
    _watches: Vec<BibWatch>,
}

impl Refs {
    fn new(app: &AppHandle, root: &Path, sources: Vec<PathBuf>) -> Refs {
        let library = Library::load(&sources).map(Arc::new).map_err(|e| e.to_string());
        let watches = sources
            .iter()
            .filter_map(|bib| {
                let (handle, project) = (app.clone(), root.to_path_buf());
                BibWatch::new(bib.clone(), move || reload(&handle, &project))
                    .map_err(|e| eprintln!("{}: not watched ({e})", bib.display()))
                    .ok()
            })
            .collect();
        Refs {
            sources,
            library,
            _watches: watches,
        }
    }
}

/// The matches of `query` in the library: words, field filters, a citation key; a word matches from the start
/// of a word of the entry. The first `limit` come back, with their marks. With nothing typed, the entries,
/// newest first, to look through.
pub fn search(library: &Library, query: &str, limit: usize) -> Found {
    if !query.trim().is_empty() {
        return library.search(query, Matching::WordStart, limit);
    }
    let ids = library.matching("", Matching::WordStart);
    Found {
        total: ids.len(),
        hits: ids.iter().take(limit).filter_map(|id| library.hit(id)).collect(),
        ..Found::default()
    }
}

/// The slot of the references of the project of `root`.
fn slot(app: &AppHandle, root: &Path) -> Arc<Mutex<Option<Refs>>> {
    let state = app.state::<AppState>();
    let mut loaded = state.references.lock().unwrap();
    loaded.entry(root.to_path_buf()).or_default().clone()
}

/// `read` given the references of the project of `root`, loaded if they are not.
fn with<T>(app: &AppHandle, root: &Path, read: impl FnOnce(&Refs) -> T) -> T {
    let slot = slot(app, root);
    let mut refs = slot.lock().unwrap();
    read(refs.get_or_insert_with(|| Refs::new(app, root, discover(app, root))))
}

/// The library of the project of `root`.
pub fn library(app: &AppHandle, root: &Path) -> Result<Arc<Library>, String> {
    with(app, root, |refs| refs.library.clone())
}

/// The `.bib` files the references of the project of `root` are read from.
pub fn sources_of(app: &AppHandle, root: &Path) -> Vec<PathBuf> {
    with(app, root, |refs| refs.sources.clone())
}

/// The files of the project of `root` changed (a `.bib` came or went, or was linked): its references are read
/// again when the files they come from are other than before. Nothing when they were not loaded.
pub fn refresh(app: &AppHandle, root: &Path) {
    let loaded = app.state::<AppState>().references.lock().unwrap().get(root).cloned();
    let Some(slot) = loaded else {
        return;
    };
    let (handle, root) = (app.clone(), root.to_path_buf());
    // off the thread of the caller: the files are listed, and a library may be long to load
    std::thread::spawn(move || {
        let sources = discover(&handle, &root);
        let mut refs = slot.lock().unwrap();
        if refs.as_ref().is_some_and(|r| r.sources != sources) {
            *refs = Some(Refs::new(&handle, &root, sources));
            drop(refs);
            tell(&handle, &root);
        }
    });
}

/// A `.bib` file of the references changed: they are read again, and the pages told.
fn reload(app: &AppHandle, root: &Path) {
    let loaded = app.state::<AppState>().references.lock().unwrap().get(root).cloned();
    // none: the project was closed since
    let Some(slot) = loaded else {
        return;
    };
    {
        let mut refs = slot.lock().unwrap();
        if refs.is_none() {
            return;
        }
        *refs = Some(Refs::new(app, root, discover(app, root)));
    }
    tell(app, root);
}

/// The project of `root` closed: its references are dropped, and its files no longer watched.
pub fn unload(app: &AppHandle, root: &Path) {
    app.state::<AppState>().references.lock().unwrap().remove(root);
}

/// The pages and the pane of the project's window told that the references were read again.
fn tell(app: &AppHandle, root: &Path) {
    let panes = windows::of_project(app, root).into_iter().map(|w| pane::label(&w));
    for label in papers_tabs::pages_of_project(app, root).into_iter().chain(panes) {
        let _ = app.emit_to(EventTarget::webview(label), "references-changed", ());
    }
}

/// A PDF of a reference opened in a tab of the project of `root` (the PDF may lie outside it).
pub fn open_pdf(app: &AppHandle, root: &Path, path: &Path) -> Result<(), String> {
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) || !path.is_file() {
        return Err(format!("{}: not a PDF file", path.display()));
    }
    windows::open_file(app, root, path, None)
}

/// The `.bib` files of the project of `root`, and of its papers library.
fn discover(app: &AppHandle, root: &Path) -> Vec<PathBuf> {
    let project = app
        .state::<AppState>()
        .projects
        .lock()
        .unwrap()
        .get(root)
        .map(|p| p.bib_files())
        .unwrap_or_default();
    let library = MachineConfig::load(None)
        .map_err(|e| e.message().to_string())
        .and_then(|machine| library_bib(root, machine))
        // the project's own files are still searched when papers' configuration cannot be read
        .unwrap_or_else(|e| {
            eprintln!("the papers library: {e}");
            None
        });
    sources(&project, library)
}

/// The `.bib` file of the papers library for a folder: of the library whose `.papers/config.json` is in the
/// folder or above it, else of the default library of `machine`. None when there is none.
fn library_bib(folder: &Path, machine: MachineConfig) -> Result<Option<PathBuf>, String> {
    let config = find_config(&folder.to_string_lossy(), machine).map_err(|e| e.message().to_string())?;
    Ok(config.bibtex.filter(|b| !b.is_empty()).map(PathBuf::from))
}

/// The files to read: those of the project, then the library's, each once (a project in the library
/// folder lists its `.bib` too); one that is not there is left out.
fn sources(project: &[PathBuf], library: Option<PathBuf>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut real: Vec<PathBuf> = Vec::new();
    for bib in project.iter().chain(&library) {
        let Ok(canonical) = bib.canonicalize() else {
            continue;
        };
        if canonical.is_file() && !real.contains(&canonical) {
            real.push(canonical);
            found.push(bib.clone());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const REFS: &str = "\
@article{robinson2012,
  author = {Robinson, Alexander and Calov, Reinhard and Ganopolski, Andrey},
  title = {Multistability and critical thresholds of the Greenland ice sheet},
  journal = {Nature Climate Change},
  year = {2012},
}

@article{feldmann2017,
  author = {Feldmann, Johannes and Levermann, Anders},
  title = {From cyclic ice streaming to Heinrich-like events},
  journal = {The Cryosphere},
  year = {2017},
}
";

    /// An empty folder of its own for a test.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("papers-review-refs-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn bib(folder: &Path, name: &str, text: &str) -> PathBuf {
        let path = folder.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn keys(found: &Found) -> Vec<&str> {
        found.hits.iter().map(|h| h.item.key.as_str()).collect()
    }

    #[test]
    fn a_papers_library_in_the_folder_or_above_gives_its_bib() {
        let dir = folder("library");
        let library = dir.join("library");
        fs::create_dir_all(library.join(".papers")).unwrap();
        fs::create_dir_all(library.join("reviews/one")).unwrap();
        fs::write(library.join(".papers/config.json"), r#"{"bibtex": "papers.bib", "version": 3}"#).unwrap();
        let papers = bib(&library, "papers.bib", REFS);
        for folder in [library.clone(), library.join("reviews/one")] {
            let found = library_bib(&folder, MachineConfig::empty()).unwrap();
            assert_eq!(found.unwrap().canonicalize().unwrap(), papers.canonicalize().unwrap());
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_papers_configuration_that_cannot_be_read_is_an_error() {
        let dir = folder("unreadable");
        fs::create_dir_all(dir.join(".papers")).unwrap();
        fs::write(dir.join(".papers/config.json"), "{bad").unwrap();
        assert!(library_bib(&dir, MachineConfig::empty()).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sources_are_the_projects_bib_files_then_the_librarys_each_once_and_all_there() {
        let dir = folder("sources");
        let (a, b) = (bib(&dir, "a.bib", ""), bib(&dir, "b.bib", ""));
        let gone = dir.join("gone.bib");
        let folder = dir.join("folder.bib");
        fs::create_dir(&folder).unwrap();
        let project = [a.clone(), gone.clone(), folder, b.clone(), a.clone()];
        assert_eq!(sources(&project, None), [a.clone(), b.clone()]);
        // the library's file, unless the project has it
        let library = bib(&dir, "library.bib", "");
        assert_eq!(sources(&project, Some(library.clone())), [a.clone(), b.clone(), library.clone()]);
        assert_eq!(sources(&[library.clone(), a.clone()], Some(library.clone())), [library, a]);
        assert_eq!(sources(&project, Some(gone)), [project[0].clone(), b]);
        assert!(sources(&[], None).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_references_are_searched_by_the_start_of_a_word() {
        let dir = folder("search");
        let library = Library::load(&[bib(&dir, "refs.bib", REFS)]).unwrap();
        assert_eq!(keys(&search(&library, "robin", 10)), ["robinson2012"]);
        assert_eq!(keys(&search(&library, "ice", 10)), ["feldmann2017", "robinson2012"]);
        assert_eq!(search(&library, "ice", 1).total, 2);
        assert_eq!(search(&library, "ice", 1).hits.len(), 1);
        // not inside a word
        assert_eq!(search(&library, "stability", 10).total, 0);
        assert_eq!(keys(&search(&library, "year:2017", 10)), ["feldmann2017"]);
        // nothing typed: all of them, newest first
        assert_eq!(keys(&search(&library, " ", 10)), ["feldmann2017", "robinson2012"]);
        assert_eq!(search(&library, "", 1).total, 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_found_reads_as_the_page_expects() {
        let dir = folder("json");
        let library = Library::load(&[bib(&dir, "refs.bib", REFS)]).unwrap();
        let found = serde_json::to_value(search(&library, "robin", 10)).unwrap();
        let hit = &found["hits"][0];
        assert_eq!(found["total"], 1);
        let mut item: Vec<&str> = hit["item"].as_object().unwrap().keys().map(String::as_str).collect();
        item.sort();
        assert_eq!(
            item,
            ["authors", "bib", "container", "doi", "files", "id", "key", "keywords", "kind", "server", "subtitle", "title", "url", "year"]
        );
        assert_eq!(hit["item"]["year"], 2012);
        assert_eq!(hit["item"]["doi"], serde_json::Value::Null);
        assert_eq!(hit["authors"], serde_json::json!([true, false, false]));
        assert_eq!(hit["title"][0]["mark"], false);
        assert!(hit["title"].as_array().unwrap().iter().all(|s| s["text"].is_string() && s["mark"].is_boolean()));
        assert_eq!(found["years"][0], serde_json::json!({"year": 2012, "count": 1, "selected": true}));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_reference_closest_to_a_text_is_found() {
        let dir = folder("closest");
        let library = Library::load(&[bib(&dir, "refs.bib", REFS)]).unwrap();
        let text = "Robinson, A., Calov, R. and Ganopolski, A.: Multistability and critical thresholds of the \
                    Greenland ice sheet, Nat. Clim. Change, 2012.";
        let found = library.closest(text, 5);
        assert_eq!(keys(&found), ["robinson2012"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bib_file_that_does_not_parse_is_an_error_naming_it() {
        let dir = folder("broken");
        let broken = bib(&dir, "broken.bib", "@article{robinson2012, title = {open");
        let error = Library::load(std::slice::from_ref(&broken)).err().unwrap().to_string();
        assert!(error.contains("broken.bib"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }
}
