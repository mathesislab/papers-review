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

New review (File > New Review from Manuscript…, `⌘N`; a PDF dropped on the
start window or on the Dock icon): the folder is made in the reviews folder
of Settings (asked for with the first review), named after the manuscript ID
or the PDF's name (`-2`, `-3`… where taken), the PDF copied into it and open in
a tab, and the project opened with its card filled in. The PDF is read with
papers-rs as papers reads one: the DOI of its metadata, else of its first
pages if the title Crossref gives it is in the text; Crossref's record gives
the title and journal (EGUsphere for a preprint without one), else the PDF's
own title is used. The ID is the suffix of a Copernicus DOI
(`10.5194/egusphere-2026-6269`), else one of that shape on the first page.
Without network, or a DOI, the fields are left empty. Nothing is written
until Create; Cancel moves the folder made for it to the Trash, as long as
it holds nothing but the PDF copied in.

## The window

One window per project, or one window for all of them with a bar to switch
between projects, as set in Settings (`⌘,`); several windows when asked
(File > Open Project in New Window…, `⇧⌘O`). `⇧⌘[` and `⇧⌘]` (Window menu)
show the tab before or after the one in front.

- **Colour bar**: a thin bar along the top of the page, under the title bar,
  in the app's colour, to tell its windows apart from others in Mission
  Control and the window switcher: the pastel colours of Papers Desktop's
  window, by name (Settings ▸ General, `window_colour`; none by default).
  Right-click it: the colour, or none. A project has a colour of its own
  (`colour = "orange"` in `review.toml`) for its tile and its dot among the
  recent projects; a new one takes the next not in use among those open.
- **Project bar** (Settings; on by default): a column at the left with a tile
  per project, in its colour with its short name (ten letters at most, on two
  lines when long) or initials, the due date under it, and down its left side
  a bar for how near that is (green, yellow within a week, orange within
  three days, red when due or past), the current
  one marked (a click on it shows or hides the files); `+` opens another. A
  click switches the window to the project (its
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
  or copied in, as asked (files dropped on the window from the Finder, the
  project's own excepted, are added the same way); a file or folder is shown in the Finder; a file
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
  ‹ and › at its left, back and forward to the page a link left; the page,
  a click on it a field to type one; zoom; open in the default app). A Markdown, text or BibTeX
  file is edited in its tab, filling the viewer (the same editor as the
  review pane, monospace unless Markdown): written half a second after the
  typing stops and as it loses the keys, read again when changed on disk
  (Reload or Keep mine with changes not yet written); a file that is not
  UTF-8 keeps its name and Open in Default App. Tabs for the files open; that
  of a paper outside the project has a `+` before its `✕`, Add to the
  Project…: linked, or copied, and the tab then shows the copy, at its page.
- **Go menu**: Back (`⌘[`) and Forward (`⌘]`) in the PDF's history, Previous
  and Next Page (`⌥⌘↑`, `⌥⌘↓`), First and Last Page, Go to Page… (`⌥⌘G`, the
  tools bar's page field). `⌘[` and `⌘]` are indent less and more in an
  editor with the keys (the review pane, a file edited in its tab), as in
  CodeMirror.
- **Review pane** (`⌘J`): a Markdown editor over the viewer, at the bottom
  or on the right (`⌘⇧J`), its inner edge dragged for its size; its side,
  sizes and whether shown kept per project (the app's `state.json`). `⌘J`
  hides it when it is open, wherever the keys are (they go back to the PDF),
  else shows it with them. Hidden, nothing remains. A click on its header (not on a
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
- Settings ▸ Editor also has what `Tab` puts in at the cursor (4 spaces, 2
  or a tab; with lines selected it indents them, `⇧Tab` takes it out), the
  readable line length (on by default: the text a column of 46em in the
  middle, a margin on each side down to a narrow window, as in Obsidian) and
  the line numbers (off by default).
- The bar of the review pane, and of a file edited in a tab, has Ω (the
  symbols, `⌘/`), the line numbers shown or hidden (in every editor), and
  the words: of the whole, or of the section shown alone, and of the
  selection (`56 of 1,234 words`).
- A file changed on disk and read again changes only what differs: the
  cursor and the undo history stay.
- A selection made with the mouse scrolls at the top and foot of the editor
  alike, faster the further into the edge and the longer it is held there
  (CodeMirror's own speed goes with the distance past the edge, which the
  pane at the foot of the screen hardly leaves).
- A line that starts with a location (`p5, l136–138:`, `p5–6:`, `l153:`,
  `p. iii:`) has it coloured, as the headings are.
- `⌘'` quotes the selection of the PDF at the cursor, as a paragraph of its
  own, with its page (the printed label, else the number) and its lines
  where the manuscript has line numbers: `**p. 5, l. 136:** "…"`,
  `**p. 5, ll. 136–138:** "…"`, `**pp. 5–6, ll. 210–228:** "…"`. The pane
  shows with the keys, to write what the quote is about; `Esc` gives them
  back to the PDF. The plugin gives the selection line by line with the
  words beside each line (`selection`); `quote.ts` takes the whole number
  nearest each line, left of it first, with no word but numbers beyond it
  (a number in the text, `and 10000 m`, is none), and keeps the page alone
  when the numbers do not fit the lines selected. Words split at a line end are made
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

- What the review pane shows: the round of its tab, or on All the whole file.
  A round holds just its section (nothing from before the first one). From
  the button in the pane's header, or File > Export as PDF (⌘E) and Copy as
  Plain Text (⇧⌘E); the labels name the section ("Export Round 2 as PDF").
  The files are named from the review and the section (`review-round-2`).
- As PDF: the Markdown to HTML (`export.ts`, from the editor's own parser),
  laid out as an A4 page by a hidden window (`/export`) and printed by
  WebKit. A header with the project's export heading (info card; default
  "Manuscript review") and, if Settings > Export asks, the journal,
  manuscript ID and date below it. Locations (`p5, l136:`) are set in a
  muted colour. Anonymous unless Settings > Export has "Sign the reviews"
  on, then the reviewer's name closes the page. Saved in `export/` of the
  project and shown in the Finder.
- As plain text, for a journal's web form: the Markdown marks dropped
  (headings are lines of their own, list items keep "- ", links become
  "text (url)"), the symbols and the locations kept. On the clipboard and
  saved in `export/`; the pane says "copied".
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
Papers Desktop: words from their start, field filters, a citation key; with
nothing typed, all of them, newest first. `⇧⌘R`
(Edit > References…) lists them over the review pane (RefList), which is
shown; with a reference selected in the PDF, the list is the papers closest
to its text until something is typed. The key on the paper marked:

- `↵` puts a citation in at the cursor: "Feldmann and Levermann (2015)",
  "Robinson et al. (2012)"; `⇧↵` the full reference, as Copernicus sets it;
- `⌘.` puts in `[@key]`, `⌘L` `\cite{key}`;
- `⌘↵` opens its PDF in a tab of the project (without a PDF, nothing);
- `Esc` closes the list.

The list shows the sources (the `.bib` files) and these keys at its foot. The
pane has no PDF to open a tab with: the page does it (`open_reference_pdf`
tells it), so a paper outside the project opens as a tab too, not kept with
the layout.

Sources (`refs.rs`): the `.bib` files of the project (its folder and the
linked files), and the `.bib` of the papers library, which papers-rs finds
(`find_config` from the project's folder: a library there or above, else the
machine configuration's default library). The files are loaded as one
papers-core library when first asked, per project, kept while the project
is open, read again when one of them changes (`BibWatch`) or when the
project's files change to other `.bib` files, and told to the page and the
pane as `references-changed`. A file that does not parse is the error the
panel shows; a papers configuration that cannot be read leaves the project's
own files. papers-core is a git dependency until it leaves papers-desktop.
The Search Papers service of Papers Desktop also works from here.

Commands: `references(query, limit)` (words matching from the start of a
word) and `closest_references(text, limit)` give papers-core's `Found`;
`reference_sources()` lists the files searched; `open_reference_pdf(path)`
tells the page of the calling window to open the PDF in a tab (`open-file`).
Edit ▸ References… (⇧⌘R) goes to the page of the window in front, as the
other page commands.

## Updates

The app looks for a new version a while after the start and twice a day
(Settings ▸ General ▸ Updates, `check_updates`; Papers Review ▸ Check for
Updates… shows them there and looks now, as Papers Desktop does: no alert),
and installs it, with its download in a bar, then restarts, only when asked (`update.rs`, as in Papers Desktop). The updates
are signed with the updater key: the public key is `plugins.updater.pubkey`
in `tauri.conf.json`, the private one the secret `TAURI_SIGNING_PRIVATE_KEY`
of the repository (no password). A release is made by tagging `vX.Y.Z`
(the version of `tauri.conf.json` and the Cargo manifest): the workflow
`release.yml` builds the app and dmg and uploads them, with the signed
`.app.tar.gz` and `latest.json` the apps read, to a draft release, which is
reviewed and published. The apps are not code-signed or notarized.

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
3. Done: quotes with page and line; the typed symbols.
4. Done: the project bar and Settings (`settings.json` in the app's data
   folder); the info card, the new project's card.
5. Done: export, as PDF and as plain text.
6. Done: references.
7. Done: text files edited in their tab; new review from a manuscript.
