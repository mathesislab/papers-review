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
