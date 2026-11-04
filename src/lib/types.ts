// What the app tells the pages (mirrors src-tauri/src/lib.rs and state.rs).

/** mirrors review_core::Colour */
export type Colour = "blue" | "teal" | "green" | "yellow" | "orange" | "red" | "pink" | "purple" | "graphite" | "none";

/** mirrors review_core::Kind */
export type Kind = "folder" | "pdf" | "markdown" | "text" | "bib" | "other";

/** mirrors review_core::Node */
export type Node = { name: string; path: string; kind: Kind; children?: Node[]; missing?: boolean };

/** mirrors ProjectView */
export type Project = {
  root: string;
  name: string;
  title: string | null;
  /** at most ten letters, for the tile of the project bar */
  short: string | null;
  journal: string | null;
  manuscript: string | null;
  /** the heading of the PDF export; the page says "Manuscript review" when unset */
  heading: string | null;
  due: string | null;
  colour: Colour;
  files: Node[];
  linked: Node[];
  /** the Markdown file of the review */
  review: string;
  /** not defined yet: the folder had no review.toml when it was opened */
  new: boolean;
};

/** mirrors Recent */
export type Recent = { root: string; name: string; colour: Colour; missing: boolean };

/** mirrors state::Layout */
export type Layout = { files: boolean; files_width: number; tabs: string[]; front: string | null; pages: Record<string, number> };

/** mirrors settings::Settings */
export type Settings = {
  /** the projects in one window, with a bar to switch between them; off: each in a window of its own */
  project_bar: boolean;
  /** the user's own sequences typed that become symbols, and their replacement; an empty one turns a default off */
  symbols: [string, string][];
  date_format: DateFormat;
  /** the font of the editors and its size (points) */
  editor_font: EditorFont;
  editor_size: number;
  /** what Tab puts in: four spaces, two, or a tab */
  indent: Indent;
  /** the text of the editors a column of a readable width, a margin on each side */
  readable_width: boolean;
  /** the lines of the editors numbered */
  line_numbers: boolean;
  /** the reviewer's name, to sign the exports with */
  reviewer_name: string;
  /** sign the PDF with it; off: anonymous */
  sign: boolean;
  /** the PDF's header holds the journal, the manuscript ID and the date as well as the heading */
  header_details: boolean;
  /** the folder the folders of new reviews are made in; null: asked for with the first */
  reviews_folder: string | null;
  /** the colour of the bar at the top of the windows; none: no bar */
  window_colour: Colour;
  /** look for updates at start and twice a day; they are installed only when asked */
  check_updates: boolean;
};

/** mirrors settings::DateFormat: 2026-11-14, 14.11.2026, 11/14/2026 */
export type DateFormat = "ymd" | "dmy" | "mdy";

/** mirrors settings::Indent */
export type Indent = "four" | "two" | "tab";

/** mirrors settings::EditorFont */
export type EditorFont = "system" | "serif" | "mono";

/** mirrors BarItem: a project of the project bar */
export type BarItem = { root: string; name: string; short: string | null; colour: Colour; due: string | null; missing: boolean };

/** mirrors ProjectInfo: what the project's info card shows and keeps */
export type ProjectInfo = {
  root: string;
  name: string;
  title: string | null;
  short: string | null;
  journal: string | null;
  manuscript: string | null;
  heading: string | null;
  /** YYYY-MM-DD */
  due: string | null;
  colour: Colour;
  /** the Markdown file of the review */
  review: string;
  /** the Markdown files of the project, and of the linked ones: those to choose the review from */
  markdown: string[];
  /** the files and folders linked from elsewhere */
  linked: string[];
};

/** mirrors InfoEdit: the fields of the card that are text */
export type InfoEdit = { title: string | null; short: string | null; journal: string | null; manuscript: string | null; heading: string | null; due: string | null };

/** mirrors papers_core::Item: a reference of a .bib file, as the references panel shows it */
export type Item = {
  /** the key, unless another .bib file searched has the same key (then `key#2`) */
  id: string;
  key: string;
  /** the BibTeX entry type, e.g. "article" */
  kind: string;
  title: string;
  /** "Given Family" */
  authors: string[];
  year: number | null;
  /** the journal or book */
  container: string | null;
  /** a preprint's server, where its journal would be */
  server: string | null;
  doi: string | null;
  url: string | null;
  keywords: string[];
  /** the PDFs of the entry */
  files: string[];
  /** the .bib file of the entry */
  bib: string;
  /** one line under the title: "Feldmann & Levermann · 2017 · Cryosphere" */
  subtitle: string;
};

/** mirrors papers_core::Span: a piece of a text shown, marked where a search matched */
export type Span = { text: string; mark: boolean };

/** mirrors papers_core::Hit */
export type Hit = {
  item: Item;
  title: Span[];
  subtitle: Span[];
  /** for each author, whether it matched */
  authors: boolean[];
};

/** mirrors papers_core::YearCount */
export type YearCount = { year: number; count: number; selected: boolean };

/** mirrors papers_core::Found: what `references` and `closest_references` return */
export type Found = {
  /** the first `limit` matches, newest first (the closest first for `closest_references`) */
  hits: Hit[];
  /** the matches there are */
  total: number;
  /** the matches of each year, the years between included (none for `closest_references`) */
  years: YearCount[];
};
