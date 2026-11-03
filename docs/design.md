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
moved in, as asked. A folder without `review.toml` is a new project: its
window opens with a card asking for the name, short name, journal,
manuscript ID, due date and review file (a new `review.md`, or a Markdown
file of the folder). Create writes `review.toml` and makes the review file;
Cancel closes the project, leaving nothing, and it is not among the recent
ones until created. A review file that is missing later is not made as one
types: the pane offers to make it.

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
  per project, in its colour with its short name (ten letters at most, on two
  lines when long) or initials, the due date under it with a dot (green, yellow
  within a week, orange within three days, red when due or past), the current
  one marked; `+` opens another. A click switches the window to the project (its
  page loads again, the review pane follows, writing what was typed to the
  old review first); right-click: Open in New Window, Remove from Bar. Open
  Project adds to the bar and switches the window in front, or brings
  forward the window that shows the project. The bar is kept in `state.json`
  and listed in the Window menu (`⌘1` to `⌘9`); removing the project of a
  window switches it to a neighbour, or closes it when none is left.
- **Files** (sidebar, `⌘⌥S`, or the button at the left of the tabs): a box at
  the top with the project's name, journal and manuscript ID, and due date
  with the days left (a click on it, or its ⓘ, opens the project's info);
  under Folder, the project's folder and its files as a tree, a line along
  what each open folder holds; under Linked, the linked files, with where
  they lie. The review file carries a `review` pill. Hidden, the viewer takes
  the whole window. Right-click: the project's folder and its folders make a Markdown file (named in a row of its
  own, `untitled.md`), or take files and folders in, linked from where they are
  or copied in, as asked; a file or folder is shown in the Finder; a file
  opens in the default app; a linked one is removed (the file stays), one of
  the project is moved to the Trash after asking.
- **Project info** (`⌘I`, File > Project Info…, a click on the project's box
  in the files, or in the bar's tile menu): a card at the top of the window
  like a macOS sheet, open until Done or `Esc` (Cancel). Name (the folder's
  name when empty), short name (ten letters at most, for the tile of the
  bar), journal, manuscript ID, due date (typed in the format of Settings or
  picked from a calendar), colour, the review file (or a new
  `review.md`), the linked files and folders, and the project's folder with
  Show in Finder (to rename it there). Written on Done; the PDF hides
  while the card shows.
- **Viewer**: a PDF in PDFKit (tauri-plugin-pdfkit), its tools in a bar
  under the tabs as in Papers Desktop (the outline, beside the PDF; find;
  the page; zoom; open in the default app). A Markdown, text or BibTeX
  file is edited in its tab, filling the viewer (the same editor as the
  review pane, monospace unless Markdown): written half a second after the
  typing stops and as it loses the keys, read again when changed on disk
  (Reload or Keep mine with changes not yet written); a file that is not
  UTF-8 keeps its name and Open in Default App. Tabs for the files open.
- **Review pane** (`⌘J`): a Markdown editor over the viewer, at the bottom
  or on the right (`⌘⇧J`), its inner edge dragged for its size; its side,
  sizes and whether shown kept per project (the app's `state.json`). `⌘J`
  hides it when it has the keys (they go back to the PDF), else shows it
  with them. Hidden, nothing remains. A click on its header (not on a
  button or tab), or its arrow, collapses it to that header (a strip on the
  right), the keys back to the PDF, and brings it back; `⌘J` expands it too. One window of a project holds its
  pane at a time.
- **The review**: any Markdown file of the project (`review` in
  `review.toml`, `review.md` without; Edit in Review Pane on a Markdown
  file), marked with a `review` pill in the files. Written half a second after the typing
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

- CodeMirror 6, Markdown. Its font (system, serif or monospace) and size
  are set in Settings, as is the format of dates (2026-11-14, 14.11.2026 or
  11/14/2026); text files take the size, in monospace.
- A line that starts with a location (`p5, l136–138:`, `p5–6:`, `l153:`,
  `p. iii:`) has it coloured, as the headings are.
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
  yet: PDFKit glues them to the line's text. `⌘;` quotes it the same way
  without the quotation marks. `⌃L` inserts the location in view.
- Symbols, in the Markdown editors (the review, a Markdown tab; not in code
  spans or blocks): typed sequences become one as the last character is
  typed (`->` →, `<-` ←, `<->` ↔, `=>` ⇒, `<=>` ⇔, `>=` ≥, `<=` ≤, `!=` ≠,
  `~=` ≈, `+-` ±, `--` –, `---` — (not in a line of dashes alone: Markdown's
  rules and tables), `...` …, powers after a letter, a digit or
  `)`: `^2` ², `^-1` ⁻¹; indices after a letter: `CO_2` CO₂); `Tab` after a
  LaTeX name as in the Julia REPL (`\alpha` α, `\Delta` Δ, `\deg` °,
  `\times` ×, `\^2` ², `\_2` ₂). `⌘Z` right after gives back what was typed.
  `⌘/` (Edit > Symbols…) lists both, filtered as typed; Enter or a click puts
  one in. In Settings each default can be turned off and sequences of one's
  own added (`settings.json`). Quotes stay straight: the review is pasted
  into plain forms.
- Saved as typed.

## Export

- The review (or its section shown in the pane) as PDF or text, from the
  File menu (`⌘E`, `⇧⌘E`), into `export/` of the project (`export_pdf`,
  `export_text`). The pane turns the Markdown into HTML and the text with
  the marks dropped (the symbols kept); the text is also copied.
- The header is the project's heading (`heading` in `review.toml`, "Manuscript
  review" without), with journal, manuscript, title and date only when the
  setting says so. The review is anonymous unless the settings give a
  reviewer's name and sign it.
- PDF printing: a hidden window (`export-<n>`, the route `/export`) asks for
  the HTML (`export_html`), renders it and says so (`export_rendered`); its
  WKWebView is printed with AppKit, A4, margins of 2 cm, to a file saved
  beside the target and renamed. macOS only. The print operation is made on
  the main thread but run on a thread of its own: WebKit's printing view
  paginates and draws from the thread that runs it, and on the main thread
  it finds blank pages or no end of them. The view must be framed before it
  runs. `cargo run --example export_pdf -- <file.pdf>` prints a long page
  that way from a window never shown.

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
