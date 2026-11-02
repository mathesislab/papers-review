<script lang="ts">
  // The start window: Open a project folder (or drop one here), and the projects opened last. It closes as a project
  // opens.

  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import { cssOf } from "$lib/colour";
  import type { Recent } from "$lib/types";

  let recent = $state<Recent[]>([]);
  let error = $state("");

  async function open(root: string) {
    error = "";
    try {
      await invoke("open_project", { path: root });
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    invoke<Recent[]>("recent").then((r) => (recent = r));
    const unlisten = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "drop") for (const path of e.payload.paths) open(path);
    });
    return () => {
      unlisten.then((f) => f());
    };
  });
</script>

<main>
  <h1>Papers Review</h1>
  <p class="lede">A review is a folder: the manuscript, what comes with it, and your review.</p>
  <div class="open">
    <button class="primary" onclick={() => invoke("choose_project")}>Open a Project Folder…</button>
    <kbd>⌘O</kbd>
    <span class="hint">or drop a folder here</span>
  </div>
  {#if error}<p class="error">{error}</p>{/if}
  {#if recent.length}
    <h2>Recent</h2>
    <ul>
      {#each recent as r (r.root)}
        <li>
          <button class="recent" disabled={r.missing} onclick={() => open(r.root)} title={r.root}>
            <span class="dot" style:background={cssOf(r.colour) ?? "transparent"}></span>
            <span class="name">{r.name}</span>
            <span class="path">{r.missing ? "not found" : r.root}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main {
    padding: 28px 32px;
    height: 100%;
    box-sizing: border-box;
    overflow: auto;
  }
  h1 {
    font-size: 22px;
    margin: 0 0 4px;
  }
  .lede {
    color: var(--muted);
    margin: 0 0 20px;
  }
  .open {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .hint {
    color: var(--muted);
  }
  button {
    font: inherit;
    color: inherit;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border: none;
    border-radius: 6px;
    padding: 6px 14px;
  }
  h2 {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    margin: 26px 0 6px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .recent {
    display: grid;
    grid-template-columns: 12px auto 1fr;
    gap: 10px;
    align-items: center;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-radius: 6px;
    padding: 6px 8px;
  }
  .recent:hover:not(:disabled) {
    background: var(--selection);
  }
  .recent:disabled {
    opacity: 0.5;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .name {
    font-weight: 500;
    white-space: nowrap;
  }
  .path {
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  .error {
    color: var(--error);
  }
</style>
