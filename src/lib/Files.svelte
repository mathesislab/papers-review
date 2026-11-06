<script lang="ts">
  // The project in a box at the top (its name, journal and manuscript, due date; a click opens its info), then the files
  // of the project's folder and those linked from elsewhere (FileTree of papers-write/project). A click opens a file in
  // a tab; folders open and close. A Markdown file, right-clicked, can be the review (the pane's), marked with a pill.
  // Right-clicked, the project's folder and its folders make a Markdown file or take files in; a file or folder can be
  // shown in the Finder, opened in the default app, removed from the project (a link) or moved to the Trash. What is
  // done is reported to the page.

  import { Menu, MenuItem } from "@tauri-apps/api/menu";
  import { FileTree } from "papers-write/project";
  import { daysLeft, daysText, formatDate, urgency } from "./due";
  import type { DateFormat, Node } from "./types";

  let {
    root,
    name,
    journal,
    manuscript,
    due,
    dateFormat,
    files,
    linked,
    front,
    open = $bindable([]),
    review,
    onopen,
    onreview,
    oninfo,
    onnew,
    onadd,
    onreveal,
    ondefault,
    onunlink,
    ontrash,
  }: {
    /** the path of the project folder */
    root: string;
    /** the name of the project: its title, else the name of its folder */
    name: string;
    journal: string | null;
    manuscript: string | null;
    due: string | null;
    /** how a date is written */
    dateFormat: DateFormat;
    files: Node[];
    linked: Node[];
    /** the file in front */
    front: string | null;
    /** the folders open in the tree (none at first) */
    open?: string[];
    /** the file of the review */
    review: string;
    onopen: (node: Node) => void;
    /** a Markdown file to be the review */
    onreview: (node: Node) => void;
    /** the project's info to show */
    oninfo: () => void;
    /** a Markdown file of this name to make in a folder of the project */
    onnew: (folder: string, name: string) => void;
    /** files, or folders, to add to a folder of the project */
    onadd: (folder: string, directory: boolean) => void;
    /** a path to show in the Finder */
    onreveal: (path: string) => void;
    /** a file to open in the default app */
    ondefault: (path: string) => void;
    /** a linked file or folder to take out of the project (the file stays where it is) */
    onunlink: (node: Node) => void;
    /** a file or folder of the project to move to the Trash */
    ontrash: (node: Node) => void;
  } = $props();

  /** A right-click on the project's box. */
  async function boxMenu(e: MouseEvent) {
    e.preventDefault();
    await (await Menu.new({ items: [await MenuItem.new({ text: "Project Info…", action: oninfo })] })).popup();
  }

  /** the journal and the manuscript ID, as far as they are set */
  const where = $derived([journal, manuscript].filter(Boolean).join(" · "));
</script>

<nav>
  <!-- the project: a click on the box, or on the button at its corner, shows its info -->
  <div class="project" oncontextmenu={boxMenu} role="presentation">
    <button class="summary" title="{root}: Project Info (⌘I)" onclick={oninfo}>
      <span class="title">{name}</span>
      {#if where}<span class="sub">{where}</span>{/if}
      {#if due}
        {@const days = daysLeft(due)}
        <span class="sub due"><span class="dot {urgency(days)}"></span>{formatDate(due, dateFormat)} · {daysText(days)}</span>
      {/if}
    </button>
    <button class="info" title="Project Info (⌘I)" aria-label="Project Info" onclick={oninfo}>
      <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
        <circle cx="8" cy="8" r="6.5" />
        <line x1="8" y1="7.2" x2="8" y2="11.3" stroke-linecap="round" />
        <circle cx="8" cy="4.9" r="0.4" fill="currentColor" />
      </svg>
    </button>
  </div>
  <div class="scroll">
    <FileTree
      {root}
      {files}
      {linked}
      {front}
      bind:open
      badge={(node) => (node.path === review ? { label: "review", title: "The review (⌘J)" } : null)}
      extra={(node) => (node.kind === "markdown" ? [{ text: "Edit in Review Pane", action: () => onreview(node) }] : [])}
      {onopen}
      {onnew}
      {onadd}
      {onreveal}
      {ondefault}
      {onunlink}
      {ontrash}
    />
  </div>
</nav>

<style>
  nav {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--sidebar);
    border-right: 1px solid var(--line);
  }
  .scroll {
    flex: 1;
    overflow: auto;
  }
  /* the project's box at the top */
  .project {
    position: relative;
    flex: none;
    margin: 8px 6px 0;
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .project:hover {
    background: color-mix(in srgb, var(--bg) 94%, var(--fg));
  }
  .summary {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    box-sizing: border-box;
    padding: 8px 30px 8px 10px;
    font: inherit;
    color: inherit;
    text-align: left;
    background: none;
    border: none;
    border-radius: 8px;
  }
  .title {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .due {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  .dot.far {
    background: var(--due-far);
  }
  .dot.near {
    background: var(--due-near);
  }
  .dot.close {
    background: var(--due-close);
  }
  .dot.due {
    background: var(--due-due);
  }
  .info {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    padding: 2px;
    color: var(--muted);
    background: none;
    border: none;
    border-radius: 50%;
  }
  .info:hover {
    color: var(--fg);
  }
</style>
