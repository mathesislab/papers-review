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

One window per project, or one window for all of them with a bar to switch
between projects, as set in Settings (`⌘,`); several windows when asked
(File > Open Project in New Window…, `⇧⌘O`). All the windows of a project
share its colour.

- **Colour bar**: a thin bar along the top of the page, under the title bar,
  in the project's colour, to tell the windows apart in Mission Control and
  the window switcher: the pastel colours of Papers Desktop's window, by
  name (`colour = "orange"` in `review.toml`). Right-click it: the colour, or
  none. A new project takes the next colour not in use among those open.
- **Project bar** (Settings; on by default): a column at the left with a tile
  per project, in its colour with its initials, the current one marked, a
  badge with the days left when the due date is within a week (`!` once
  past); `+` opens another. A click switches the window to the project (its
  page loads again, the review pane follows, writing what was typed to the
  old review first); right-click: Open in New Window, Remove from Bar. Open
  Project adds to the bar and switches the window in front, or brings
  forward the window that shows the project. The bar is kept in `state.json`
  and listed in the Window menu (`⌘1` to `⌘9`); removing the project of a
  window switches it to a neighbour, or closes it when none is left.
- **Files** (sidebar, `⌘⌥S`, or the button at the left of the tabs): the
  project's folder at the top and its files as a tree, a line along what
  each open folder holds; the linked files apart, with where they lie; the
  due date. Hidden, the viewer takes the whole window.
- **Viewer**: a PDF in PDFKit (tauri-plugin-pdfkit), its tools in a bar
  under the tabs as in Papers Desktop (the outline, beside the PDF; find;
  the page; zoom; open in the default app); a text file rendered or as source (`⌘R`: Markdown, READMEs, code
  and data descriptions). Tabs for the files open.
- **Review pane** (`⌘J`): a Markdown editor over the viewer, at the bottom
  or on the right (`⌘⇧J`), its inner edge dragged for its size; its side,
  sizes and whether shown kept per project (the app's `state.json`). `⌘J`
  hides it when it has the keys (they go back to the PDF), else shows it
  with them. Hidden, nothing remains. The arrow of its header collapses it
  to that header (a strip on the right), the keys back to the PDF, and
  brings it back; `⌘J` expands it too. One window of a project holds its
  pane at a time.
- **The review**: any Markdown file of the project (`review` in
  `review.toml`, `review.md` without; Edit in Review Pane on a Markdown
  file), marked ✎ in the files. Written half a second after the typing
  stops and as the pane loses the keys; read again when changed elsewhere,
  or, with changes typed not yet written, Reload or Keep mine.
- **Tabs of the pane**: the headings at the shallowest level with two of
  them (`## Round 1`, `## Round 2` under a `# Review`); a tab shows its
  section alone, the rest of the file hidden (it stays one file), All the
  whole of it.

The PDF view lies above the page of the window (a native view over the web
view), so the page cannot draw over it. The pane is a second web view above
the first (Tauri's multiwebview, the `unstable` feature), opaque and square
(the web view is not transparent); translucency would need an
NSVisualEffectView under it.

## Writing the review

- CodeMirror 6, Markdown.
- `⌘'` quotes the selection of the PDF at the cursor, as a paragraph of its
  own, with its page (the printed label, else the number) and its lines
  where the manuscript has line numbers: `**p. 5, l. 136:** "…"`,
  `**p. 5, ll. 136–138:** "…"`, `**pp. 5–6, ll. 210–228:** "…"`. The pane
  shows with the keys, to write what the quote is about; `Esc` gives them
  back to the PDF. The plugin gives the selection line by line with the
  words beside each line (`selection`); `quote.ts` takes the whole number
  nearest each line, left of it first, and keeps the page alone when the
  numbers do not fit the lines selected. Words split at a line end are made
  whole, ligatures undone. The numbers of a two-column layout are not read
  yet: PDFKit glues them to the line's text. `⌃L` inserts the location in
  view.
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
2. Done: the review pane, the second web view over the viewer; CodeMirror,
   saving, the file watched; any Markdown file as the review; the tabs of its
   sections; the pane collapsed to its header, the PDF's tools bar, the
   files as a clearer tree.
3. Quotes with page and line (done); the symbols.
4. The project bar and Settings (done): `settings.json` in the app's data
   folder.
5. Export.
6. References.
7. Text files; new review from a dropped manuscript.
