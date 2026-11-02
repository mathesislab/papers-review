<script lang="ts">
  // The page of a project's window: the project, its files, the file in front.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import type { Project } from "$lib/types";

  let project = $state<Project | null>(null);
  let error = $state("");

  async function load() {
    try {
      project = await invoke<Project>("project");
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    load();
    const unlisten = listen<string>("menu", (e) => command(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  });

  /** A command of the menu bar. */
  function command(id: string) {
    void id;
  }
</script>

<main>
  {#if project}
    <h1>{project.name}</h1>
    <p class="muted">{project.root}</p>
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</main>

<style>
  main {
    padding: 20px;
  }
  .muted {
    color: var(--muted);
  }
  .error {
    color: var(--error);
  }
</style>
