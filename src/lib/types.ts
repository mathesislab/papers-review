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
  /** at most four letters, for the tile of the project bar */
  short: string | null;
  journal: string | null;
  manuscript: string | null;
  due: string | null;
  colour: Colour;
  files: Node[];
  linked: Node[];
  /** the Markdown file of the review */
  review: string;
};

/** mirrors Recent */
export type Recent = { root: string; name: string; colour: Colour; missing: boolean };

/** mirrors state::Layout */
export type Layout = { files: boolean; files_width: number; tabs: string[]; front: string | null; pages: Record<string, number> };

/** mirrors settings::Settings */
export type Settings = {
  /** the projects in one window, with a bar to switch between them; off: each in a window of its own */
  project_bar: boolean;
};

/** mirrors BarItem: a project of the project bar */
export type BarItem = { root: string; name: string; short: string | null; colour: Colour; due: string | null; missing: boolean };
