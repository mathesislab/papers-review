<script lang="ts">
  // The page of a project's window: the colour bar (right-click: its colour), the project bar at the left when the
  // setting is on (a tile per project to switch to; right-click: open in a new window, remove), the files (⌘⌥S hides
  // them), the files open in tabs, the one in front: a PDF in PDFKit, with its outline, page, zoom and find; a
  // Markdown, text or BibTeX file edited in place (FileEditor). How the window looked is kept between sessions (the
  // app's state.json); the commands of the menu bar come as `menu`. The window switched to another project loads this
  // page again. The project's info (⌘I) is a card over the top of it, the PDF hidden meanwhile.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { CheckMenuItem, Menu, MenuItem } from "@tauri-apps/api/menu";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { confirm, message } from "@tauri-apps/plugin-dialog";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import type { Entry } from "tauri-plugin-pdfkit-api";
  import "$lib/theme.css";
  import { addFiles } from "$lib/add";
  import { badge, chip } from "$lib/bar";
  import { COLOURS, cssOf } from "$lib/colour";
  import { dueText } from "$lib/due";
  import FileEditor, { type Status } from "$lib/FileEditor.svelte";
  import Files from "$lib/Files.svelte";
  import InfoCard from "$lib/InfoCard.svelte";
  import PdfView from "$lib/PdfView.svelte";
  import { quote } from "$lib/quote";
  import type { BarItem, Colour, Kind, Layout, Node, Project, Settings } from "$lib/types";

  let project = $state<Project | null>(null);
  let settings = $state<Settings | null>(null);
  /** the projects of the project bar, in order */
  let bar = $state<BarItem[]>([]);
  const showBar = $derived(settings?.project_bar ?? false);
  let error = $state("");
  let layout = $state<Layout>({ files: true, files_width: 240, tabs: [], front: null, pages: {} });
  /** the layout read: written back only after, to the project it was read for */
  let ready = false;
  let layoutRoot = "";

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
  /** The kinds of file edited in their tab. */
  const EDITED: Kind[] = ["markdown", "text", "bib"];
  /** the file in front, edited, and its editor */
  let editor = $state<FileEditor>();
  let status = $state<Status>("saved");
  let writeError = $state("");
  /** the files that could not be read (not UTF-8, say), by their path, with why: not edited */
  let unreadable = $state<Record<string, string>>({});
  const edited = $derived(front && EDITED.includes(front.kind) && !unreadable[front.path] ? front : undefined);
  // the status of the file before is not this one's
  $effect(() => {
    layout.front;
    [status, writeError] = ["saved", ""];
  });
  // the file in front watched for changes on disk while it is edited
  $effect(() => {
    const path = edited?.path;
    if (!path) return;
    invoke("watch_file", { path });
    return () => {
      invoke("unwatch_file", { path });
    };
  });
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

  async function loadSettings() {
    settings = await invoke<Settings>("settings");
  }

  async function loadBar() {
    bar = await invoke<BarItem[]>("bar");
  }

  /** The layout and the file in front written now, to the project they are of: a switch or a close would lose the
   *  write waiting. */
  async function flush() {
    clearTimeout(timer);
    await Promise.all([
      editor?.flush(),
      ready ? invoke("set_layout", { root: layoutRoot, layout: $state.snapshot(layout) }) : undefined,
    ]);
  }

  /** The window switched to the project of a tile; the page loads again as it is done. */
  async function switchTo(root: string) {
    if (root === project?.root) return;
    try {
      await flush();
      await invoke("switch_project", { root });
    } catch (e) {
      await message(String(e), { title: "Switch Project", kind: "error" });
    }
  }

  /** The info card shown. The PDF is a native view over the page, so it is hidden while the card shows. */
  let card = $state(false);

  function openCard() {
    if (card) return;
    card = true;
    pdf?.cover(true);
  }

  function closeCard() {
    card = false;
    pdf?.cover(false);
  }

  /** A right-click on a tile: the project in a window of its own, or out of the bar; its info, if it is the one of
   *  this window. */
  async function tileMenu(e: MouseEvent, item: BarItem) {
    e.preventDefault();
    const items = await Promise.all([
      MenuItem.new({ text: "Open in New Window", action: () => invoke("open_in_new_window", { root: item.root }) }),
      MenuItem.new({ text: "Remove from Bar", action: () => invoke("remove_from_bar", { root: item.root }) }),
      ...(item.root === project?.root ? [MenuItem.new({ text: "Project Info…", action: openCard })] : []),
    ]);
    await (await Menu.new({ items })).popup();
  }

  /** The name of a tile and what is due, for its tooltip. */
  const tooltip = (item: BarItem) => [item.name, item.missing ? "not found" : item.due ? dueText(item.due) : ""].filter(Boolean).join("\n");

  /** A file to the front, in a tab of its own. */
  function open(node: Node) {
    if (!layout.tabs.includes(node.path)) layout.tabs.push(node.path);
    layout.front = node.path;
  }

  function close(path: string) {
    const i = layout.tabs.indexOf(path);
    if (i < 0) return;
    delete unreadable[path];
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

  /** A file read for its editor; one that cannot be read is shown as the others are, by its name. */
  async function read(path: string) {
    try {
      return { path, text: await invoke<string>("read_file", { path }) };
    } catch (e) {
      unreadable[path] = String(e);
      throw e;
    }
  }

  const write = (path: string, text: string) => invoke<void>("write_file", { path, text });

  /** Undo and Redo of the menu: the history of the editor in focus, else the field of the page. */
  function edit(what: string) {
    if (editor?.hasFocus()) (what === "redo" ? editor.redo : editor.undo)();
    else document.execCommand(what);
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

  /** An error of what was asked from the files, said in a dialog. */
  const fail = (title: string, e: unknown) => message(String(e), { title, kind: "error" });

  /** A Markdown file made in a folder, and opened in a tab. */
  async function newFile(folder: string, name: string) {
    try {
      const path = await invoke<string>("new_file", { folder, name });
      await load();
      const node = nodes.get(path);
      if (node) open(node);
    } catch (e) {
      await fail("New Markdown File", e);
    }
  }

  /** Files, or folders, linked or copied into a folder of the project. */
  async function add(folder: string, directory: boolean) {
    try {
      await addFiles(directory, folder);
    } catch (e) {
      await fail("Add to the Project", e);
    }
  }

  /** A path gone from the project: its tabs close. */
  function closeUnder(path: string) {
    for (const tab of [...layout.tabs]) if (tab === path || tab.startsWith(`${path}/`)) close(tab);
  }

  /** A linked file or folder out of the project; it stays where it is. */
  async function unlink(node: Node) {
    try {
      await invoke("unlink", { path: node.path });
      closeUnder(node.path);
    } catch (e) {
      await fail("Remove Link", e);
    }
  }

  /** A file or folder of the project to the Trash, once asked. */
  async function trash(node: Node) {
    const sure = await confirm(`Move “${node.name}” to the Trash?`, { title: "Move to Trash", kind: "warning", okLabel: "Move to Trash" });
    if (!sure) return;
    try {
      await invoke("trash", { path: node.path });
      closeUnder(node.path);
    } catch (e) {
      await fail("Move to Trash", e);
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
      case "info":
        openCard();
        break;
    }
  }

  onMount(() => {
    (async () => {
      await Promise.all([loadSettings(), loadBar()]);
      await load();
      if (!project) return;
      const saved = await invoke<Layout>("layout");
      // the files gone since are not opened again
      saved.tabs = saved.tabs.filter((t) => nodes.has(t));
      if (saved.front && !saved.tabs.includes(saved.front)) saved.front = saved.tabs[0] ?? null;
      layout = saved;
      layoutRoot = project.root;
      ready = true;
    })();
    const unlisten = listen<string>("menu", (e) => command(e.payload));
    // the colour or the files changed, from this window or another of the project
    const unlistenChanged = listen("project-changed", () => load());
    // the review pane hidden: the keys back to the PDF
    const unlistenHidden = listen("pane-hidden", () => pdf?.focus());
    // the window switched to another project: what is waiting to be written goes first
    const unlistenSwitched = listen("project-switched", async () => {
      await flush();
      location.reload();
    });
    const unlistenBar = listen("bar-changed", () => loadBar());
    const unlistenEdit = listen<string>("edit", (e) => edit(e.payload));
    const unlistenFile = listen<string>("file-changed", (e) => {
      if (e.payload === edited?.path) editor?.changedOnDisk();
    });
    const unlistenSettings = listen("settings-changed", () => loadSettings());
    return () => {
      unlistenSwitched.then((f) => f());
      unlistenBar.then((f) => f());
      unlistenEdit.then((f) => f());
      unlistenFile.then((f) => f());
      unlistenSettings.then((f) => f());
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
    // the project named, as the window may show another by then
    const root = layoutRoot;
    timer = setTimeout(() => invoke("set_layout", { root, layout: snapshot }), 300);
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
    style:grid-template-columns={`${showBar ? "56px " : ""}${layout.files ? `${layout.files_width}px ` : ""}1fr`}
  >
    {#if showBar}
      <!-- the project bar: a tile per project, the current one marked; the days left shown when few -->
      <aside class="projects" aria-label="Projects">
        {#each bar as item (item.root)}
          {@const due = badge(item.due)}
          {@const label = chip(item.name, item.short)}
          <button
            class="tile"
            class:current={item.root === project.root}
            class:missing={item.missing}
            title={tooltip(item)}
            aria-label={item.name}
            onclick={() => switchTo(item.root)}
            oncontextmenu={(e) => tileMenu(e, item)}
          >
            <span class="chip" style:background={cssOf(item.colour) ?? "var(--tile-none)"} style:font-size="{label.size}px">{label.text}</span>
            {#if due}<span class="badge" class:urgent={due.urgent}>{due.text}</span>{/if}
          </button>
        {/each}
        <button class="tile add" title="Open Project… (⌘O)" aria-label="Open Project" onclick={() => invoke("choose_project")}>
          <span class="chip">+</span>
        </button>
      </aside>
    {/if}
    {#if layout.files}
      <div class="files">
        <Files
          root={project.root}
          name={project.name}
          files={project.files}
          linked={project.linked}
          due={project.due}
          front={layout.front}
          review={project.review}
          onopen={open}
          onreview={(node) => editReview(node.path)}
          oninfo={openCard}
          onnew={newFile}
          onadd={add}
          onreveal={(path) => revealItemInDir(path).catch((e) => fail("Reveal in Finder", e))}
          ondefault={(path) => openPath(path).catch((e) => fail("Open in Default App", e))}
          onunlink={unlink}
          ontrash={trash}
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
      {#if edited}
        <!-- the file in front, edited: its name, whether it is written -->
        <div class="tools">
          <span class="name" title={edited.path}>{edited.name}</span>
          <span class="state">
            {#if status === "error"}<span class="error" title={writeError}>not saved</span>{:else}{status === "saved" ? "saved" : "editing"}{/if}
          </span>
          {@render actions(edited)}
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
          {:else if edited}
            {#key edited.path}
              {@const path = edited.path}
              <div class="editor">
                <FileEditor
                  bind:this={editor}
                  load={() => read(path)}
                  save={write}
                  mono={edited.kind !== "markdown"}
                  bind:status
                  bind:error={writeError}
                />
              </div>
            {/key}
          {:else if front}
            <div class="other">
              <p>{front.name}</p>
              {#if unreadable[front.path]}<p class="muted" title={unreadable[front.path]}>{unreadable[front.path]}</p>{/if}
              <div class="actions">{@render actions(front)}</div>
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

<!-- the buttons of a file that is not a PDF -->
{#snippet actions(file: Node)}
  {#if file.kind === "markdown" && file.path !== project?.review}
    <button onclick={() => editReview(file.path)}>Edit in Review Pane</button>
  {/if}
  <button onclick={() => openPath(file.path)}>Open in Default App</button>
{/snippet}

{#if card}
  <InfoCard onclose={closeCard} />
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
  .projects {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    min-height: 0;
    padding: 10px 0;
    overflow-y: auto;
    scrollbar-width: none;
    background: var(--sidebar);
    border-right: 1px solid var(--line);
  }
  .tile {
    position: relative;
    flex: none;
    display: flex;
    justify-content: center;
    padding: 0;
    background: none;
    border: none;
    font: inherit;
  }
  /* the tile of the project in view: a mark at the bar's edge, a ring round the tile */
  .tile.current::before {
    content: "";
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }
  .tile.current .chip {
    box-shadow:
      0 0 0 2px var(--sidebar),
      0 0 0 4px var(--accent);
  }
  .tile.missing {
    opacity: 0.45;
  }
  .chip {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 9px;
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.25);
  }
  .tile:not(.current):hover .chip {
    filter: brightness(0.94);
  }
  .tile.add .chip {
    box-sizing: border-box;
    border: 1px dashed var(--border);
    color: var(--muted);
    font-size: 18px;
    font-weight: 400;
    text-shadow: none;
  }
  .tile.add:hover .chip {
    background: var(--chip);
  }
  .badge {
    position: absolute;
    top: -4px;
    right: 3px;
    min-width: 15px;
    height: 15px;
    padding: 0 3px;
    box-sizing: border-box;
    border-radius: 8px;
    box-shadow: 0 0 0 2px var(--sidebar);
    background: #e8892b;
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    line-height: 15px;
    text-align: center;
  }
  .badge.urgent {
    background: #d63c2a;
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
  .tools .name {
    font-weight: 600;
  }
  .tools .state {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
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
  .editor {
    position: absolute;
    inset: 0;
    background: var(--bg);
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
