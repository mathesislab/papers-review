<script lang="ts">
  // The review pane: a web view of its own over the viewer of a project's window (src-tauri/src/pane.rs), along a
  // side of it (in the middle at the top or the bottom, as tall as the viewer on the left or the right) or free. Its
  // header dragged moves it, free; its edges and corners dragged set its size (at the top or the bottom its sides
  // widen it on both sides). The menu of its header, or ⌘⇧J, puts it in another place; ⌘J shows or hides it.
  // Collapsed by its arrow or a click on its header, it is its header bar alone, where it lies, or a strip down the
  // left or right side with the name written upright along it.
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
  import { CheckMenuItem, Menu, MenuItem } from "@tauri-apps/api/menu";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { keymap } from "@codemirror/view";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import { FileEditor, RefList, PREFS, insertQuote, insertSymbol, prefsFrom, sections, setEditorFont, showSection, wordCount, wordsOf, type Found, type Section, type Status } from "papers-write/editor";
  import EditorButtons from "$lib/EditorButtons.svelte";
  import { formatDate } from "$lib/due";
  import { exportName, pdfDocument, section, textDocument, toHtml, toPlainText, type Frame } from "$lib/export";
  import { leads } from "$lib/leads";
  import type { ProjectInfo, Settings } from "$lib/types";

  type Dock = "bottom" | "right" | "top" | "left" | "free";
  /** what of the pane is dragged: its header (`move`), or an edge or a corner, by the compass */
  type Grip = "move" | "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
  /** what the review pane shows while the review is empty */
  const PLACEHOLDER = "Your review, in Markdown: ## for its sections, a section a round, say.";
  type Placed = { dock: Dock; collapsed: boolean };
  type Review = { path: string; name: string; text: string; exists: boolean };

  let dock = $state<Dock>("bottom");
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
      const sectioned = active === null ? undefined : sections(all)[active];
      const markdown = section(all, sectioned);
      if (!markdown.trim()) throw "There is nothing to export";
      const name = exportName(path, active === null ? null : (sectioned?.title ?? null));
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

  /** The edges and corners of the pane that are dragged, where it lies: those away from the side of the viewer. */
  const GRIPS: Record<Dock, Grip[]> = {
    bottom: ["n", "w", "e", "nw", "ne"],
    top: ["s", "w", "e", "sw", "se"],
    right: ["w"],
    left: ["e"],
    free: ["n", "s", "e", "w", "ne", "nw", "se", "sw"],
  };
  /** How far the header is dragged before it moves the pane rather than being a click (points). */
  const SLOP = 4;

  /** `grip` dragged from `e`: the app follows by how far, in points of the screen (the pane moves under the pointer,
   *  the screen does not). Let go without having gone `SLOP` (the header only), `onclick` instead. */
  function drag(grip: Grip, e: PointerEvent, onclick?: () => void) {
    if (e.button !== 0) return;
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    const [startX, startY] = [e.screenX, e.screenY];
    let [moved, frame] = [!onclick, 0];
    const by = (m: PointerEvent) => ({ grip, dx: m.screenX - startX, dy: m.screenY - startY });
    const move = (m: PointerEvent) => {
      moved ||= Math.hypot(m.screenX - startX, m.screenY - startY) >= SLOP;
      if (!moved) return;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => invoke("pane_drag", { ...by(m), done: false }));
    };
    const up = (m: PointerEvent) => {
      cancelAnimationFrame(frame);
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", up);
      if (moved) invoke("pane_drag", { ...by(m), done: true });
      else onclick?.();
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", up);
  }

  /** The header pressed, not on a button or tab: dragged, the pane moved (free); clicked, collapsed or expanded. */
  function pressHeader(e: PointerEvent) {
    if ((e.target as Element).closest("button")) return;
    drag("move", e, () => invoke("pane_collapse", { collapsed: !collapsed }));
  }

  const PLACES: [Dock, string][] = [
    ["bottom", "At the Bottom"],
    ["right", "On the Right"],
    ["top", "At the Top"],
    ["left", "On the Left"],
    ["free", "Free"],
  ];

  /** The menu of the place button: where the pane lies, the one it is in checked. */
  async function placeMenu() {
    const items = await Promise.all(
      PLACES.map(([to, text]) => CheckMenuItem.new({ text, checked: to === dock, action: () => void invoke("pane_dock", { dock: to }) })),
    );
    await (await Menu.new({ items })).popup();
  }

  /** The arrow of the collapse button: toward where the pane goes as it collapses (its side), back as it opens. */
  const COLLAPSES: Record<Dock, string> = { bottom: "down", top: "up", free: "up", right: "right", left: "left" };
  const OPPOSITE: Record<string, string> = { down: "up", up: "down", right: "left", left: "right" };
  const arrowWay = $derived(collapsed ? OPPOSITE[COLLAPSES[dock]] : COLLAPSES[dock]);
  /** collapsed on the left or the right: a strip down that side */
  const strip = $derived(collapsed && (dock === "left" || dock === "right"));

  const text = () => editor?.view()?.state.doc.toString() ?? "";

  const load = async () => {
    const review = await invoke<Review>("review");
    missing = !review.exists;
    return review;
  };
  /** the references, for the list: the app's commands */
  const search = (query: string, limit: number) => invoke<Found>("references", { query, limit });
  const closest = (text: string, limit: number) => invoke<Found>("closest_references", { text, limit });
  const sources = () => invoke<string[]>("reference_sources");
  const save = (path: string, text: string) => invoke<void>("save_review", { path, text });

  /** Escape: the keys back to the PDF, the pane left as it is. */
  const release = keymap.of([{ key: "Escape", run: () => (invoke("pane_release"), true) }]);
  const more = [release, ...leads];

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
      if (placed) [dock, collapsed] = [placed.dock, placed.collapsed];
    });
    loadSettings();
    const unlisteners = [
      listen("settings-changed", loadSettings),
      listen<Placed>("pane-placed", (e) => {
        dock = e.payload.dock;
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
      // ⌘. of the menu (Cite as [@key]): the paper marked in the list, as [@key]
      listen("cite-key", () => refs?.cite("pandoc")),
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
    <svg class={arrowWay} width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <polyline points="2.5,4.5 6,8 9.5,4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>
{/snippet}

<div class="pane" class:collapsed>
  {#if !collapsed}
    {#each GRIPS[dock] as grip (grip)}
      <div class="grip {grip}" onpointerdown={(e) => drag(grip, e)} role="separator" aria-orientation={grip === "n" || grip === "s" ? "horizontal" : "vertical"}></div>
    {/each}
  {/if}
  <header class:strip onpointerdown={pressHeader} role="presentation">
    {#if strip}
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
      <button class="place" onclick={placeMenu} title="Move (⌘⇧J)" aria-label="Move">
        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
          <rect x="1.5" y="2" width="11" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" />
          <rect x="4" y="7.5" width="6" height="2.5" rx="0.5" fill="currentColor" />
        </svg>
      </button>
      <button onclick={() => invoke("pane_hide")} title="Hide (⌘J)">✕</button>
      {@render arrow()}
    {/if}
  </header>
  <main>
    <!-- kept while the file is missing, to read it into when it is made -->
    <div class="editor" class:hidden={missing}>
      <FileEditor bind:this={editor} {load} {save} {symbols} {prefs} extensions={more} placeholder={PLACEHOLDER} onchange={count} bind:status bind:error bind:path bind:selected />
    </div>
    {#if listing}
      {#key opened}
        <RefList bind:this={refs} text={reference} {search} {closest} {sources} oninsert={cited} onopen={openPdf} onclose={closeList} />
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
    container: pane / inline-size;
  }
  /* the edges, 6px in, and the corners, 12px square, over them */
  .grip {
    position: absolute;
    z-index: 2;
  }
  .n,
  .s {
    left: 0;
    right: 0;
    height: 6px;
    cursor: row-resize;
  }
  .e,
  .w {
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: col-resize;
  }
  .ne,
  .nw,
  .se,
  .sw {
    width: 12px;
    height: 12px;
    z-index: 3;
  }
  .n,
  .ne,
  .nw {
    top: 0;
  }
  .s,
  .se,
  .sw {
    bottom: 0;
  }
  .w,
  .nw,
  .sw {
    left: 0;
  }
  .e,
  .ne,
  .se {
    right: 0;
  }
  .nw,
  .se {
    cursor: nwse-resize;
  }
  .ne,
  .sw {
    cursor: nesw-resize;
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
  .export,
  .place {
    display: flex;
    padding: 4px 6px;
  }
  .arrow {
    display: flex;
    padding: 4px 6px;
  }
  /* in the middle of the header, not among the buttons at its end; a strip keeps it at its top */
  header:not(.strip) .arrow {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
  }
  /* a narrow pane: at the end, where it does not lie over the words and the buttons */
  @container pane (max-width: 640px) {
    header:not(.strip) .arrow {
      position: static;
      transform: none;
    }
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
