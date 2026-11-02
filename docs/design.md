# Design and plans

## Principles

- A project is a folder: the manuscript and its versions, the files that
  come with it (supplement, code and data, READMEs, the editor's letter), the
  review, and files elsewhere linked from it. Nothing else is the truth: no
  database of the app, no account, nothing sent.
- Plain files, any of which another app opens: PDFs, Markdown, text. The
  folder may live in a synced folder (Google Drive, Dropbox, iCloud): the app
  writes atomically and reloads a file changed on disk.
- No layout imposed. The review is any Markdown file of the project; rounds,
  sections and file names are the author's.
- The PDF shows as Preview shows it (PDFKit), and keeps its size: the
  review, the files and the references come over it when called, and go.
- Built with Tauri 2 (Rust) and SvelteKit, as Papers Desktop; macOS first.

## The project

```
egusphere-2026-1234/
├── review.toml     title, journal, manuscript, due date, colour, linked files
├── review.md       the review (one section per round, or a file per round)
├── round-1/
│   ├── manuscript.pdf
│   └── code-and-data/README.md
├── round-2/
│   ├── revised-manuscript.pdf
│   └── response-to-reviewers.pdf
└── export/
```

`review.toml` is the one file of the app: what the folder does not tell.
Files from elsewhere are linked (their path in `review.toml`), copied or
moved in, as asked. A folder without `review.toml` opens too: the app writes
one when there is something to keep.

New review: a manuscript dropped on the app makes the folder; the title,
journal and DOI are read from the PDF with papers-rs (as papers reads them).

## The window

One window per project, several when asked (a file opened in a new window);
all the windows of a project share its colour.

- **Colour bar**: a thin bar along the top of the page, under the title bar,
  in the project's colour, to tell the windows apart in Mission Control and
  the window switcher: the pastel colours of Papers Desktop's window, by
  name (`colour = "orange"` in `review.toml`). Right-click it: the colour, or
  none. A new project takes the next colour not in use among those open.
- **Files** (sidebar, `⌘⌥S`): the files of the project by folder, the linked
  files, the due date. Hidden, the viewer takes the whole window.
- **Viewer**: a PDF in PDFKit (tauri-plugin-pdfkit), with find, zoom, the
  outline; a text file rendered or as source (`⌘R`: Markdown, READMEs, code
  and data descriptions). Tabs for the files open.
- **Review pane** (`⌘J`): a Markdown editor floating over the viewer, at the
  bottom or on the right (`⌘⇧J`), its size and place kept per project.
  Tabs: the top-level headings of the file (a level to choose), or several
  files. Hidden, a strip remains, or nothing in full screen.

The PDF view lies above the page of the window (a native view over the web
view), so the page cannot draw over it. The pane is a second web view above
the first (Tauri's multiwebview, the `unstable` feature); else a child
window attached to the main one. Its translucency needs an
NSVisualEffectView under it; an opaque pane needs nothing.

## Writing the review

- CodeMirror 6, Markdown.
- `⌘'` quotes the selection of the PDF at the cursor, with its page, and its
  line where the manuscript has line numbers (the numbers in the margin
  beside the selection): `**p. 5, l. 136:** "…"`. `⌃L` inserts the location
  in view.
- Typed sequences become symbols: `->` →, `<-` ←, `<->` ↔, `=>` ⇒, `>=` ≥,
  `<=` ≤, `!=` ≠, `~=` ≈, `+-` ±, LaTeX names (`\Delta` Δ, `\times` ×,
  `\deg` °), straight quotes to curly ones, `--` –, `...` …. `⌘Z` right
  after keeps what was typed; `⌘/` lists them; the list is editable.
- Saved as typed.

## Export

- A round (or the whole file) as PDF: the Markdown to HTML in Rust
  (comrak), printed by WebKit; a header (journal, manuscript, title, date),
  anonymous unless asked.
- Plain text on the clipboard for a journal's web form: the symbols kept,
  the Markdown marks dropped.
- Into `export/` of the project.

## References

A `.bib` in the project, or a papers library (a `.papers/config.json` in
the folder or above it, or papers' default library), is searched as in
Papers Desktop: words, field filters, a citation key, or a reference
selected in the PDF. `↵` opens the paper's PDF in a tab of the same
project, `⌘↵` in a new window; `⌘.` copies `[@key]`, `⌘L` `\cite{key}`.

papers-rs finds the library (its configuration: the library of a folder,
the default one); papers-core (Papers Desktop) loads and searches the
`.bib` files, a git dependency until it leaves papers-desktop. The Search
Papers service of Papers Desktop also works from here.

## Crates

- [papers-rs](https://github.com/mathesislab/papers-rs): the PDF's title and
  DOI, the papers configuration.
- papers-core (in [papers-desktop](https://github.com/mathesislab/papers-desktop)):
  the `.bib` files, their search.
- [tauri-plugin-pdfkit](https://github.com/mathesislab/tauri-plugin-pdfkit):
  the PDF view.

## Earlier tools

None writes a review beside the manuscript from a folder of plain files.
Obsidian with PDF++ comes closest (a folder, a PDF pane, a selection copied
as a quote linked to its page), as a general notes app, side by side, with
no line numbers or rounds. Skim, Highlights, PDF Expert and Zotero keep
notes as annotations, in the PDF or a database. hypothes.is (used by AGU
for reviews) is on the web. pdfannots and academic-doc-generator turn PDF
annotations into Markdown with pages (the latter reads the line numbers of
the margin).

## First steps

1. Done: the window, the files, a PDF in the viewer (tauri-plugin-pdfkit),
   the colour bar, `review.toml`; the state between sessions in the app's
   data folder (`state.json`: the recent projects, the windows open, how
   each looked).
2. The review pane: the second web view over the viewer, CodeMirror, saving.
3. Quotes with page and line; the symbols.
4. Export.
5. References.
6. Text files; new review from a dropped manuscript.
