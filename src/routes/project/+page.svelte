<script lang="ts">
  // A tab of a project's window (papers-tabs: a page per tab, the others hidden but alive): the colour bar of the
  // settings (right-click: its colour), the project bar at the left when the setting is on (a tile per project to
  // switch to; right-click: open in a new window, remove), the files (⌘⌥S hides them; shown or not and their width the
  // same in all the project's tabs, ProjectSidebar), the tabs of the project (Tabs), and the file of this tab: a PDF in
  // PDFKit, with its outline, page, zoom and find; a Markdown, text or BibTeX file edited in place (FileEditor); or
  // none, the project's files alone. A file chosen opens in its tab, or this one when it shows none. The commands of
  // the menu bar come as `menu` to the tab in front. The project's info (⌘I) is a card over the top of it, the PDF
  // hidden meanwhile; so is the card that asks for the name, due date and review file of a new project (a folder
  // without a review.toml).

  import { invoke } from "@tauri-apps/api/core";
  import { emitTo } from "@tauri-apps/api/event";
  import { listen } from "$lib/events";
  import { CheckMenuItem, Menu, MenuItem } from "@tauri-apps/api/menu";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { confirm, message } from "@tauri-apps/plugin-dialog";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { onMount, tick } from "svelte";
  import type { Entry } from "tauri-plugin-pdfkit-api";
  import "$lib/theme.css";
  import { addFiles, addPaths } from "$lib/add";
  import { COLOURS, ProjectBar, ProjectSidebar, Tabs, cssOf, type Tile } from "papers-write/project";
  import { daysLeft, formatDate, shortDate, urgency } from "$lib/due";
  import { FileEditor, PREFS, prefsFrom, setEditorFont, wordCount, wordsOf, type Status } from "papers-write/editor";
  import EditorButtons from "$lib/EditorButtons.svelte";
  import { locations } from "$lib/locations";
  import Files from "$lib/Files.svelte";
  import InfoCard from "$lib/InfoCard.svelte";
  import NewProjectCard from "$lib/NewProjectCard.svelte";
  import PdfView from "$lib/PdfView.svelte";
  import { quote, quoteText } from "$lib/quote";
  import type { BarItem, Colour, Kind, Node, Project, Settings } from "$lib/types";

  let project = $state<Project | null>(null);
  let settings = $state<Settings | null>(null);
  /** the projects of the project bar, in order */
  let bar = $state<BarItem[]>([]);
  const showBar = $derived(settings?.project_bar ?? false);
  let error = $state("");
  /** the file of this tab (none: the project's files alone), and the page of it in view as last left (a PDF) */
  let file = $state<string | null>(null);
  let startPage = $state(1);
  /** the files panel, as all the project's tabs show it */
  const side = new ProjectSidebar();
  /** whether this tab is the one in front of its window */
  let inFront = $state(true);

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

  /** the file of a path: of the project, or a PDF from outside it (a paper of the references) */
  const nodeOf = (path: string): Node => nodes.get(path) ?? { name: path.split("/").pop() ?? path, path, kind: "pdf" };
  /** whether this tab's file is from outside the project */
  const outside = $derived(!!project && !!file && !nodes.has(file));

  const front = $derived(file ? nodeOf(file) : undefined);

  /** the PDF in front, its page count and page in view */
  let pdf = $state<PdfView>();
  let count = $state(0);
  let current = $state(1);
  /** whether it has a page to go back to (the one a link left) or forward */
  let history = $state({ back: false, forward: false });
  /** the page being typed in the field that replaces "Page X of N" (Go to Page…), null when it is not shown */
  let typing = $state<string | null>(null);
  let pageInput = $state<HTMLInputElement>();
  /** its outline (empty until it is open), and whether the panel for it is shown */
  let outline = $state<Entry[]>([]);
  let showOutline = $state(false);
  // the outline of the file before is not this one's
  $effect(() => {
    file;
    outline = [];
  });
  /** the find field, and its text */
  let findInput = $state<HTMLInputElement>();
  /** The kinds of file edited in their tab. */
  const EDITED: Kind[] = ["markdown", "latex", "text", "bib"];
  /** the file in front, edited, and its editor */
  let editor = $state<FileEditor>();
  let status = $state<Status>("saved");
  let writeError = $state("");
  /** the files that could not be read (not UTF-8, say), by their path, with why: not edited */
  let unreadable = $state<Record<string, string>>({});
  const edited = $derived(front && EDITED.includes(front.kind) && !unreadable[front.path] ? front : undefined);
  // the status of the file before is not this one's
  $effect(() => {
    file;
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
    // told again when this tab comes to the front
    if (!viewer || !inFront) return;
    const area = viewer;
    const tell = () => {
      const box = area.getBoundingClientRect();
      const viewport = window.innerHeight;
      invoke("pane_area", { area: { x: box.left, y: box.top, width: box.width, height: box.height, viewport } });
    };
    const observer = new ResizeObserver(tell);
    observer.observe(area);
    return () => observer.disconnect();
  });
  let findText = $state("");
  let notFound = $state(false);

  /** the page in view kept, a moment after it is turned to */
  let pageTimer: ReturnType<typeof setTimeout> | undefined;
  function turned(path: string, page: number) {
    if (path !== file) return;
    current = page;
    clearTimeout(pageTimer);
    pageTimer = setTimeout(() => invoke("set_pdf_page", { path, page }), 300);
  }

  function recorded(path: string, h: { back: boolean; forward: boolean }) {
    if (path === file) history = h;
  }

  /** The field for a page shown, with the page in view selected. */
  async function askPage() {
    if (front?.kind !== "pdf" || !count) return;
    typing = String(current);
    await tick();
    pageInput?.focus();
    pageInput?.select();
  }

  /** The field for a page closed, at the page typed if `go`; the keys go back to the PDF. */
  function answerPage(go: boolean) {
    const page = Number.parseInt(typing ?? "", 10);
    typing = null;
    if (go && Number.isFinite(page)) pdf?.pageTo(Math.min(Math.max(page, 1), count));
    pdf?.focus();
  }

  async function search(again: boolean, backwards = false) {
    if (!pdf) return;
    notFound = findText !== "" && !(await pdf.find(findText, again, backwards));
  }

  /** the colour of the bar at the top of the window (the settings) */
  const barColour = $derived(settings ? cssOf(settings.window_colour) : null);

  /** A right-click on the bar: the colour of the bar of the app's windows (the settings). */
  async function colourMenu(e: MouseEvent) {
    e.preventDefault();
    const items = COLOURS.map((c) =>
      CheckMenuItem.new({
        text: c.name,
        checked: settings?.window_colour === c.colour,
        action: () => setColour(c.colour),
      }),
    );
    await (await Menu.new({ items: await Promise.all(items) })).popup();
  }

  async function setColour(colour: Colour) {
    const now = await invoke<Settings>("settings");
    await invoke("set_settings", { settings: { ...now, window_colour: colour } });
  }

  async function load() {
    try {
      const next = await invoke<Project>("project");
      project = next;
    } catch (e) {
      error = String(e);
    }
  }

  /** what the editor takes from the settings */
  const prefs = $derived(settings ? prefsFrom(settings) : PREFS);
  /** the words of the file edited, and of its selection */
  let words = $state(0);
  let selected = $state(0);

  async function loadSettings() {
    settings = await invoke<Settings>("settings");
    setEditorFont(settings.editor_font, settings.editor_size);
  }

  async function loadBar() {
    bar = await invoke<BarItem[]>("bar");
  }

  /** The file of this tab read again (it was given another, or none); the text edited before written first. */
  async function readFile() {
    await editor?.flush();
    const next = await invoke<string | null>("page_file");
    startPage = next && nodeOf(next).kind === "pdf" ? await invoke<number>("pdf_page", { path: next }) : 1;
    file = next;
  }

  /** The window switched to the project of a tile (its tabs as left), or, for its own, the files shown or hidden. */
  async function switchTo(root: string) {
    if (root === project?.root) return command("files");
    try {
      await invoke("switch_project", { root });
    } catch (e) {
      await message(String(e), { title: "Switch Project", kind: "error" });
    }
  }

  /** The info card shown, or the card that asks for the name of a new project. The PDF and the review pane are
   *  native views above the page, so they hide while a card shows. */
  let card = $state(false);
  const covered = $derived(card || (project?.new ?? false));
  $effect(() => {
    invoke("page_covered", { covered });
  });
  /** the PDF hidden while this tab is not in front too, or it would keep the keys */
  const pdfCovered = $derived(covered || !inFront);

  function openCard() {
    if (!project?.new) card = true;
  }

  function closeCard() {
    card = false;
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
  const LEVELS = { far: "ok", near: "soon", close: "near", due: "due" } as const;

  /** A project of the bar as a tile of the project bar: the due date as its detail, how near it is as its level. */
  const tileOf = (item: BarItem): Tile => ({
    id: item.root,
    name: item.name,
    short: item.short,
    colour: item.colour,
    current: item.root === project?.root,
    detail: item.due ? shortDate(item.due) : undefined,
    level: item.due ? LEVELS[urgency(daysLeft(item.due))] : null,
    missing: item.missing,
    tooltip: tooltip(item),
  });

  const tooltip = (item: BarItem) =>
    [item.name, item.missing ? "not found" : item.due ? `Due ${formatDate(item.due, settings?.date_format ?? "ymd")}` : ""].filter(Boolean).join("\n");

  /** A file to the front: its tab, or this one when it shows none. */
  function open(node: Node) {
    invoke("open_file", { path: node.path }).catch((e) => fail("Open", e));
  }

  /** The narrowest and widest the files panel is dragged. */
  const FILES_WIDTH = { min: 160, max: 520 };

  /** The right edge of the files panel dragged: its width follows. The PDF lies over the page, but the drag stays
   *  with the edge: AppKit sends the moves to the view the press went to. */
  function resize(e: PointerEvent) {
    const edge = e.currentTarget as HTMLElement;
    edge.setPointerCapture(e.pointerId);
    const [start, width] = [e.clientX, side.width];
    const move = (m: PointerEvent) => {
      side.set({ width: Math.round(Math.min(FILES_WIDTH.max, Math.max(FILES_WIDTH.min, width + m.clientX - start))) }, false);
    };
    const up = () => {
      side.keep();
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

  /** Whether a field of the page has the keys (the find or page field, a field of a card), not the editor. */
  function inField() {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement) || active.closest(".cm-editor")) return false;
    return active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement || active.isContentEditable;
  }

  /** Undo, Redo, Symbols…, Back and Forward of the menu, which come here when the page has the keys (the pane is
   *  told otherwise): with a file edited in front, those of its editor (the history, the symbols, the indent) unless a
   *  field of the page has the keys, whose Undo and Redo they are; with the PDF in front, Back and Forward are its.
   *  Not `hasFocus()`: the editor has lost it to the menu by then. */
  function edit(what: string) {
    const toEditor = !!editor && !!edited && !inField();
    if (what === "back" || what === "forward") {
      if (toEditor) (what === "back" ? editor!.outdent : editor!.indent)();
      else if (what === "back") pdf?.back();
      else pdf?.forward();
    } else if (what === "symbols") {
      if (toEditor) editor!.list();
    } else if (toEditor) (what === "redo" ? editor!.redo : editor!.undo)();
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
      await invoke("open_file", { path });
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

  /** The paper of this tab, from outside the project, added to it: linked, it stays where it is, and is a linked
   *  file of the project once that is loaded; copied, the tab shows the copy, in the project's folder, at the same
   *  page. */
  async function addOutside(path: string) {
    try {
      const done = await addPaths([path], null);
      if (!done) return;
      await load();
      if (done === "link") return;
      const copy = `${project?.root}/${path.split("/").pop()}`;
      await invoke("set_pdf_page", { path: copy, page: current });
      await invoke("page_shows", { path: copy });
    } catch (e) {
      await fail("Add to the Project", e);
    }
  }

  /** Files or folders dropped on the window from the Finder: added to the project. Those of the project itself
   *  are left (the project does not add them again either), so as not to ask about nothing. Not while a card is
   *  shown. */
  async function drop(paths: string[]) {
    if (!project || covered) return;
    const root = project.root;
    const added = paths.filter((p) => p !== root && !p.startsWith(`${root}/`));
    if (!added.length) return;
    try {
      await addPaths(added, null);
    } catch (e) {
      await fail("Add to the Project", e);
    }
  }

  /** A linked file or folder out of the project; it stays where it is. */
  async function unlink(node: Node) {
    try {
      await invoke("unlink", { path: node.path });
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
    } catch (e) {
      await fail("Move to Trash", e);
    }
  }

  /** ⌘U: the selection of the PDF as a quote, into the review at its cursor; ⌘Y the same without quotation marks. */
  async function quoteSelection(plain = false) {
    const selection = await pdf?.selection();
    if (!selection) return;
    try {
      await invoke("quote", { text: quote(selection, !plain) });
    } catch (e) {
      error = String(e);
    }
  }

  /** ⇧⌘R: the references listed in the review pane, which is shown; the text of the selection in the PDF, if there
   *  is one, is the reference to look for. */
  async function references() {
    const selection = await pdf?.selection();
    const text = selection ? quoteText(selection) : "";
    try {
      await invoke("pane_show");
      await emitTo({ kind: "Webview", label: `${getCurrentWindow().label}-pane` }, "references", { text });
    } catch (e) {
      error = String(e);
    }
  }

  /** A command of the menu bar. */
  function command(id: string) {
    switch (id) {
      case "files":
        side.set({ shown: !side.shown });
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
      case "previous-page":
        pdf?.turn("previous");
        break;
      case "next-page":
        pdf?.turn("next");
        break;
      case "first-page":
        pdf?.turn("first");
        break;
      case "last-page":
        pdf?.turn("last");
        break;
      case "go-to-page":
        askPage();
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
      case "insert":
        quoteSelection(true);
        break;
      case "references":
        references();
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
      await readFile();
    })();
    const stopSide = side.start();
    const unlisten = listen<string>("menu", (e) => command(e.payload));
    // the colour or the files changed, from this window or another of the project
    const unlistenChanged = listen("project-changed", () => load());
    // the review pane hidden: the keys back to the PDF
    const unlistenHidden = listen("pane-hidden", () => pdf?.focus());
    // this tab given another file, or none
    const unlistenOpened = listen("opened", () => readFile());
    // this tab closing: what was typed written first
    const unlistenClosing = listen("closing", async () => {
      await editor?.flush();
      invoke("close_page");
    });
    const unlistenShown = listen<boolean>("tab-shown", (e) => (inFront = e.payload));
    const unlistenBar = listen("bar-changed", () => loadBar());
    const unlistenEdit = listen<string>("edit", (e) => edit(e.payload));
    // a click or a field in the page: the keys are the page's, not the pane's (which hears no blur when the keys go
    // from it to the page), for the menu's Undo, Redo, Back and Forward
    const toPage = () => invoke("pane_focus", { focused: false });
    document.addEventListener("pointerdown", toPage, true);
    document.addEventListener("focusin", toPage);
    const unlistenFile = listen<string>("file-changed", (e) => {
      if (e.payload === edited?.path) editor?.changedOnDisk();
    });
    const unlistenSettings = listen("settings-changed", () => loadSettings());
    // files dropped from the Finder
    const unlistenDrop = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "drop") drop(e.payload.paths);
    });
    return () => {
      document.removeEventListener("pointerdown", toPage, true);
      document.removeEventListener("focusin", toPage);
      unlistenDrop.then((f) => f());
      stopSide();
      unlistenOpened.then((f) => f());
      unlistenClosing.then((f) => f());
      unlistenShown.then((f) => f());
      unlistenBar.then((f) => f());
      unlistenEdit.then((f) => f());
      unlistenFile.then((f) => f());
      unlistenSettings.then((f) => f());
      unlisten.then((f) => f());
      unlistenChanged.then((f) => f());
      unlistenHidden.then((f) => f());
    };
  });

</script>

<!-- the colour of the app's windows (the settings), at the top, to tell them apart among others; a right-click
     chooses it -->
{#if project && barColour}
  <div class="colour" style:background={barColour} title="The colour of the windows (Settings): right-click to choose" oncontextmenu={colourMenu} role="presentation"></div>
{/if}

{#if project}
  <div
    class="window"
    class:nofiles={!side.shown}
    class:coloured={barColour}
    style:grid-template-columns={`${showBar ? "92px " : ""}${side.shown ? `${side.width}px ` : ""}1fr`}
  >
    {#if showBar}
      <!-- the project bar (papers-write's): a tile per project, the current one marked, with its due date and a bar for how near it is -->
      <ProjectBar
        items={bar.map(tileOf)}
        onswitch={switchTo}
        onmenu={(id, e) => tileMenu(e, bar.find((item) => item.root === id)!)}
        onadd={() => invoke("choose_project")}
        addTitle="Open Project… (⌘O)"
      />
    {/if}
    {#if side.shown}
      <div class="files">
        <Files
          root={project.root}
          name={project.name}
          journal={project.journal}
          manuscript={project.manuscript}
          due={project.due}
          dateFormat={settings?.date_format ?? "ymd"}
          files={project.files}
          linked={project.linked}
          front={file}
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
        <button class="toggle" class:on={side.shown} onclick={() => command("files")} title="Show or Hide Files (⌘⌥S)" aria-label="Show or Hide Files">
          <svg width="16" height="14" viewBox="0 0 16 14" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
            <rect x="0.65" y="0.65" width="14.7" height="12.7" rx="2.5" />
            <line x1="5.65" y1="1" x2="5.65" y2="13" />
            <line x1="2.4" y1="4" x2="4" y2="4" stroke-linecap="round" />
            <line x1="2.4" y1="6" x2="4" y2="6" stroke-linecap="round" />
            <line x1="2.4" y1="8" x2="4" y2="8" stroke-linecap="round" />
          </svg>
        </button>
        <div class="tabs">
          <!-- the tabs of the project; a lone tab of the files alone is no tab to show -->
          <Tabs least={file ? 1 : 2}>
            {#snippet extra(tab)}
              {#if tab.id === getCurrentWebview().label && outside && file}
                {@const path = file}
                <button onclick={() => addOutside(path)} title="Add to the Project…" aria-label="Add to the Project">
                  <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M6 2v8M2 6h8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
                </button>
              {/if}
            {/snippet}
          </Tabs>
        </div>
      </div>
      {#if front?.kind === "pdf"}
        <!-- the tools of the PDF in front, under the tabs -->
        <div class="tools">
          <button class="icon" disabled={!history.back} onclick={() => pdf?.back()} title="Back (⌘[)" aria-label="Back">‹</button>
          <button class="icon" disabled={!history.forward} onclick={() => pdf?.forward()} title="Forward (⌘])" aria-label="Forward">›</button>
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
          {#if typing !== null}
            <!-- a page typed: Enter goes, Esc and leaving the field give it up -->
            <input
              bind:this={pageInput}
              bind:value={typing}
              class="page"
              inputmode="numeric"
              autocomplete="off"
              aria-label="Go to Page"
              onkeydown={(e) => {
                if (e.key === "Enter") answerPage(true);
                if (e.key === "Escape") answerPage(false);
              }}
              onblur={() => (typing = null)}
            />
            <span class="page">of {count}</span>
          {:else if count}
            <button class="page" onclick={askPage} title="Go to Page… (⌥⌘G)">Page {current} of {count}</button>
          {:else}
            <span class="page"></span>
          {/if}
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
          <EditorButtons framed symbols={edited.kind === "markdown"} lineNumbers={prefs.lineNumbers} onsymbols={() => editor?.list()} />
          <span class="state">
            {wordCount(words, selected)} ·
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
                page={startPage}
                covered={pdfCovered}
                onpage={turned}
                onhistory={recorded}
                onopen={(c, o) => {
                  count = c;
                  outline = o;
                  current = startPage;
                  history = { back: false, forward: false };
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
                  mono={edited.kind !== "markdown" && edited.kind !== "latex"}
                  latex={edited.kind === "latex"}
                  extensions={edited.kind === "markdown" ? locations : []}
                  symbols={settings?.symbols}
                  {prefs}
                  onchange={(text) => (words = wordsOf(text))}
                  bind:selected
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
              <p class="muted">Choose a file{side.shown ? "" : " (⌘⌥S shows them)"}.</p>
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

{#if project?.new}
  <NewProjectCard format={settings?.date_format ?? "ymd"} oncreated={load} />
{:else if card}
  <InfoCard format={settings?.date_format ?? "ymd"} onclose={closeCard} />
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
    align-items: center;
    min-width: 0;
    padding: 0 6px;
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
  .tools button.page {
    border-color: transparent;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .tools button.page:hover {
    border-color: var(--border);
  }
  .tools input.page {
    width: 56px;
    height: 24px;
    padding: 0 6px;
    text-align: right;
    color: var(--fg);
  }
  .tools button:disabled {
    opacity: 0.4;
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
