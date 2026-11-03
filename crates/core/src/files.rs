//! The files of a project, as a tree: folders first, then files, by name;
//! hidden files and `review.toml` left out.

use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::project::MANIFEST;

/// What a file is, for how it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Folder,
    Pdf,
    Markdown,
    /// text to read as it is: code, data descriptions, configuration
    Text,
    Bib,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
    /// a linked file that is not there
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
}

/// The most files and folders a tree lists: a folder of data can hold many.
const MOST: usize = 5000;

pub fn kind(path: &Path) -> Kind {
    if path.is_dir() {
        return Kind::Folder;
    }
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "pdf" => Kind::Pdf,
        "md" | "markdown" => Kind::Markdown,
        "bib" => Kind::Bib,
        "txt" | "text" | "csv" | "tsv" | "json" | "yaml" | "yml" | "toml" | "xml" | "nml" | "cfg"
        | "ini" | "log" | "tex" | "jl" | "py" | "r" | "m" | "f" | "f90" | "f95" | "c" | "h"
        | "cpp" | "rs" | "sh" | "ncl" => Kind::Text,
        "" if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().to_uppercase().starts_with("README")) =>
        {
            Kind::Text
        }
        _ => Kind::Other,
    }
}

/// The tree of a folder.
pub fn tree(root: &Path) -> Vec<Node> {
    let mut count = 0;
    children(root, &mut count)
}

fn children(folder: &Path, count: &mut usize) -> Vec<Node> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut nodes: Vec<Node> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == MANIFEST {
                return None;
            }
            let path = entry.path();
            Some(Node {
                kind: kind(&path),
                name,
                path,
                children: Vec::new(),
                missing: false,
            })
        })
        .collect();
    nodes.sort_by(order);
    nodes.truncate(MOST.saturating_sub(*count));
    *count += nodes.len();
    for node in &mut nodes {
        if node.kind == Kind::Folder && *count < MOST {
            node.children = children(&node.path, count);
        }
    }
    nodes
}

/// Folders first, then by name, as the Finder sorts (case aside, numbers by value).
fn order(a: &Node, b: &Node) -> Ordering {
    let folder = |n: &Node| n.kind != Kind::Folder;
    folder(a).cmp(&folder(b)).then_with(|| natural(&a.name, &b.name))
}

fn natural(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let number = |s: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(c) = s.peek().copied().filter(char::is_ascii_digit) {
                        digits.push(c);
                        s.next();
                    }
                    digits
                };
                let (m, n) = (number(&mut a), number(&mut b));
                let (m, n) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                match m.len().cmp(&n.len()).then_with(|| m.cmp(n)) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }
            (Some(x), Some(y)) => {
                match x.to_lowercase().cmp(y.to_lowercase()) {
                    Ordering::Equal => {
                        a.next();
                        b.next();
                    }
                    other => return other,
                }
            }
        }
    }
}

/// A file linked from elsewhere, there or not.
pub fn linked(path: &Path) -> Node {
    Node {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path.to_path_buf(),
        kind: kind(path),
        children: Vec::new(),
        missing: !path.exists(),
    }
}

/// The files of `kind` among `nodes` and the folders in them, added to `found`; a linked file that is
/// not there is left out.
pub fn of_kind(nodes: &[Node], kind: Kind, found: &mut Vec<PathBuf>) {
    for node in nodes {
        if node.kind == kind && !node.missing {
            found.push(node.path.clone());
        }
        of_kind(&node.children, kind, found);
    }
}

/// A file or a folder (with all it holds) copied to `to`, which is not there yet.
pub fn copy(from: &Path, to: &Path) -> io::Result<()> {
    if from.is_dir() {
        fs::create_dir(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(nodes: &[Node]) -> Vec<&str> {
        nodes.iter().map(|n| n.name.as_str()).collect()
    }

    #[test]
    fn folders_first_by_name_hidden_files_and_review_toml_left_out() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for folder in ["round-2", "round-10", "Round-1", ".git"] {
            fs::create_dir(root.join(folder)).unwrap();
        }
        for file in ["review.md", "review.toml", ".DS_Store", "a.pdf", "round-2/revised.pdf"] {
            fs::write(root.join(file), "").unwrap();
        }
        let tree = tree(root);
        assert_eq!(names(&tree), ["Round-1", "round-2", "round-10", "a.pdf", "review.md"]);
        assert_eq!(names(&tree[1].children), ["revised.pdf"]);
        assert_eq!(tree[1].children[0].kind, Kind::Pdf);
        assert_eq!(tree[4].kind, Kind::Markdown);
    }

    #[test]
    fn kinds() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(kind(dir.path()), Kind::Folder);
        assert_eq!(kind(Path::new("x/Paper.PDF")), Kind::Pdf);
        assert_eq!(kind(Path::new("x/refs.bib")), Kind::Bib);
        assert_eq!(kind(Path::new("x/run.jl")), Kind::Text);
        assert_eq!(kind(Path::new("x/README")), Kind::Text);
        assert_eq!(kind(Path::new("x/fig.png")), Kind::Other);
    }

    #[test]
    fn a_tree_lists_so_many_files_at_most() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        fs::create_dir(&data).unwrap();
        for i in 0..MOST + 10 {
            fs::write(data.join(format!("{i}.nc")), "").unwrap();
        }
        let tree = tree(dir.path());
        assert_eq!(tree[0].children.len(), MOST - 1);
    }
}
