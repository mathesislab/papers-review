<script lang="ts">
  // The buttons of an editor's bar, in the review pane and over a file in a tab: the list of symbols (Markdown only),
  // and the line numbers shown or hidden in every editor (the settings). Framed like the tools of the main window, or
  // bare like those of the pane's header.
  import { toggleLineNumbers } from "$lib/prefs";

  let {
    symbols,
    lineNumbers,
    framed = false,
    onsymbols,
  }: {
    /** whether the editor has symbols (it is Markdown) */
    symbols: boolean;
    /** whether the lines are numbered */
    lineNumbers: boolean;
    framed?: boolean;
    onsymbols: () => void;
  } = $props();
</script>

{#if symbols}
  <button
    class="symbols"
    class:framed
    onclick={onsymbols}
    title="Symbols (⌘/). Or type them: -> for →, CO_2 for CO₂, \alpha and Tab for α"
    aria-label="Symbols">Ω</button
  >
{/if}
<button
  class:framed
  class:on={lineNumbers}
  onclick={toggleLineNumbers}
  title={lineNumbers ? "Hide Line Numbers" : "Show Line Numbers"}
  aria-label={lineNumbers ? "Hide Line Numbers" : "Show Line Numbers"}
  aria-pressed={lineNumbers}
>
  <svg width="15" height="12" viewBox="0 0 15 12" aria-hidden="true" fill="none" stroke="currentColor" stroke-linecap="round">
    <path d="M1.5 1.6h1v3.2M1.3 4.8h2.4" stroke-width="1" />
    <path d="M1.2 7.6c.3-.6 2.1-.7 2.1.3 0 .9-2.1 1.5-2.1 2.6h2.3" stroke-width="1" />
    <path d="M6 2.5h7.5M6 6h7.5M6 9.5h5" stroke-width="1.3" />
  </svg>
</button>

<style>
  button {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 26px;
    padding: 2px 6px;
    border: 1px solid transparent;
    border-radius: 5px;
    background: none;
    color: var(--muted);
    font: inherit;
  }
  button:hover {
    background: var(--chip);
  }
  button.on {
    color: var(--accent);
  }
  .framed {
    min-width: 28px;
    height: 24px;
    border-color: var(--border);
    border-radius: 6px;
    color: var(--fg);
  }
  .framed.on {
    background: var(--line);
    color: var(--fg);
  }
  .symbols {
    font-family: "New York", ui-serif, Georgia, serif;
    font-size: 13px;
    line-height: 1;
  }
</style>
