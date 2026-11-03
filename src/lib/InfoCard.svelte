<script lang="ts">
  // The card of the project's info, over the top of the window like a macOS sheet: its name, short name (for the
  // tile of the project bar), journal, manuscript, due date, colour, the file of the review, the files and folders
  // linked from elsewhere, and the project's folder (to rename in the Finder). Open until Done or Esc. What is typed,
  // chosen as the colour or the review is written on Done, Cancel (or Esc) drops it; the linked files change as they
  // are added or taken out.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import { addFiles } from "./add";
  import { SHORT, initials } from "./bar";
  import { COLOURS } from "./colour";
  import type { Colour, InfoEdit, ProjectInfo } from "./types";

  let { onclose }: { onclose: () => void } = $props();

  /** the project as it is on disk */
  let info = $state<ProjectInfo | null>(null);
  /** the fields, as edited */
  let title = $state("");
  let short = $state("");
  let journal = $state("");
  let manuscript = $state("");
  let due = $state("");
  let colour = $state<Colour>("none");
  let review = $state("");
  let error = $state("");

  /** The folder of a path, and its name. */
  const base = (path: string) => path.slice(path.lastIndexOf("/") + 1);
  const dir = (path: string) => path.slice(0, path.lastIndexOf("/")).replace(/^\/Users\/[^/]+/, "~");

  /** A file of the project by its path in it, else by its path. */
  const within = (path: string) => (info && path.startsWith(`${info.root}/`) ? path.slice(info.root.length + 1) : path);

  /** `review.md`, to make when the project has none. */
  const fresh = $derived(info ? `${info.root}/review.md` : "");
  const reviews = $derived(info?.markdown.map((path) => ({ path, text: within(path) })) ?? []);

  /** Nothing but spaces: not set. */
  const blank = (text: string) => text.trim() || null;

  async function load(first: boolean) {
    try {
      const next = await invoke<ProjectInfo>("project_info");
      info = next;
      if (!first) return;
      title = next.title ?? "";
      short = next.short ?? "";
      journal = next.journal ?? "";
      manuscript = next.manuscript ?? "";
      due = next.due ?? "";
      colour = next.colour;
      review = next.review;
    } catch (e) {
      error = String(e);
    }
  }

  async function done() {
    if (!info) return;
    const edit: InfoEdit = {
      title: blank(title),
      short: blank(Array.from(short.trim()).slice(0, SHORT).join("")),
      journal: blank(journal),
      manuscript: blank(manuscript),
      due: due || null,
    };
    try {
      if ((Object.keys(edit) as (keyof InfoEdit)[]).some((key) => edit[key] !== info![key])) await invoke("set_project_info", { info: edit });
      if (colour !== info.colour) await invoke("set_colour", { colour });
      if (review !== info.review) {
        // review.md chosen where there is none: made first
        const path = info.markdown.includes(review) ? review : await invoke<string>("new_file", { folder: info.root, name: "review.md" });
        await invoke("set_review", { path });
      }
      onclose();
    } catch (e) {
      error = String(e);
    }
  }

  async function add(directory: boolean) {
    try {
      await addFiles(directory, null);
      await load(false);
    } catch (e) {
      error = String(e);
    }
  }

  async function unlink(path: string) {
    try {
      await invoke("unlink", { path });
      await load(false);
    } catch (e) {
      error = String(e);
    }
  }

  function focus(input: HTMLInputElement) {
    input.focus();
  }

  onMount(() => {
    load(true);
    // the linked files changed, from here or another window of the project
    const unlisten = listen("project-changed", () => load(false));
    return () => {
      unlisten.then((f) => f());
    };
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="scrim" role="presentation"></div>
<form class="card" aria-label="Project Info" onsubmit={(e) => (e.preventDefault(), done())}>
  {#if info}
    {@const folder = base(info.root)}
    <div class="grid">
      <label for="info-title">Name</label>
      <input id="info-title" bind:value={title} use:focus placeholder={folder} autocomplete="off" spellcheck="false" />

      <label for="info-short">Short name</label>
      <input id="info-short" class="short" bind:value={short} maxlength={SHORT} placeholder={initials(info.name)} autocomplete="off" spellcheck="false" />

      <label for="info-journal">Journal</label>
      <input id="info-journal" bind:value={journal} autocomplete="off" spellcheck="false" />

      <label for="info-manuscript">Manuscript ID</label>
      <input id="info-manuscript" bind:value={manuscript} autocomplete="off" spellcheck="false" />

      <label for="info-due">Due</label>
      <div class="row">
        <input id="info-due" type="date" bind:value={due} />
        {#if due}<button type="button" class="plain" onclick={() => (due = "")}>Clear</button>{/if}
      </div>

      <span class="label">Colour</span>
      <div class="row swatches">
        {#each COLOURS as c (c.colour)}
          <button
            type="button"
            class="swatch"
            class:none={c.css === null}
            class:on={c.colour === colour}
            style:background={c.css}
            title={c.name}
            aria-label={c.name}
            aria-pressed={c.colour === colour}
            onclick={() => (colour = c.colour)}
          ></button>
        {/each}
      </div>

      <label for="info-review">Review file</label>
      <select id="info-review" bind:value={review}>
        {#each reviews as r (r.path)}<option value={r.path}>{r.text}</option>{/each}
        {#if !info.markdown.includes(fresh)}<option value={fresh}>New review.md…</option>{/if}
      </select>

      <span class="label top">Linked</span>
      <div class="linked">
        {#each info.linked as path (path)}
          <div class="link" title={path}>
            <span class="name">{base(path)}</span>
            <span class="from">{dir(path)}</span>
            <button type="button" class="plain" onclick={() => unlink(path)} title="Remove Link" aria-label="Remove Link to {base(path)}">✕</button>
          </div>
        {:else}
          <span class="none-linked">No files linked from elsewhere</span>
        {/each}
        <div class="row">
          <button type="button" onclick={() => add(false)}>Add Files…</button>
          <button type="button" onclick={() => add(true)}>Add Folder…</button>
        </div>
      </div>

      <span class="label">Folder</span>
      <div class="row">
        <span class="path" title={info.root}>{info.root.replace(/^\/Users\/[^/]+/, "~")}</span>
        <button type="button" onclick={() => revealItemInDir(info!.root).catch((e) => (error = String(e)))}>Show in Finder</button>
      </div>
    </div>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <footer>
    <button type="button" onclick={onclose}>Cancel</button>
    <button type="submit" class="default" disabled={!info}>Done</button>
  </footer>
</form>

<style>
  /* absolute, not fixed: WebKit tints the title bar with what is fixed at the top of the page */
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 90;
    background: rgba(0, 0, 0, 0.22);
  }
  .card {
    position: absolute;
    z-index: 100;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    box-sizing: border-box;
    width: min(500px, calc(100vw - 32px));
    max-height: calc(100vh - 24px);
    overflow-y: auto;
    padding: 18px 20px 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-top: none;
    border-radius: 0 0 12px 12px;
    box-shadow: 0 10px 34px rgba(0, 0, 0, 0.3);
  }
  .grid {
    display: grid;
    grid-template-columns: 92px 1fr;
    align-items: center;
    gap: 9px 12px;
  }
  label,
  .label {
    color: var(--muted);
    text-align: right;
  }
  .label.top {
    align-self: start;
    padding-top: 3px;
  }
  input,
  select {
    box-sizing: border-box;
    min-width: 0;
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--fg);
    font: inherit;
    outline: none;
    -webkit-user-select: text;
    user-select: text;
  }
  input:focus,
  select:focus {
    border-color: var(--accent);
  }
  input.short {
    width: 80px;
  }
  input[type="date"] {
    width: 150px;
  }
  select {
    width: 100%;
  }
  button {
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    color: var(--fg);
    font: inherit;
    white-space: nowrap;
  }
  button.default {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  button:disabled {
    opacity: 0.5;
  }
  button.plain {
    padding: 0 6px;
    border: none;
    color: var(--muted);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .swatches {
    gap: 7px;
  }
  .swatch {
    flex: none;
    width: 20px;
    height: 20px;
    padding: 0;
    border-radius: 50%;
  }
  .swatch.none {
    background: linear-gradient(to top right, transparent calc(50% - 1px), var(--muted) 50%, transparent calc(50% + 1px));
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--accent);
  }
  .linked {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .from,
  .path {
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 11px;
  }
  .link .from {
    text-align: right;
  }
  .none-linked {
    color: var(--muted);
  }
  .error {
    margin: 10px 0 0;
    color: var(--error);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
</style>
