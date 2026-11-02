<script lang="ts">
  // The files of the project, as a tree (folders first), then the files linked from elsewhere, then the due date.
  // A click opens a file in a tab; folders open and close. A Markdown file, right-clicked, can be the review (the
  // pane's), marked ✎.

  import { Menu, MenuItem } from "@tauri-apps/api/menu";
  import type { Kind, Node } from "./types";

  let {
    files,
    linked,
    due,
    front,
    review,
    onopen,
    onreview,
  }: {
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

  const LABELS: Record<Kind, string> = {
    folder: "",
    pdf: "PDF",
    markdown: "MD",
    text: "TXT",
    bib: "BIB",
    other: "",
  };

  /** The days left to the due date, said plainly. */
  function dueText(date: string): string {
    const days = Math.round((new Date(date).getTime() - Date.now()) / 86_400_000);
    const when = new Date(date).toLocaleDateString(undefined, { day: "numeric", month: "short" });
    if (days > 1) return `Due ${when} · in ${days} days`;
    if (days === 1) return `Due ${when} · tomorrow`;
    if (days === 0) return `Due ${when} · today`;
    return `Due ${when} · ${-days} days ago`;
  }
</script>

{#snippet tree(nodes: Node[], depth: number)}
  {#each nodes as node (node.path)}
    {#if node.kind === "folder"}
      <button class="item" style:padding-left="{8 + depth * 14}px" onclick={() => toggle(node)} title={node.path}>
        <span class="disclosure">{closed.has(node.path) ? "▸" : "▾"}</span>
        <span class="name">{node.name}</span>
      </button>
      {#if !closed.has(node.path) && node.children}
        {@render tree(node.children, depth + 1)}
      {/if}
    {:else}
      <button
        class="item"
        class:front={node.path === front}
        class:missing={node.missing}
        style:padding-left="{8 + depth * 14}px"
        onclick={() => onopen(node)}
        oncontextmenu={(e) => menu(e, node)}
        disabled={node.missing}
        title={node.missing ? `${node.path}: not found` : node.path}
      >
        <span class="disclosure"></span>
        <span class="kind {node.kind}">{LABELS[node.kind]}</span>
        <span class="name">{node.name}</span>
        {#if node.path === review}<span class="review" title="The review (⌘J)">✎</span>{/if}
      </button>
    {/if}
  {/each}
{/snippet}

<nav>
  <div class="scroll">
    {@render tree(files, 0)}
    {#if linked.length}
      <h4>Linked</h4>
      {@render tree(linked, 0)}
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
    padding: 3px 8px;
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
  .disclosure {
    width: 10px;
    flex: none;
    color: var(--muted);
    font-size: 10px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
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
