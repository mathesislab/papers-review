//! A new review from a manuscript: the PDF is read as papers reads one (the DOI in its metadata, else in the text
//! of its first pages, and the paper of that DOI found on Crossref with its title in the text), and the review's
//! folder is made in the reviews folder of the Settings, with the PDF copied in. The title, the journal and the
//! manuscript ID read go to the new project's card. What cannot be read, or fetched, is left empty.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use papers_rs::extract::doi::{parse_dois, title_in_text};
use papers_rs::extract::pdf::Pdf;
use papers_rs::fetch::http::Network;
use papers_rs::fetch::{ErrorKind, Fetcher};
use serde_json::Value;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::settings::Settings;
use papers_tabs::Layout;
use crate::{AppState, windows};

/// The most DOIs of the text asked of Crossref.
const CANDIDATES: usize = 3;

/// What is read of a manuscript.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Manuscript {
    pub doi: Option<String>,
    pub title: Option<String>,
    pub journal: Option<String>,
    /// the manuscript's identifier, as a Copernicus journal gives it (`egusphere-2026-6269`)
    pub id: Option<String>,
}

/// The manuscript in `pdf`; the Crossref records are kept in `cache`.
pub fn read(pdf: &Path, cache: &Path) -> Manuscript {
    let agent = format!("Papers Review/{}", env!("CARGO_PKG_VERSION"));
    let fetcher = Network::with_user_agent(&agent)
        .map(|network| Fetcher::with_transport(Box::new(network), &cache.to_string_lossy()))
        .inspect_err(|e| eprintln!("{e}"))
        .ok();
    read_with(pdf, fetcher.as_ref())
}

/// The manuscript in `pdf`, asking `fetcher` for the record of its DOI; none: no network.
fn read_with(path: &Path, fetcher: Option<&Fetcher>) -> Manuscript {
    let Ok(pdf) = Pdf::open(&path.to_string_lossy()) else {
        return Manuscript::default();
    };
    let (doi, record) = match find_doi(&pdf, fetcher) {
        Some((doi, record)) => (Some(doi), record),
        None => (None, None),
    };
    let id = doi.as_deref().and_then(id_of_doi).or_else(|| id_in(&pdf.page_text(0)));
    let (title, journal) = record.as_ref().map(|r| (title_of(r), journal_of(r, id.as_deref()))).unwrap_or_default();
    let title = title.or_else(|| info_title(&pdf));
    Manuscript { doi, title, journal, id }
}

/// The DOI of the PDF and its Crossref record, if it could be had: the DOI of the metadata, trusted; else the first
/// DOI of the text whose title, from Crossref, is in the text (a DOI cited there is the paper of another).
fn find_doi(pdf: &Pdf, fetcher: Option<&Fetcher>) -> Option<(String, Option<Value>)> {
    if let Some(doi) = pdf.metadata_doi() {
        let record = fetcher.and_then(|f| f.fetch_doi_metadata(&doi).ok());
        return Some((doi, record));
    }
    let head = pdf.head(10, 200);
    let dois = parse_dois(&head, &pdf.tail()).unwrap_or_default();
    let fetcher = fetcher?;
    for doi in dois.into_iter().take(CANDIDATES) {
        match fetcher.fetch_doi_metadata(&doi) {
            Ok(record) if title_of(&record).is_some_and(|t| title_in_text(&t, &head)) => return Some((doi, Some(record))),
            // no network: the other DOIs would fail the same
            Err(e) if e.kind == ErrorKind::Network => return None,
            _ => {}
        }
    }
    None
}

/// The title in the document information of the PDF, unless it is one that a program gives to any document.
fn info_title(pdf: &Pdf) -> Option<String> {
    let title = pdf.info_field("Title").trim().to_string();
    let lower = title.to_lowercase();
    (!title.is_empty() && lower != "untitled" && !lower.starts_with("microsoft word - ")).then_some(title)
}

/// The title of a Crossref record, without markup.
fn title_of(record: &Value) -> Option<String> {
    let title = first(record, "title")?;
    let mut plain = String::new();
    let mut tag = false;
    for c in title.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if !tag => plain.push(c),
            _ => {}
        }
    }
    Some(plain.replace("&amp;", "&").split_whitespace().collect::<Vec<_>>().join(" ")).filter(|t| !t.is_empty())
}

/// The journal of a Crossref record (a Copernicus journal's discussions are its journal's); EGUsphere preprints have
/// none in the record.
fn journal_of(record: &Value, id: Option<&str>) -> Option<String> {
    match first(record, "container-title") {
        Some(name) => Some(name.trim_end_matches(" Discussions").to_string()),
        None => id.filter(|id| id.starts_with("egusphere-")).map(|_| "EGUsphere".to_string()),
    }
}

/// The first of the texts under `key` in a record, if there is one that is not empty.
fn first(record: &Value, key: &str) -> Option<String> {
    let text = record.get(key)?.as_array()?.first()?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// The manuscript ID in a Copernicus DOI: its suffix, `10.5194/<journal>-<year>-<number>`.
fn id_of_doi(doi: &str) -> Option<String> {
    let doi = doi.trim().to_lowercase();
    let suffix = doi.strip_prefix("10.5194/")?;
    id_in(suffix).filter(|id| id == suffix)
}

/// The first Copernicus-style manuscript ID in a text, in lower case: `<journal>-<year>-<number>`, as
/// `\b[a-z]{2,12}-\d{4}-\d{1,6}\b` finds it.
fn id_in(text: &str) -> Option<String> {
    let digits = |s: &str, n: std::ops::RangeInclusive<usize>| n.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-')).find_map(|run| {
        let parts: Vec<&str> = run.split('-').collect();
        parts.windows(3).find_map(|w| {
            let letters = (2..=12).contains(&w[0].len()) && w[0].bytes().all(|b| b.is_ascii_alphabetic());
            (letters && digits(w[1], 4..=4) && digits(w[2], 1..=6)).then(|| w.join("-").to_lowercase())
        })
    })
}

/// Whether a path is a PDF file by its name.
pub fn is_pdf(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

/// The name of the review's folder: the ID, else the stem of the PDF's name, with what a name cannot hold replaced.
fn folder_name(id: Option<&str>, pdf: &Path) -> String {
    let stem = pdf.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let name: String = id.map(str::to_string).unwrap_or(stem).replace(['/', ':'], "-");
    let name = name.trim().trim_start_matches('.');
    if name.is_empty() { "review".to_string() } else { name.to_string() }
}

/// A folder `name` made in `parent`, or `name-2`, `name-3`… where it is taken.
fn make_folder(parent: &Path, name: &str) -> io::Result<PathBuf> {
    for n in 1.. {
        let folder = parent.join(if n == 1 { name.to_string() } else { format!("{name}-{n}") });
        match fs::create_dir(&folder) {
            Ok(()) => return Ok(folder),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

/// The reviews folder of the Settings; if it is not set, asked for with a folder picker and kept. None: not chosen.
/// Blocks, so not for the thread of the app.
fn reviews_folder(app: &AppHandle) -> Option<PathBuf> {
    let state = app.state::<AppState>();
    let set = state.settings.lock().unwrap().reviews_folder.clone();
    if set.is_some() {
        return set;
    }
    let folder = app
        .dialog()
        .file()
        .set_title("Choose the folder for your reviews")
        .set_can_create_directories(true)
        .blocking_pick_folder()?
        .into_path()
        .ok()?;
    let settings = Settings {
        reviews_folder: Some(folder.clone()),
        ..state.settings.lock().unwrap().clone()
    };
    crate::apply_settings(app, settings);
    Some(folder)
}

/// A review of the manuscript `pdf` started: its folder in the reviews folder, the PDF copied into it and open in
/// a tab, and the new project open with its card filled in as far as the PDF could be read. Nothing is done when
/// the reviews folder is not chosen. Blocks while the PDF is read and Crossref asked.
pub fn start(app: &AppHandle, pdf: &Path) -> Result<(), String> {
    if !is_pdf(pdf) || !pdf.is_file() {
        return Err(format!("{}: not a PDF file", pdf.display()));
    }
    let Some(reviews) = reviews_folder(app) else {
        return Ok(());
    };
    let state = app.state::<AppState>();
    let manuscript = read(pdf, &state.data.join("cache"));
    let folder = make_folder(&reviews, &folder_name(manuscript.id.as_deref(), pdf))
        .map_err(|e| format!("{}: {e}", reviews.display()))?;
    let copy = folder.join(pdf.file_name().ok_or_else(|| format!("{}: not a file", pdf.display()))?);
    if let Err(e) = fs::copy(pdf, &copy) {
        let _ = fs::remove_dir(&folder);
        return Err(format!("{}: {e}", copy.display()));
    }
    let root = folder.canonicalize().map_err(|e| format!("{}: {e}", folder.display()))?;
    let copy = root.join(copy.file_name().unwrap_or_default());
    state.started.lock().unwrap().insert(root.clone(), copy.clone());
    // the project is loaded, with what was read set in it (nothing is written until it is defined), and the PDF
    // is in its saved layout, before its page asks for either
    windows::load(app, &root)?;
    {
        let mut projects = state.projects.lock().unwrap();
        if let Some(project) = projects.get_mut(&root) {
            let set = project
                .set_title(manuscript.title)
                .and_then(|()| project.set_journal(manuscript.journal))
                .and_then(|()| project.set_manuscript(manuscript.id));
            if let Err(e) = set {
                eprintln!("{e}");
            }
        }
    }
    let layout = Layout { files: vec![copy.clone()], front: Some(copy) };
    state.saved.lock().unwrap().tabs.insert(root.clone(), layout);
    windows::open_project(app, &root).inspect_err(|_| {
        state.saved.lock().unwrap().tabs.remove(&root);
        windows::release(app, &root);
    })
}

/// A review of `pdf` started in the background (from the menu, or a file opened with the app): a failure is shown.
pub fn begin(app: &AppHandle, pdf: PathBuf) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = start(&app, &pdf) {
            app.dialog().message(e).title("New Review").show(|_| {});
        }
    });
}


/// A new review from a manuscript cancelled: its saved layout goes, and its folder, holding nothing but the PDF
/// copied in, goes to the Trash (taken back from there if need be).
pub fn undo(app: &AppHandle, root: &Path, pdf: &Path) {
    app.state::<AppState>().saved.lock().unwrap().tabs.remove(root);
    if holds_only(root, pdf)
        && let Err(e) = crate::trash::trash(root)
    {
        eprintln!("{e}");
    }
}

/// Whether a folder holds nothing but `file` (and the Finder's .DS_Store).
fn holds_only(folder: &Path, file: &Path) -> bool {
    fs::read_dir(folder).is_ok_and(|entries| {
        entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.file_name().is_none_or(|n| n != ".DS_Store"))
            .all(|p| p == file)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_id_of_a_copernicus_doi_is_its_suffix() {
        assert_eq!(id_of_doi("10.5194/egusphere-2026-6269").as_deref(), Some("egusphere-2026-6269"));
        assert_eq!(id_of_doi("10.5194/TC-2026-123").as_deref(), Some("tc-2026-123"));
    }

    #[test]
    fn a_doi_that_is_not_a_copernicus_manuscript_has_no_id() {
        assert_eq!(id_of_doi("10.5194/tc-18-1234-2024"), None);
        assert_eq!(id_of_doi("10.5194/egusphere-2026-6269-supplement"), None);
        assert_eq!(id_of_doi("10.1038/egusphere-2026-6269"), None);
        assert_eq!(id_of_doi("10.1016/j.epsl.2020.116283"), None);
    }

    #[test]
    fn the_id_in_a_text_is_the_first_of_its_shape() {
        let page = "Preprint. Discussion started: 3 March 2026\nhttps://doi.org/x EGUsphere-2026-6269 (tc-2026-123)";
        assert_eq!(id_in(page).as_deref(), Some("egusphere-2026-6269"));
        assert_eq!(id_in("see tc-2026-123.").as_deref(), Some("tc-2026-123"));
        assert_eq!(id_in("preprint cp-2025-7 and more").as_deref(), Some("cp-2025-7"));
    }

    #[test]
    fn a_text_without_the_shape_has_no_id() {
        assert_eq!(id_in(""), None);
        assert_eq!(id_in("2026-6269 and egusphere-26-6269 and a-2026-1 and tc-2026-1234567"), None);
        assert_eq!(id_in("tc-2026-12a and tc_x-2026-12 and 9tc-2026-12"), None);
    }

    #[test]
    fn the_title_and_journal_are_those_of_the_record() {
        let record = json!({
            "title": ["Overshoot of <i>CO</i><sub>2</sub> thresholds &amp; the ice\n sheet"],
            "container-title": ["The Cryosphere Discussions"],
        });
        assert_eq!(title_of(&record).as_deref(), Some("Overshoot of CO2 thresholds & the ice sheet"));
        assert_eq!(journal_of(&record, None).as_deref(), Some("The Cryosphere"));
        assert_eq!(title_of(&json!({"title": [" "]})), None);
        assert_eq!(title_of(&json!({})), None);
    }

    #[test]
    fn an_egusphere_preprint_without_a_journal_is_in_egusphere() {
        let record = json!({"title": ["Overshoot"], "container-title": []});
        assert_eq!(journal_of(&record, Some("egusphere-2026-6269")).as_deref(), Some("EGUsphere"));
        assert_eq!(journal_of(&record, Some("tc-2026-123")), None);
        assert_eq!(journal_of(&record, None), None);
    }

    #[test]
    fn the_folder_is_named_after_the_id_else_the_file() {
        let pdf = Path::new("/x/Smith et al. 2026.pdf");
        assert_eq!(folder_name(Some("tc-2026-123"), pdf), "tc-2026-123");
        assert_eq!(folder_name(None, pdf), "Smith et al. 2026");
        assert_eq!(folder_name(None, Path::new("/x/a:b.pdf")), "a-b");
        assert_eq!(folder_name(None, Path::new("/x/..pdf")), "review");
    }

    #[test]
    fn a_taken_folder_is_numbered() {
        let dir = std::env::temp_dir().join(format!("papers-review-manuscript-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(make_folder(&dir, "tc-2026-123").unwrap(), dir.join("tc-2026-123"));
        assert_eq!(make_folder(&dir, "tc-2026-123").unwrap(), dir.join("tc-2026-123-2"));
        assert_eq!(make_folder(&dir, "tc-2026-123").unwrap(), dir.join("tc-2026-123-3"));
        assert!(make_folder(&dir.join("not-there"), "x").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pdf_without_a_doi_is_read_without_the_network() {
        let pdf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/fixtures/lineno.pdf");
        let manuscript = read_with(&pdf, None);
        assert_eq!((manuscript.doi, manuscript.journal, manuscript.id), (None, None, None));
    }

    #[test]
    fn a_file_that_is_not_a_pdf_is_read_as_nothing() {
        assert_eq!(read_with(Path::new("/not/there.pdf"), None), Manuscript::default());
        assert_eq!(read_with(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml").as_path(), None), Manuscript::default());
    }

    #[test]
    fn a_folder_holding_the_copied_pdf_alone_is_one_to_take_back() {
        let dir = std::env::temp_dir().join(format!("papers-review-holds-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("manuscript.pdf");
        fs::write(&pdf, "pdf").unwrap();
        fs::write(dir.join(".DS_Store"), "").unwrap();
        assert!(holds_only(&dir, &pdf));
        fs::write(dir.join("review.md"), "mine").unwrap();
        assert!(!holds_only(&dir, &pdf));
        let _ = fs::remove_dir_all(&dir);
    }
}
