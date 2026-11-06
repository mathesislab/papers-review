// What the app tells the pages (mirrors src-tauri/src/lib.rs and state.rs).

import type { EditorFont, Indent } from "papers-write/editor";
import type { Colour, Kind, Node } from "papers-write/project";

// The types of the editor and of the references are papers-write's; kept importable from here.
export type { EditorFont, Found, Hit, Indent, Item, Span, YearCount } from "papers-write/editor";

// The colour of a project is papers-write's (mirrors review_core::Colour).
export type { Colour };

// A file of a project and what it is are papers-write's (mirrors review_core::Kind and Node, as write_core's).
export type { Kind, Node };

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
  /** what Tab puts in: two spaces, four, or a tab */
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
