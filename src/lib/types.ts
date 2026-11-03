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
};

/** mirrors settings::DateFormat: 2026-11-14, 14.11.2026, 11/14/2026 */
export type DateFormat = "ymd" | "dmy" | "mdy";

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
