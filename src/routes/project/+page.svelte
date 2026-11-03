<script lang="ts">
  // The page of a project's window: the colour bar (right-click: its colour), the files (⌘⌥S hides them), the files
  // open in tabs, the one in front: a PDF in PDFKit, with its outline, page, zoom and find. How the window looked is
  // kept between sessions (the app's state.json); the commands of the menu bar come as `menu`.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { CheckMenuItem, Menu } from "@tauri-apps/api/menu";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import type { Entry } from "tauri-plugin-pdfkit-api";
  import "$lib/theme.css";
  import { COLOURS, cssOf } from "$lib/colour";
  import Files from "$lib/Files.svelte";
  import PdfView from "$lib/PdfView.svelte";
  import { quote } from "$lib/quote";
  import type { Colour, Layout, Node, Project } from "$lib/types";

  let project = $state<Project | null>(null);
  let error = $state("");
  let layout = $state<Layout>({ files: true, files_width: 240, tabs: [], front: null, pages: {} });
  /** the layout read: written back only after */
  let ready = false;

  /** every file of the project, by its path */
  const nodes = $derived.by(() => {
    const all = new Map<string, Node>();
    const add = (list: Node[]) => {
      for (const node of list) {
        all.set(node.path, node);
        if (node.children) add(node.children);
      }
    };
    if (project) {
      add(project.files);
      add(project.linked);
    }
    return all;
  });

  const front = $derived(layout.front ? nodes.get(layout.front) : undefined);

  /** the PDF in front, its page count and page in view */
  let pdf = $state<PdfView>();
  let count = $state(0);
  let current = $state(1);
  /** its outline (empty until it is open), and whether the panel for it is shown */
  let outline = $state<Entry[]>([]);
  let showOutline = $state(false);
  // the outline of the file before is not this one's
  $effect(() => {
    layout.front;
    outline = [];
  });
  /** the find field, and its text */
  let findInput = $state<HTMLInputElement>();
  /** the viewer: where it is tells the review pane where to lie */
  let viewer = $state<HTMLDivElement>();

  $effect(() => {
    if (!viewer) return;
    const area = viewer;
    const tell = () => {
      const box = area.getBoundingClientRect();
      invoke("pane_area", { area: { x: box.left, y: box.top, width: box.width, height: box.height } });
    };
    const observer = new ResizeObserver(tell);
    observer.observe(area);
    return () => observer.disconnect();
  });
  let findText = $state("");
  let notFound = $state(false);

  function turned(path: string, page: number) {
    layout.pages[path] = page;
    if (path === layout.front) current = page;
  }

  async function search(again: boolean, backwards = false) {
    if (!pdf) return;
    notFound = findText !== "" && !(await pdf.find(findText, again, backwards));
  }

  /** A right-click on the bar: the colour of the project, for all its windows (review.toml). */
  async function colourMenu(e: MouseEvent) {
    e.preventDefault();
    const items = COLOURS.map((c) =>
      CheckMenuItem.new({
        text: c.name,
        checked: project?.colour === c.colour,
        action: () => setColour(c.colour),
      }),
    );
    await (await Menu.new({ items: await Promise.all(items) })).popup();
  }

  async function setColour(colour: Colour) {
    try {
      await invoke("set_colour", { colour });
    } catch (e) {
      error = String(e);
    }
  }

  async function load() {
    try {
      project = await invoke<Project>("project");
    } catch (e) {
      error = String(e);
    }
  }

  /** A file to the front, in a tab of its own. */
  function open(node: Node) {
    if (!layout.tabs.includes(node.path)) layout.tabs.push(node.path);
    layout.front = node.path;
  }

  function close(path: string) {
    const i = layout.tabs.indexOf(path);
    if (i < 0) return;
    layout.tabs.splice(i, 1);
    if (layout.front === path) layout.front = layout.tabs[Math.min(i, layout.tabs.length - 1)] ?? null;
  }

  /** The narrowest and widest the files panel is dragged. */
  const FILES_WIDTH = { min: 160, max: 520 };

  /** The right edge of the files panel dragged: its width follows. The PDF lies over the page, but the drag stays
   *  with the edge: AppKit sends the moves to the view the press went to. */
  function resize(e: PointerEvent) {
    const edge = e.currentTarget as HTMLElement;
    edge.setPointerCapture(e.pointerId);
    const [start, width] = [e.clientX, layout.files_width];
    const move = (m: PointerEvent) => {
      layout.files_width = Math.round(Math.min(FILES_WIDTH.max, Math.max(FILES_WIDTH.min, width + m.clientX - start)));
    };
    const up = () => {
      edge.removeEventListener("pointermove", move);
      edge.removeEventListener("pointerup", up);
    };
    edge.addEventListener("pointermove", move);
    edge.addEventListener("pointerup", up);
  }

  /** A Markdown file as the review, in the pane. */
  async function editReview(path: string) {
    try {
      await invoke("set_review", { path });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  /** ⌘': the selection of the PDF as a quote, into the review at its cursor. */
  async function quoteSelection() {
    const selection = await pdf?.selection();
    if (!selection) return;
    try {
      await invoke("quote", { text: quote(selection) });
    } catch (e) {
      error = String(e);
    }
  }

  /** A command of the menu bar. */
  function command(id: string) {
    switch (id) {
      case "files":
        layout.files = !layout.files;
        break;
      case "close-tab":
        if (layout.front) close(layout.front);
        else getCurrentWindow().close();
        break;
      case "zoom-in":
        pdf?.zoom(1);
        break;
      case "zoom-out":
        pdf?.zoom(-1);
        break;
      case "zoom-fit":
        pdf?.zoom(0);
        break;
      case "find":
        findInput?.focus();
        findInput?.select();
        break;
      case "find-next":
        search(true);
        break;
      case "find-previous":
        search(true, true);
        break;
      case "quote":
        quoteSelection();
        break;
    }
  }

  onMount(() => {
    (async () => {
      await load();
      const saved = await invoke<Layout>("layout");
      // the files gone since are not opened again
      saved.tabs = saved.tabs.filter((t) => nodes.has(t));
      if (saved.front && !saved.tabs.includes(saved.front)) saved.front = saved.tabs[0] ?? null;
      layout = saved;
      ready = true;
    })();
    const unlisten = listen<string>("menu", (e) => command(e.payload));
    // the colour or the files changed, from this window or another of the project
    const unlistenChanged = listen("project-changed", () => load());
    // the review pane hidden: the keys back to the PDF
    const unlistenHidden = listen("pane-hidden", () => pdf?.focus());
    return () => {
      unlisten.then((f) => f());
      unlistenChanged.then((f) => f());
      unlistenHidden.then((f) => f());
    };
  });

  // the layout kept, a moment after it changes
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const snapshot = $state.snapshot(layout);
    if (!ready) return;
    clearTimeout(timer);
    timer = setTimeout(() => invoke("set_layout", { layout: snapshot }), 300);
  });
</script>

<!-- the project's colour, at the top, to tell its windows apart among others; a right-click chooses it -->
{#if project && cssOf(project.colour)}
  <div class="colour" style:background={cssOf(project.colour)} title="The colour of the project: right-click to choose" oncontextmenu={colourMenu} role="presentation"></div>
{/if}

{#if project}
  <div
    class="window"
    class:nofiles={!layout.files}
    class:coloured={cssOf(project.colour)}
    style:grid-template-columns={layout.files ? `${layout.files_width}px 1fr` : "1fr"}
  >
    {#if layout.files}
      <div class="files">
        <Files
          name={project.name}
          files={project.files}
          linked={project.linked}
          due={project.due}
          front={layout.front}
          review={project.review}
          onopen={open}
          onreview={(node) => editReview(node.path)}
        />
        <!-- its right edge, dragged, sets its width -->
        <div class="splitter" onpointerdown={resize} role="separator" aria-orientation="vertical"></div>
      </div>
    {/if}
    <section class="main">
      <div class="bar">
        <!-- the files panel, shown or hidden as by ⌘⌥S -->
        <button class="toggle" class:on={layout.files} onclick={() => command("files")} title="Show or Hide Files (⌘⌥S)" aria-label="Show or Hide Files">
          <svg width="16" height="14" viewBox="0 0 16 14" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
            <rect x="0.65" y="0.65" width="14.7" height="12.7" rx="2.5" />
            <line x1="5.65" y1="1" x2="5.65" y2="13" />
            <line x1="2.4" y1="4" x2="4" y2="4" stroke-linecap="round" />
            <line x1="2.4" y1="6" x2="4" y2="6" stroke-linecap="round" />
            <line x1="2.4" y1="8" x2="4" y2="8" stroke-linecap="round" />
          </svg>
        </button>
        <div class="tabs">
          {#each layout.tabs as path (path)}
            <div class="tab" class:front={path === layout.front}>
              <button class="label" onclick={() => (layout.front = path)} title={path}>{nodes.get(path)?.name ?? path}</button>
              <button class="close" onclick={() => close(path)} title="Close Tab (⌘W)">✕</button>
            </div>
          {/each}
        </div>
      </div>
      {#if front?.kind === "pdf"}
        <!-- the tools of the PDF in front, under the tabs -->
        <div class="tools">
          {#if outline.length}
            <button class:on={showOutline} onclick={() => (showOutline = !showOutline)}>Outline</button>
          {/if}
          <input
            bind:this={findInput}
            bind:value={findText}
            class:missed={notFound}
            type="search"
            placeholder="Find in the PDF (⌘F)"
            autocomplete="off"
            autocorrect="off"
            spellcheck="false"
            oninput={() => search(false)}
            onkeydown={(e) => {
              if (e.key === "Enter") search(true, e.shiftKey);
              if (e.key === "Escape") {
                e.preventDefault();
                findText = "";
                search(false);
                findInput?.blur();
              }
            }}
          />
          <span class="page">{count ? `Page ${current} of ${count}` : ""}</span>
          <button onclick={() => pdf?.zoom(-1)} title="Zoom Out (⌘-)">−</button>
          <button onclick={() => pdf?.zoom(0)} title="Fit Width (⌘0)">Fit</button>
          <button onclick={() => pdf?.zoom(1)} title="Zoom In (⌘=)">+</button>
          <button class="icon" onclick={() => openPath(front.path)} title="Open in Default App" aria-label="Open in Default App">↗</button>
        </div>
      {/if}
      <div class="body">
        <!-- the outline beside the viewer, not over it: the PDF is native and lies over the viewer alone -->
        {#if front?.kind === "pdf" && showOutline && outline.length}
          <nav class="outline">{@render entries(outline, 0)}</nav>
        {/if}
        <div class="viewer" bind:this={viewer}>
          {#if front?.kind === "pdf"}
            {#key front.path}
              <PdfView
                bind:this={pdf}
                path={front.path}
                page={layout.pages[front.path] ?? 1}
                onpage={turned}
                onopen={(c, o) => {
                  count = c;
                  outline = o;
                  current = layout.pages[front.path] ?? 1;
                }}
              />
            {/key}
          {:else if front}
            <div class="other">
              <p>{front.name}</p>
              <div class="actions">
                {#if front.kind === "markdown" && front.path !== project.review}
                  <button onclick={() => editReview(front.path)}>Edit in Review Pane</button>
                {/if}
                <button onclick={() => openPath(front.path)}>Open in Default App</button>
              </div>
            </div>
          {:else}
            <div class="other">
              <p class="muted">{project.name}</p>
              <p class="muted">Choose a file{layout.files ? "" : " (⌘⌥S shows them)"}.</p>
            </div>
          {/if}
        </div>
      </div>
    </section>
  </div>
{:else if error}
  <p class="error">{error}</p>
{/if}

<!-- the entries of an outline, each nested ones indented below it -->
{#snippet entries(list: Entry[], depth: number)}
  {#each list as entry}
    <button class="entry" style:padding-left="{10 + depth * 14}px" onclick={() => pdf?.go(entry.target)} title={entry.title}>{entry.title}</button>
    {#if entry.items.length}
      {@render entries(entry.items, depth + 1)}
    {/if}
  {/each}
{/snippet}

<style>
  /* absolute, not fixed: WebKit tints the title bar with what is fixed at the top of the page (as in Papers
     Desktop) */
  .colour {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 50;
    height: 6px;
  }
  .window {
    height: 100vh;
    box-sizing: border-box;
    display: grid;
  }
  .window.coloured {
    padding-top: 6px;
  }
  .files {
    position: relative;
    min-width: 0;
    min-height: 0;
  }
  .splitter {
    position: absolute;
    top: 0;
    bottom: 0;
    right: -3px;
    width: 6px;
    z-index: 5;
    cursor: col-resize;
  }
  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .bar {
    flex: none;
    display: flex;
    height: 30px;
    background: var(--sidebar);
    border-bottom: 1px solid var(--line);
  }
  .tabs {
    flex: 1;
    display: flex;
    min-width: 0;
    overflow-x: auto;
  }
  .toggle {
    flex: none;
    display: flex;
    align-items: center;
    padding: 0 10px;
    color: var(--muted);
    background: none;
    border: none;
    border-right: 1px solid var(--line);
  }
  .toggle.on {
    color: var(--accent);
  }
  .tools {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 12px;
    background: var(--bg);
    border-bottom: 1px solid var(--line);
  }
  .tools .page {
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .tools button {
    padding: 3px 9px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    color: var(--fg);
    font: inherit;
    font-size: 12px;
    white-space: nowrap;
  }
  .tools button.icon {
    min-width: 28px;
    padding: 3px 7px;
  }
  .tools button.on {
    background: var(--line);
  }
  .tools input {
    width: min(360px, 40%);
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: transparent;
    color: var(--fg);
    font: inherit;
    outline: none;
  }
  .tools input:focus {
    border-color: var(--accent);
  }
  .tools input.missed {
    border-color: var(--error);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .outline {
    flex: none;
    width: 240px;
    box-sizing: border-box;
    padding: 6px 0;
    overflow-y: auto;
    background: var(--sidebar);
    border-right: 1px solid var(--line);
  }
  .entry {
    display: block;
    width: 100%;
    box-sizing: border-box;
    padding-top: 3px;
    padding-bottom: 3px;
    padding-right: 10px;
    font: inherit;
    color: inherit;
    text-align: left;
    background: none;
    border: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .entry:hover {
    background: var(--chip);
  }
  .tab {
    display: flex;
    align-items: center;
    max-width: 220px;
    border-right: 1px solid var(--line);
  }
  .tab.front {
    background: var(--bg);
  }
  .tab button {
    font: inherit;
    color: inherit;
    background: none;
    border: none;
  }
  .label {
    padding: 0 4px 0 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .close {
    color: var(--muted);
    font-size: 10px;
    padding: 0 8px;
    visibility: hidden;
  }
  .tab:hover .close,
  .tab.front .close {
    visibility: visible;
  }
  .viewer {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    background: var(--viewer);
  }
  .other {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }
  .muted {
    color: var(--muted);
    margin: 0;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .error {
    color: var(--error);
    padding: 20px;
  }
</style>
