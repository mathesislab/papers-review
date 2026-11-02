<script lang="ts">
  // The review pane: a web view of its own over the viewer of a project's window (src-tauri/src/pane.rs), at the
  // bottom or on the right. Its inner edge, dragged, sets its size; ⌘J shows or hides it, ⌘⇧J moves it.
  //
  // The review, a Markdown file of the project, is written a moment after the typing stops, and as the pane loses
  // the keys. Changed elsewhere (an editor, a sync client), it is read again; with changes of its own not yet
  // written, the pane asks which to keep.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { EditorView } from "@codemirror/view";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import { makeEditor, replaceText, showSection, wordsOf } from "$lib/editor";
  import { sections, type Section } from "$lib/sections";

  type Side = "bottom" | "right";
  type Review = { path: string; name: string; text: string };

  let side = $state<Side>("bottom");
  /** its height at the bottom, its width on the right (points) */
  let size = $state(280);

  let host: HTMLElement;
  let view: EditorView | undefined;
  /** the file read, and its text as last read or written */
  let path = $state("");
  let name = $state("");
  let written = "";
  let words = $state(0);
  let status = $state<"saved" | "editing" | "error">("saved");
  let error = $state("");
  /** changed elsewhere while changes of its own were not written */
  let conflict = $state(false);
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

  const text = () => view?.state.doc.toString() ?? "";

  /** The review read: into the editor, made the first time. */
  async function load() {
    try {
      const review = await invoke<Review>("review");
      [path, name, written, conflict] = [review.path, review.name, review.text, false];
      if (view) replaceText(view, review.text);
      else view = makeEditor(host, review.text, typed);
      count(review.text);
      status = "saved";
    } catch (e) {
      [status, error] = ["error", String(e)];
    }
  }

  let timer: ReturnType<typeof setTimeout> | undefined;

  function typed(now: string) {
    count(now);
    status = "editing";
    clearTimeout(timer);
    timer = setTimeout(save, 500);
  }

  /** What was typed, written to the file it was read from. */
  async function save() {
    clearTimeout(timer);
    const now = text();
    if (!view || now === written || conflict) return;
    try {
      await invoke("save_review", { path, text: now });
      written = now;
      if (text() === now) status = "saved";
    } catch (e) {
      [status, error] = ["error", String(e)];
    }
  }

  /** The file changed on disk: read again, unless it is what was written, or changes typed are not written yet. */
  async function changedOnDisk() {
    const review = await invoke<Review>("review");
    if (review.path !== path || review.text === written) return;
    if (text() === written) await load();
    else conflict = true;
  }

  onMount(() => {
    load();
    // placed before this page listened, maybe
    invoke<{ side: Side; size: number } | null>("pane_placement").then((placed) => {
      if (placed) [side, size] = [placed.side, placed.size];
    });
    const unlisteners = [
      listen<{ side: Side; size: number }>("pane-placed", (e) => {
        side = e.payload.side;
        size = e.payload.size;
      }),
      listen("review-changed", changedOnDisk),
      // another file as the review: what was typed goes to the one before, then the new one is read
      listen("review-switched", async () => {
        await save();
        show(null);
        await load();
      }),
    ];
    // ⌘J hides the pane when it has the keys, else brings it forward with them
    const focused = () => {
      invoke("pane_focus", { focused: true });
      view?.focus();
    };
    const blurred = () => {
      invoke("pane_focus", { focused: false });
      save();
    };
    window.addEventListener("focus", focused);
    window.addEventListener("blur", blurred);
    return () => {
      for (const u of unlisteners) u.then((f) => f());
      window.removeEventListener("focus", focused);
      window.removeEventListener("blur", blurred);
      view?.destroy();
    };
  });
</script>

<div class="pane {side}">
  <div class="edge" onpointerdown={drag} role="separator" aria-orientation={side === "bottom" ? "horizontal" : "vertical"}></div>
  <header>
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
  </header>
  {#if conflict}
    <div class="conflict">
      <span>{name} changed on disk.</span>
      <button onclick={load}>Reload</button>
      <button
        onclick={() => {
          conflict = false;
          save();
        }}>Keep mine</button
      >
    </div>
  {/if}
  <main bind:this={host}></main>
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
    padding: 6px 8px 6px 14px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
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
  .conflict {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 14px;
    background: var(--chip);
    border-bottom: 1px solid var(--line);
  }
  .conflict button {
    color: var(--fg);
    border: 1px solid var(--border);
  }
  main {
    flex: 1;
    min-height: 0;
  }
</style>
