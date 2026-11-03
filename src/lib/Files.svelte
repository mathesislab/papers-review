<script lang="ts">
  // The files of the project, as a tree (folders first) under the project's name, then the files linked from elsewhere
  // (each with the folder it is in), then the due date. A click opens a file in a tab; folders open and close. A Markdown file, right-clicked, can be the review (the
  // pane's), marked ✎.

  import { Menu, MenuItem } from "@tauri-apps/api/menu";
  import { dueText } from "./due";
  import type { Kind, Node } from "./types";

  let {
    name,
    files,
    linked,
    due,
    front,
    review,
    onopen,
    onreview,
  }: {
    /** the name of the project folder */
    name: string;
    files: Node[];
    linked: Node[];
    due: string | null;
    /** the file in front */
    front: string | null;
    /** the file of the review */
    review: string;
    onopen: (node: Node) => void;
    /** a Markdown file to be the review */
    onreview: (node: Node) => void;
  } = $props();

  async function menu(e: MouseEvent, node: Node) {
    if (node.kind !== "markdown") return;
    e.preventDefault();
    const item = await MenuItem.new({ text: "Edit in Review Pane", action: () => onreview(node) });
    await (await Menu.new({ items: [item] })).popup();
  }

  /** the folders closed (all open at first) */
  let closed = $state(new Set<string>());

  function toggle(folder: Node) {
    const next = new Set(closed);
    if (next.has(folder.path)) next.delete(folder.path);
    else next.add(folder.path);
    closed = next;
  }

  /** The folder holding a path, with the home folder as ~ (empty for a path at the root). */
  function parent(path: string): string {
    const dir = path.slice(0, path.lastIndexOf("/"));
    return dir.replace(/^\/Users\/[^/]+/, "~");
  }

  const LABELS: Record<Kind, string> = {
    folder: "",
    pdf: "PDF",
    markdown: "MD",
    text: "TXT",
    bib: "BIB",
    other: "",
  };
</script>

{#snippet chevron(open: boolean)}
  <svg class="chevron" class:open viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
    <path d="M4 2.25 7.75 6 4 9.75" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{/snippet}

{#snippet folderIcon()}
  <svg class="glyph" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
    <path
      d="M1.75 4.5A1.25 1.25 0 0 1 3 3.25h2.9c.33 0 .64.14.86.38l.78.87h4.46A1.25 1.25 0 0 1 13.25 5.75v5.5A1.25 1.25 0 0 1 12 12.5H3a1.25 1.25 0 0 1-1.25-1.25z"
      fill="currentColor"
      opacity="0.85"
    />
  </svg>
{/snippet}

{#snippet linkIcon()}
  <svg class="glyph link" viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
    <path
      d="M6.75 9.25a2.5 2.5 0 0 0 3.54 0l2.2-2.2a2.5 2.5 0 0 0-3.54-3.54l-.6.6M9.25 6.75a2.5 2.5 0 0 0-3.54 0l-2.2 2.2a2.5 2.5 0 0 0 3.54 3.54l.6-.6"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
    />
  </svg>
{/snippet}

<!-- The nodes at one level; a level in a folder sits in a box that draws the guide line under its folder. A linked
     node at the top is marked with a link glyph and the folder it comes from. -->
{#snippet tree(nodes: Node[], depth: number, outside: boolean)}
  {#each nodes as node (node.path)}
    {@const from = outside && depth === 0 ? parent(node.path) : ""}
    {#if node.kind === "folder"}
      {@const empty = !node.children?.length}
      {@const open = !empty && !closed.has(node.path)}
      <button class="item" class:empty onclick={() => !empty && toggle(node)} title={node.path}>
        <span class="disclosure">{#if !empty}{@render chevron(open)}{/if}</span>
        {@render folderIcon()}
        {#if from}{@render linkIcon()}{/if}
        <span class="name">{node.name}</span>
        {#if from}<span class="from">{from}</span>{/if}
      </button>
      {#if open && node.children}
        <div class="children" style:--depth={depth + 1}>
          {@render tree(node.children, depth + 1, outside)}
        </div>
      {/if}
    {:else}
      <button
        class="item"
        class:front={node.path === front}
        class:missing={node.missing}
        onclick={() => onopen(node)}
        oncontextmenu={(e) => menu(e, node)}
        disabled={node.missing}
        title={node.missing ? `${node.path}: not found` : node.path}
      >
        <span class="disclosure"></span>
        <span class="kind {node.kind}">{LABELS[node.kind]}</span>
        {#if from}{@render linkIcon()}{/if}
        <span class="name">{node.name}</span>
        {#if from}<span class="from">{from}</span>{/if}
        {#if node.path === review}<span class="review" title="The review (⌘J)">✎</span>{/if}
      </button>
    {/if}
  {/each}
{/snippet}

<nav>
  <div class="scroll">
    <div class="project" title={name}>
      {@render folderIcon()}
      <span class="name">{name}</span>
    </div>
    <div class="level" style:--depth={0}>
      {@render tree(files, 0, false)}
    </div>
    {#if linked.length}
      <h4>Linked</h4>
      <div class="level" style:--depth={0}>
        {@render tree(linked, 0, true)}
      </div>
    {/if}
  </div>
  {#if due}<footer>{dueText(due)}</footer>{/if}
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
    padding: 6px 6px 10px;
  }
  .project {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 2px 0 3px;
    padding: 3px 8px;
    font-weight: 600;
    white-space: nowrap;
  }
  .project .glyph {
    color: var(--accent);
  }
  h4 {
    margin: 12px 8px 3px;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    font: inherit;
    color: inherit;
    text-align: left;
    background: none;
    border: none;
    border-radius: 5px;
    padding: 3px 8px 3px calc(8px + var(--depth) * 14px);
    white-space: nowrap;
  }
  .item:hover:not(:disabled) {
    background: var(--chip);
  }
  .item.front {
    background: var(--selection);
  }
  .item.missing {
    opacity: 0.5;
  }
  /* a folder's contents: a guide line under the folder's chevron spans them */
  .children {
    position: relative;
  }
  .children::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 2px;
    left: calc(8px + 9px + (var(--depth) - 1) * 14px - 0.5px);
    border-left: 1px solid var(--line);
    pointer-events: none;
  }
  .disclosure {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    flex: none;
    border-radius: 4px;
    color: var(--muted);
  }
  .item:hover:not(:disabled) .disclosure:not(:empty) {
    background: color-mix(in srgb, var(--fg) 12%, transparent);
    color: var(--fg);
  }
  .chevron {
    transition: transform 0.12s ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .glyph {
    flex: none;
    color: var(--muted);
  }
  .item.empty .name {
    color: var(--muted);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* the folder a linked node is in, dimmed and cut short first */
  .from {
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
    opacity: 0.75;
    font-size: 11px;
  }
  .review {
    margin-left: auto;
    color: var(--muted);
  }
  .kind {
    flex: none;
    width: 26px;
    font: 600 8.5px/1 -apple-system, sans-serif;
    padding: 3px 0;
    border-radius: 3px;
    text-align: center;
    color: #fff;
    background: #9a9aa2;
  }
  .kind:empty {
    background: none;
  }
  .kind.pdf {
    background: #d04545;
  }
  .kind.markdown {
    background: #4a76d6;
  }
  .kind.bib {
    background: #2f9e6e;
  }
  footer {
    padding: 8px 14px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    white-space: nowrap;
  }
</style>
