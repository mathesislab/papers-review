<script lang="ts">
  // The Settings window (⌘,): whether the projects open in one window, with a bar to switch between them, or each in
  // a window of its own. Kept in the app's data folder (settings.json); the project windows are told as it changes.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import type { Settings } from "$lib/types";

  let settings = $state<Settings | null>(null);

  function change(e: Event) {
    if (!settings) return;
    settings.project_bar = (e.currentTarget as HTMLInputElement).checked;
    invoke("set_settings", { settings: $state.snapshot(settings) });
  }

  onMount(() => {
    invoke<Settings>("settings").then((s) => (settings = s));
    const unlisten = listen<Settings>("settings-changed", (e) => (settings = e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  });
</script>

<main>
  {#if settings}
    <label class="option">
      <input type="checkbox" checked={settings.project_bar} onchange={change} />
      <span>Open projects in one window, with a bar to switch between them</span>
    </label>
    <p class="note">Off: each project opens in a window of its own.</p>
  {/if}
</main>

<style>
  main {
    padding: 28px 32px;
  }
  .option {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  input {
    margin: 0;
    flex: none;
  }
  .note {
    color: var(--muted);
    margin: 6px 0 0 22px;
  }
</style>
