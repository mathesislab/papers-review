<script lang="ts">
  // The card of a new project, over the top of the window like the project's info: a folder without a review.toml
  // asks for the project's name, short name, journal, manuscript, due date and the file of its review, a new one
  // (review.md) or a Markdown file of the folder. Create writes review.toml and makes the file; Cancel closes the
  // project, which leaves nothing behind. There is no Esc: it would close the project by a stray key.

  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { SHORT, initials } from "./bar";
  import "./card.css";
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
  h2 {
    margin: 0 0 14px;
    font-size: 13px;
    font-weight: 600;
    text-align: center;
  }
  .column {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
</style>
