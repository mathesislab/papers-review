<script lang="ts">
  // The card of a new project, over the top of the window like the project's info: a folder without a review.toml
  // asks for the project's name, short name, journal, manuscript, due date and the file of its review, a new one
  // (review.md) or a Markdown file of the folder. Create writes review.toml and makes the file; Cancel closes the
  // project, which leaves nothing behind. There is no Esc: it would close the project by a stray key.

  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { SHORT, initials } from "./bar";
  import DateField from "./DateField.svelte";
  import type { DateFormat, InfoEdit, ProjectInfo } from "./types";

  let { format, oncreated }: { format: DateFormat; oncreated: () => void } = $props();

  /** the project as it is on disk: its folder and the Markdown files there */
  let info = $state<ProjectInfo | null>(null);
  /** the fields, as edited */
  let title = $state("");
  let short = $state("");
  let journal = $state("");
  let manuscript = $state("");
  let due = $state<string | null>(null);
  /** the name of the review file to make, unless a Markdown file of the folder is chosen */
  let file = $state("review.md");
  /** the Markdown file chosen (its path); empty: a new file */
  let existing = $state("");
  let error = $state("");
  /** asked of the app, not yet answered */
  let busy = $state(false);

  /** A file of the project by its path in it. */
  const within = (path: string) => (info && path.startsWith(`${info.root}/`) ? path.slice(info.root.length + 1) : path);

  /** Nothing but spaces: not set. */
  const blank = (text: string) => text.trim() || null;

  async function load() {
    try {
      const next = await invoke<ProjectInfo>("project_info");
      info = next;
      // a review.md there already is the review, rather than a new one
      const there = `${next.root}/review.md`;
      if (next.markdown.includes(there)) existing = there;
    } catch (e) {
      error = String(e);
    }
  }

  async function create() {
    const edit: InfoEdit = {
      title: blank(title),
      short: blank(Array.from(short.trim()).slice(0, SHORT).join("")),
      journal: blank(journal),
      manuscript: blank(manuscript),
      due,
    };
    busy = true;
    try {
      await invoke("create_project", { info: edit, review: file, existing: existing || null });
      oncreated();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function cancel() {
    busy = true;
    try {
      await invoke("cancel_project");
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }

  function focus(input: HTMLInputElement) {
    input.focus();
  }

  onMount(() => {
    load();
  });
</script>

<div class="scrim" role="presentation"></div>
<form class="card" aria-label="New project" onsubmit={(e) => (e.preventDefault(), create())}>
  <h2>New project</h2>
  {#if info}
    {@const folder = info.root.slice(info.root.lastIndexOf("/") + 1)}
    <div class="grid">
      <label for="new-title">Name</label>
      <input id="new-title" bind:value={title} use:focus placeholder={folder} autocomplete="off" spellcheck="false" />

      <label for="new-short">Short name</label>
      <input id="new-short" class="short" bind:value={short} maxlength={SHORT} placeholder={initials(title.trim() || folder)} autocomplete="off" spellcheck="false" />

      <label for="new-journal">Journal</label>
      <input id="new-journal" bind:value={journal} autocomplete="off" spellcheck="false" />

      <label for="new-manuscript">Manuscript ID</label>
      <input id="new-manuscript" bind:value={manuscript} autocomplete="off" spellcheck="false" />

      <label for="new-due">Due</label>
      <div class="row">
        <DateField id="new-due" bind:value={due} {format} />
      </div>

      <label for="new-review">Review file</label>
      <div class="column">
        {#if info.markdown.length}
          <select id="new-review" bind:value={existing}>
            <option value="">New file</option>
            {#each info.markdown as path (path)}<option value={path}>{within(path)}</option>{/each}
          </select>
        {/if}
        {#if !existing}
          <input id={info.markdown.length ? undefined : "new-review"} bind:value={file} placeholder="review.md" aria-label="Name of the new review file" autocomplete="off" spellcheck="false" />
        {/if}
      </div>
    </div>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <footer>
    <button type="button" onclick={cancel} disabled={busy}>Cancel</button>
    <button type="submit" class="default" disabled={!info || busy || (!existing && !file.trim())}>Create</button>
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
  h2 {
    margin: 0 0 14px;
    font-size: 13px;
    font-weight: 600;
    text-align: center;
  }
  .grid {
    display: grid;
    grid-template-columns: 92px 1fr;
    align-items: center;
    gap: 9px 12px;
  }
  label {
    color: var(--muted);
    text-align: right;
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
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .column {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
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
