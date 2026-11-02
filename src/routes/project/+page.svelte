<script lang="ts">
  // The page of a project's window: the colour bar (right-click: its colour), the files (⌘⌥S hides them), the files
  // open in tabs, the one in front: a PDF in PDFKit, with its page, zoom and find. How the window looked is kept
  // between sessions (the app's state.json); the commands of the menu bar come as `menu`.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { CheckMenuItem, Menu } from "@tauri-apps/api/menu";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import { COLOURS, cssOf } from "$lib/colour";
  import Files from "$lib/Files.svelte";
  import PdfView from "$lib/PdfView.svelte";
  import type { Colour, Layout, Node, Project } from "$lib/types";

  let project = $state<Project | null>(null);
  let error = $state("");
  let layout = $state<Layout>({ files: true, tabs: [], front: null, pages: {} });
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
  /** the find field, and its text */
  let findInput = $state<HTMLInputElement>();
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
    return () => {
      unlisten.then((f) => f());
      unlistenChanged.then((f) => f());
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
  <div class="window" class:nofiles={!layout.files} class:coloured={cssOf(project.colour)}>
    {#if layout.files}
      <Files files={project.files} linked={project.linked} due={project.due} front={layout.front} onopen={open} />
    {/if}
    <section class="main">
      <div class="bar">
      <div class="tabs">
        {#each layout.tabs as path (path)}
          <div class="tab" class:front={path === layout.front}>
            <button class="label" onclick={() => (layout.front = path)} title={path}>{nodes.get(path)?.name ?? path}</button>
            <button class="close" onclick={() => close(path)} title="Close Tab (⌘W)">✕</button>
          </div>
        {/each}
      </div>
      {#if front?.kind === "pdf"}
        <div class="tools">
          <span class="page">{current} / {count}</span>
          <button onclick={() => pdf?.zoom(-1)} title="Zoom Out (⌘-)">−</button>
          <button onclick={() => pdf?.zoom(0)} title="Fit Width (⌘0)">Fit</button>
          <button onclick={() => pdf?.zoom(1)} title="Zoom In (⌘=)">+</button>
          <input
            bind:this={findInput}
            bind:value={findText}
            class:missed={notFound}
            placeholder="Find (⌘F)"
            autocomplete="off"
            autocorrect="off"
            spellcheck="false"
            oninput={() => search(false)}
            onkeydown={(e) => {
              if (e.key === "Enter") search(true, e.shiftKey);
              if (e.key === "Escape") {
                findText = "";
                search(false);
                findInput?.blur();
              }
            }}
          />
        </div>
      {/if}
      </div>
      <div class="viewer">
        {#if front?.kind === "pdf"}
          {#key front.path}
            <PdfView
              bind:this={pdf}
              path={front.path}
              page={layout.pages[front.path] ?? 1}
              onpage={turned}
              onopen={(c) => {
                count = c;
                current = layout.pages[front.path] ?? 1;
              }}
            />
          {/key}
        {:else if front}
          <div class="other">
            <p>{front.name}</p>
            <button onclick={() => openPath(front.path)}>Open in Default App</button>
          </div>
        {:else}
          <div class="other">
            <p class="muted">{project.name}</p>
            <p class="muted">Choose a file{layout.files ? "" : " (⌘⌥S shows them)"}.</p>
          </div>
        {/if}
      </div>
    </section>
  </div>
{:else if error}
  <p class="error">{error}</p>
{/if}

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
    grid-template-columns: 240px 1fr;
  }
  .window.coloured {
    padding-top: 6px;
  }
  .window.nofiles {
    grid-template-columns: 1fr;
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
  .tools {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
  }
  .tools .page {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    margin-right: 6px;
  }
  .tools button {
    font: inherit;
    color: inherit;
    background: var(--chip);
    border: none;
    border-radius: 5px;
    padding: 2px 8px;
  }
  .tools input {
    font: inherit;
    width: 150px;
    margin-left: 6px;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--fg);
  }
  .tools input.missed {
    border-color: var(--error);
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
  .error {
    color: var(--error);
    padding: 20px;
  }
</style>
