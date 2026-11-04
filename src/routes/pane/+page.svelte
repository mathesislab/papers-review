<script lang="ts">
  // The review pane: a web view of its own over the viewer of a project's window (src-tauri/src/pane.rs), at the
  // bottom or on the right. Its inner edge, dragged, sets its size; ⌘J shows or hides it, ⌘⇧J moves it. Collapsed
  // by its arrow, it is its header bar alone: along the bottom, or a strip down the right side with the name written
  // upright along it.
  //
  // The review, a Markdown file of the project, is edited in a FileEditor: written a moment after the typing stops,
  // and as the pane loses the keys; read again when changed elsewhere (an editor, a sync client), the pane asking
  // which to keep when changes of its own are not yet written. A review file that is not there is not made as one
  // types: the pane says so, with a button to make it.
  //
  // ⇧⌘R lists the references of the project over the editor (RefList), to cite one at the cursor; the page sends the
  // text of a reference selected in the PDF, if there is one.
  //
  // What is shown, a round (its tab) or all of the text, is exported from the button of the header or the File menu:
  // as a PDF, saved in the project and shown in the Finder, or as plain text, copied and saved (src/lib/export.ts).

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "$lib/events";
  import { Menu, MenuItem } from "@tauri-apps/api/menu";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { keymap } from "@codemirror/view";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import { PREFS, insertQuote, insertSymbol, prefsFrom, setEditorFont, showSection, wordCount, wordsOf } from "$lib/editor";
  import EditorButtons from "$lib/EditorButtons.svelte";
  import { formatDate } from "$lib/due";
  import { exportName, pdfDocument, section, textDocument, toHtml, toPlainText, type Frame } from "$lib/export";
  import FileEditor, { type Status } from "$lib/FileEditor.svelte";
  import RefList from "$lib/RefList.svelte";
  import { sections, type Section } from "$lib/sections";
  import type { ProjectInfo, Settings } from "$lib/types";

  type Side = "bottom" | "right";
  type Placed = { side: Side; size: number; collapsed: boolean };
  type Review = { path: string; name: string; text: string; exists: boolean };

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
  /** the words selected */
  let selected = $state(0);
  let status = $state<Status>("saved");
  let error = $state("");
  /** the review file is not there (yet): said instead of the editor */
  let missing = $state(false);
  let createError = $state("");
  /** the references listed over the editor; `opened` counts the times, to start the list afresh each time */
  let listing = $state(false);
  let opened = $state(0);
  /** the reference selected in the PDF when it was opened */
  let reference = $state("");
  let refs = $state<RefList>();
  /** the user's own sequences typed that become symbols (the settings) */
  let symbols = $state<[string, string][]>([]);
  /** what the editor takes from the settings */
  let prefs = $state(PREFS);
  /** the sections, as tabs, and the one shown alone (null: all of the text) */
  let tabs = $state<Section[]>([]);
  let active = $state<number | null>(null);
  /** what an export says: that it is done, or what failed */
  let note = $state("");
  let exportError = $state("");
  let exporting = false;
  let noteTimer: ReturnType<typeof setTimeout> | undefined;

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

  /** What an export is of: the tab shown, else the whole review. */
  const target = $derived((active === null ? undefined : tabs[active]?.title) ?? "Review");

  /** Today, as `2026-11-14`. */
  function today() {
    const now = new Date();
    return [now.getFullYear(), now.getMonth() + 1, now.getDate()].map((n, i) => String(n).padStart(i ? 2 : 4, "0")).join("-");
  }

  /** What is shown exported as a PDF in the project's `export` folder, shown in the Finder, or as plain text,
   *  copied and saved there. */
  async function exportAs(kind: "pdf" | "text") {
    if (exporting || missing) return;
    exporting = true;
    clearTimeout(noteTimer);
    [note, exportError] = ["", ""];
    try {
      await editor?.flush();
      const all = text();
      const markdown = section(all, active);
      if (!markdown.trim()) throw "There is nothing to export";
      const name = exportName(path, active === null ? null : (sections(all)[active]?.title ?? null));
      const [info, settings] = await Promise.all([invoke<ProjectInfo>("project_info"), invoke<Settings>("settings")]);
      const frame: Frame = {
        heading: info.heading ?? "",
        details: settings.header_details ? [info.journal, info.manuscript, formatDate(today(), settings.date_format)] : [],
        signature: settings.sign ? settings.reviewer_name : null,
      };
      if (kind === "text") {
        await invoke("export_text", { name, text: textDocument({ ...frame, body: toPlainText(markdown) }) });
        note = "copied";
        noteTimer = setTimeout(() => (note = ""), 3000);
        return;
      }
      const html = pdfDocument({ ...frame, body: toHtml(markdown) });
      await revealItemInDir(await invoke<string>("export_pdf", { name, html }));
    } catch (e) {
      exportError = String(e);
    } finally {
      exporting = false;
    }
  }

  /** The menu of the export button. */
  async function exportMenu() {
    const items = await Promise.all([
      MenuItem.new({ text: `Export ${target} as PDF`, action: () => exportAs("pdf") }),
      MenuItem.new({ text: `Copy ${target} as Plain Text`, action: () => exportAs("text") }),
    ]);
    await (await Menu.new({ items })).popup();
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

  /** A click on the header, not on a button or tab: the pane collapsed, or expanded again. */
  function toggle(e: MouseEvent) {
    if (!(e.target as Element).closest("button")) invoke("pane_collapse", { collapsed: !collapsed });
  }

  const text = () => editor?.view()?.state.doc.toString() ?? "";

  const load = async () => {
    const review = await invoke<Review>("review");
    missing = !review.exists;
    return review;
  };
  const save = (path: string, text: string) => invoke<void>("save_review", { path, text });

  /** Escape: the keys back to the PDF, the pane left as it is. */
  const release = keymap.of([{ key: "Escape", run: () => (invoke("pane_release"), true) }]);
  const more = [release];

  /** The references listed, closest to the text of a reference selected in the PDF if there is one. Asked again
   *  without a text, the list that is open stays as it is. */
  function references(text: string) {
    if (listing && !text) return refs?.focus();
    [reference, listing] = [text, true];
    opened++;
  }

  /** A citation or reference chosen: at the cursor. */
  function cited(text: string) {
    const view = editor?.view();
    if (!view) return;
    listing = false;
    insertSymbol(view, text);
  }

  /** The PDF of a paper chosen: in a tab of the project, the keys to the page. */
  function openPdf(path: string) {
    listing = false;
    invoke("open_reference_pdf", { path });
    invoke("pane_release");
  }

  /** The list closed: the keys to the editor. */
  function closeList() {
    listing = false;
    editor?.focus();
  }

  /** The review file made, empty, and read into the editor. */
  async function create() {
    createError = "";
    try {
      await invoke("create_review");
      await editor?.reload();
      editor?.focus();
    } catch (e) {
      createError = String(e);
    }
  }

  const loadSettings = () =>
    invoke<Settings>("settings").then((s) => {
      symbols = s.symbols;
      prefs = prefsFrom(s);
      setEditorFont(s.editor_font, s.editor_size);
    });

  onMount(() => {
    // placed before this page listened, maybe
    invoke<Placed | null>("pane_placement").then((placed) => {
      if (placed) [side, size, collapsed] = [placed.side, placed.size, placed.collapsed];
    });
    loadSettings();
    const unlisteners = [
      listen("settings-changed", loadSettings),
      listen<Placed>("pane-placed", (e) => {
        side = e.payload.side;
        size = e.payload.size;
        collapsed = e.payload.collapsed;
      }),
      listen("review-changed", () => editor?.changedOnDisk()),
      // Export as PDF and Copy as Plain Text of the File menu
      listen<"pdf" | "text">("export", (e) => exportAs(e.payload)),
      // Undo, Redo, Symbols…, Back and Forward of the menu: the editor's own history, its list of symbols, its indent
      listen<string>("edit", (e) => {
        if (e.payload === "symbols") editor?.list();
        else if (e.payload === "back") editor?.outdent();
        else if (e.payload === "forward") editor?.indent();
        else if (e.payload === "redo") editor?.redo();
        else editor?.undo();
      }),
      // ⌘U and ⌘Y in the page: a quote of the PDF at the cursor, in quotation marks or not
      listen<string>("review-insert", (e) => {
        const view = editor?.view();
        if (view) insertQuote(view, e.payload);
      }),
      // ⇧⌘R in the page: the references, with the text of the reference selected in the PDF
      listen<{ text: string }>("references", (e) => references(e.payload.text)),
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
      if (listing) refs?.focus();
      else editor?.focus();
    };
    const blurred = () => {
      invoke("pane_focus", { focused: false });
      editor?.flush();
    };
    // a click or a field in the pane: the keys are the pane's, when the window's focus does not say so (the keys
    // coming from the page of the same window)
    const toPane = () => invoke("pane_focus", { focused: true });
    window.addEventListener("focus", focused);
    window.addEventListener("blur", blurred);
    document.addEventListener("pointerdown", toPane, true);
    document.addEventListener("focusin", toPane);
    return () => {
      for (const u of unlisteners) u.then((f) => f());
      window.removeEventListener("focus", focused);
      window.removeEventListener("blur", blurred);
      document.removeEventListener("pointerdown", toPane, true);
      document.removeEventListener("focusin", toPane);
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
  <header class:strip={collapsed && side === "right"} onclick={toggle} role="presentation">
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
      {#if !missing}
        <span class="meta">
          {wordCount(words, selected)} ·
          {#if status === "error"}
            <span class="error" title={error}>not saved</span>
          {:else if exportError}
            <span class="error" title={exportError}>not exported</span>
          {:else if note}
            {note}
          {:else}
            {status === "saved" ? "saved" : "editing"}
          {/if}
        </span>
      {:else}
        <span class="meta"></span>
      {/if}
      {#if !missing}
        <EditorButtons symbols={true} lineNumbers={prefs.lineNumbers} onsymbols={() => editor?.list()} />
      {/if}
      <button class="export" onclick={exportMenu} disabled={missing} title="Export {target}" aria-label="Export {target}">
        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
          <path d="M7 1.5v6.5M4.5 5.5 7 8l2.5-2.5M2 8.5v3a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1v-3" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
      <button onclick={() => invoke("pane_side")} title={side === "bottom" ? "To the Right (⌘⇧J)" : "To the Bottom (⌘⇧J)"}>
        {side === "bottom" ? "⇥" : "⤓"}
      </button>
      <button onclick={() => invoke("pane_hide")} title="Hide (⌘J)">✕</button>
      {@render arrow()}
    {/if}
  </header>
  <main>
    <!-- kept while the file is missing, to read it into when it is made -->
    <div class="editor" class:hidden={missing}>
      <FileEditor bind:this={editor} {load} {save} {symbols} {prefs} extensions={more} onchange={count} bind:status bind:error bind:path bind:selected />
    </div>
    {#if listing}
      {#key opened}
        <RefList bind:this={refs} text={reference} oninsert={cited} onopen={openPdf} onclose={closeList} />
      {/key}
    {/if}
    {#if missing}
      <div class="missing">
        <p>No review file yet</p>
        <button class="make" onclick={create}>Create {name || "review.md"}</button>
        {#if createError}<p class="error">{createError}</p>{/if}
      </div>
    {/if}
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
    position: relative;
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
  button:disabled {
    opacity: 0.4;
  }
  button:disabled:hover {
    background: none;
  }
  .export {
    display: flex;
    padding: 4px 6px;
  }
  .arrow {
    display: flex;
    padding: 4px 6px;
  }
  /* in the middle of the header, not among the buttons at its end; the strip on the right keeps it at its top */
  header:not(.strip) .arrow {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
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
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .editor {
    height: 100%;
  }
  .editor.hidden {
    visibility: hidden;
  }
  .missing {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
  }
  .missing p {
    margin: 0;
  }
  .missing .error {
    color: var(--error);
  }
  .make {
    color: var(--fg);
    border: 1px solid var(--border);
    padding: 3px 12px;
  }
  .collapsed main {
    display: none;
  }
</style>
