<script lang="ts">
  // The review pane: a web view of its own over the viewer of a project's window (src-tauri/src/pane.rs), at the
  // bottom or on the right. Its inner edge, dragged, sets its size; ⌘J shows or hides it, ⌘⇧J moves it. Collapsed
  // by its arrow, it is its header bar alone: along the bottom, or a strip down the right side with the name written
  // upright along it.
  //
  // The review, a Markdown file of the project, is edited in a FileEditor: written a moment after the typing stops,
  // and as the pane loses the keys; read again when changed elsewhere (an editor, a sync client), the pane asking
  // which to keep when changes of its own are not yet written.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { keymap } from "@codemirror/view";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import { insertQuote, insertText, showSection, wordsOf } from "$lib/editor";
  import FileEditor, { type Status } from "$lib/FileEditor.svelte";
  import { sections, type Section } from "$lib/sections";

  type Side = "bottom" | "right";
  type Placed = { side: Side; size: number; collapsed: boolean };
  type Review = { path: string; name: string; text: string };

  let side = $state<Side>("bottom");
  /** its height at the bottom, its width on the right (points) */
  let size = $state(280);
  /** only its header bar shown */
  let collapsed = $state(false);

  let editor = $state<FileEditor>();
  /** the file read */
  let path = $state("");
  const name = $derived(path.split("/").pop() ?? "");
  let words = $state(0);
  let status = $state<Status>("saved");
  let error = $state("");
  /** the sections, as tabs, and the one shown alone (null: all of the text) */
  let tabs = $state<Section[]>([]);
  let active = $state<number | null>(null);

  /** The tabs and the words of what is shown, from the text. */
  function count(now: string) {
    tabs = sections(now);
    if (active !== null && active >= tabs.length) show(null);
    const section = active === null ? undefined : tabs[active];
    words = wordsOf(section ? now.slice(section.from, section.to) : now);
  }

  function show(index: number | null) {
    active = index;
    const view = editor?.view();
    if (view) showSection(view, index);
    count(text());
  }

  /** The inner edge dragged: the size follows, in points of the screen (the pane moves as it grows). */
  function drag(e: PointerEvent) {
    const edge = e.currentTarget as HTMLElement;
    edge.setPointerCapture(e.pointerId);
    const [startX, startY, start] = [e.screenX, e.screenY, size];
    let frame = 0;
    const to = (m: PointerEvent) => (side === "bottom" ? start - (m.screenY - startY) : start - (m.screenX - startX));
    const move = (m: PointerEvent) => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => invoke("pane_size", { size: to(m), done: false }));
    };
    const up = (m: PointerEvent) => {
      cancelAnimationFrame(frame);
      invoke("pane_size", { size: to(m), done: true });
      edge.removeEventListener("pointermove", move);
      edge.removeEventListener("pointerup", up);
    };
    edge.addEventListener("pointermove", move);
    edge.addEventListener("pointerup", up);
  }

  /** The collapsed header or strip clicked, but not on a button: the pane expands. */
  function expand(e: MouseEvent) {
    if (collapsed && !(e.target as Element).closest("button")) invoke("pane_collapse", { collapsed: false });
  }

  const text = () => editor?.view()?.state.doc.toString() ?? "";

  const load = () => invoke<Review>("review");
  const save = (path: string, text: string) => invoke<void>("save_review", { path, text });

  /** Escape: the keys back to the PDF, the pane left as it is. */
  const release = keymap.of([{ key: "Escape", run: () => (invoke("pane_release"), true) }]);
  const more = [release];

  onMount(() => {
    // placed before this page listened, maybe
    invoke<Placed | null>("pane_placement").then((placed) => {
      if (placed) [side, size, collapsed] = [placed.side, placed.size, placed.collapsed];
    });
    const unlisteners = [
      listen<Placed>("pane-placed", (e) => {
        side = e.payload.side;
        size = e.payload.size;
        collapsed = e.payload.collapsed;
      }),
      listen("review-changed", () => editor?.changedOnDisk()),
      // Undo and Redo of the menu: the editor's own history
      listen<string>("edit", (e) => (e.payload === "redo" ? editor?.redo() : editor?.undo())),
      // ⌘' and ⌘; in the page: a quote of the PDF, or its text alone, at the cursor
      listen<{ text: string; paragraph: boolean }>("review-insert", (e) => {
        const view = editor?.view();
        if (view) (e.payload.paragraph ? insertQuote : insertText)(view, e.payload.text);
      }),
      // another file as the review: what was typed goes to the one before, then the new one is read
      listen("review-switched", async () => {
        await editor?.flush();
        show(null);
        await editor?.reload();
      }),
    ];
    // ⌘J hides the pane when it has the keys, else brings it forward with them
    const focused = () => {
      invoke("pane_focus", { focused: true });
      editor?.focus();
    };
    const blurred = () => {
      invoke("pane_focus", { focused: false });
      editor?.flush();
    };
    window.addEventListener("focus", focused);
    window.addEventListener("blur", blurred);
    return () => {
      for (const u of unlisteners) u.then((f) => f());
      window.removeEventListener("focus", focused);
      window.removeEventListener("blur", blurred);
    };
  });
</script>

{#snippet arrow()}
  <button
    class="arrow"
    onclick={() => invoke("pane_collapse", { collapsed: !collapsed })}
    title={collapsed ? "Expand (⌘J)" : "Collapse"}
    aria-label={collapsed ? "Expand" : "Collapse"}
  >
    <svg class={side === "bottom" ? (collapsed ? "up" : "down") : collapsed ? "left" : "right"} width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <polyline points="2.5,4.5 6,8 9.5,4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>
{/snippet}

<div class="pane {side}" class:collapsed>
  {#if !collapsed}
    <div class="edge" onpointerdown={drag} role="separator" aria-orientation={side === "bottom" ? "horizontal" : "vertical"}></div>
  {/if}
  <header class:strip={collapsed && side === "right"} onclick={expand} role="presentation">
    {#if collapsed && side === "right"}
      {@render arrow()}
      <span class="name" title={path}>{name || "Review"}</span>
    {:else}
      <span class="name" title={path}>{name || "Review"}</span>
      {#if tabs.length}
        <nav class="tabs">
          <button class:on={active === null} onclick={() => show(null)}>All</button>
          {#each tabs as tab, i (i)}
            <button class:on={active === i} onclick={() => show(i)} title={tab.title}>{tab.title}</button>
          {/each}
        </nav>
      {/if}
      <span class="meta">
        {words} words ·
        {#if status === "error"}<span class="error" title={error}>not saved</span>{:else}{status === "saved" ? "saved" : "editing"}{/if}
      </span>
      <button onclick={() => invoke("pane_side")} title={side === "bottom" ? "To the Right (⌘⇧J)" : "To the Bottom (⌘⇧J)"}>
        {side === "bottom" ? "⇥" : "⤓"}
      </button>
      <button onclick={() => invoke("pane_hide")} title="Hide (⌘J)">✕</button>
      {@render arrow()}
    {/if}
  </header>
  <main>
    <FileEditor bind:this={editor} {load} {save} extensions={more} onchange={count} bind:status bind:error bind:path />
  </main>
</div>

<style>
  :global(html, body) {
    background: transparent;
  }
  .pane {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    /* square: the web view is not transparent, round corners would show its background */
    border: 1px solid var(--border);
    overflow: hidden;
    box-sizing: border-box;
  }
  .edge {
    position: absolute;
    z-index: 2;
  }
  .bottom .edge {
    top: 0;
    left: 0;
    right: 0;
    height: 6px;
    cursor: row-resize;
  }
  .right .edge {
    top: 0;
    bottom: 0;
    left: 0;
    width: 6px;
    cursor: col-resize;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    /* the height of the collapsed pane (HEADER in pane.rs) less the border of the pane, 1px at each side */
    height: calc(32px - 2px);
    box-sizing: border-box;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .collapsed header {
    border-bottom: none;
    cursor: pointer;
  }
  .collapsed header.strip {
    flex: 1;
    flex-direction: column;
    height: auto;
    min-height: 0;
    padding: 6px 0;
    gap: 10px;
  }
  .strip .name {
    min-height: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    writing-mode: vertical-rl;
  }
  .name {
    font-weight: 600;
  }
  .tabs {
    display: flex;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    background: var(--chip);
    border-radius: 7px;
    padding: 2px;
  }
  .tabs button {
    flex: none;
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
    padding: 1px 9px;
  }
  .tabs button.on {
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }
  .meta {
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
  }
  .error {
    color: var(--error);
  }
  button {
    font: inherit;
    color: var(--muted);
    background: none;
    border: none;
    border-radius: 5px;
    padding: 1px 6px;
  }
  button:hover {
    background: var(--chip);
  }
  .arrow {
    display: flex;
    padding: 4px 6px;
  }
  .arrow svg {
    transition: transform 0.12s;
  }
  .arrow .up {
    transform: rotate(180deg);
  }
  .arrow .right {
    transform: rotate(-90deg);
  }
  .arrow .left {
    transform: rotate(90deg);
  }
  main {
    flex: 1;
    min-height: 0;
  }
  .collapsed main {
    display: none;
  }
</style>
