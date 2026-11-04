<script lang="ts">
  // The Settings window (⌘,): whether the projects open in one window, with a bar to switch between them, or each in
  // a window of its own; the folder new reviews are made in; how dates are written; the font and size of the editors; the exports of the review (the
  // reviewer's name, whether the PDF is signed with it, whether its header has the journal, the manuscript ID and the
  // date); the updates (the version running, whether to look for them, Check Now, the one found to install); and the sequences typed in the review that become symbols: the defaults, each turned off if not wanted,
  // and the user's own. Kept in the app's data folder (settings.json); the project windows are told as it changes.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, tick } from "svelte";
  import "$lib/theme.css";
  import { DEFAULTS } from "$lib/symbols";
  import { Updates } from "$lib/updates.svelte";
  import type { DateFormat, EditorFont, Indent, Settings } from "$lib/types";

  const FORMATS: [DateFormat, string][] = [
    ["ymd", "2026-11-14"],
    ["dmy", "14.11.2026"],
    ["mdy", "11/14/2026"],
  ];
  const FONTS: [EditorFont, string][] = [
    ["system", "System"],
    ["serif", "Serif"],
    ["mono", "Monospace"],
  ];

  const INDENTS: [Indent, string][] = [
    ["four", "4 spaces"],
    ["two", "2 spaces"],
    ["tab", "A tab"],
  ];

  let settings = $state<Settings | null>(null);
  const updates = new Updates();
  /** the sequence and the replacement of a pair being added */
  let keys = $state("");
  let insert = $state("");
  const addable = $derived(keys.length >= 2 && insert !== "");

  function save() {
    if (settings) invoke("set_settings", { settings: $state.snapshot(settings) });
  }

  function change(e: Event) {
    if (!settings) return;
    settings.project_bar = (e.currentTarget as HTMLInputElement).checked;
    save();
  }

  async function chooseReviews() {
    if (!settings) return;
    const folder = await open({
      directory: true,
      title: "Choose the folder for your reviews",
      defaultPath: settings.reviews_folder ?? undefined,
      canCreateDirectories: true,
    });
    if (!folder) return;
    settings.reviews_folder = folder;
    save();
  }

  /** The pairs of the user, but for those of `sequence`. */
  const without = (sequence: string) => (settings?.symbols ?? []).filter(([k]) => k !== sequence);

  /** Whether a default sequence is turned off: a pair of the user gives it an empty replacement. */
  const off = (sequence: string) => !!settings?.symbols.some(([k, v]) => k === sequence && !v);

  function turn(sequence: string, on: boolean) {
    if (!settings) return;
    settings.symbols = on ? without(sequence) : [...without(sequence), [sequence, ""]];
    save();
  }

  function add() {
    if (!settings || !addable) return;
    settings.symbols = [...without(keys), [keys, insert]];
    [keys, insert] = ["", ""];
    save();
  }

  function remove(sequence: string) {
    if (!settings) return;
    settings.symbols = without(sequence);
    save();
  }

  /** Scroll to a section (its id): the window opens at one after an update, and the menu may ask for one. */
  async function show(section: string) {
    await tick();
    document.getElementById(section)?.scrollIntoView();
  }

  onMount(() => {
    Promise.all([invoke<Settings>("settings"), updates.load()]).then(([s]) => {
      settings = s;
      if (location.hash) show(location.hash.slice(1));
    });
    const unlisten = [
      listen<Settings>("settings-changed", (e) => (settings = e.payload)),
      listen<string>("section", (e) => show(e.payload)),
    ];
    return () => unlisten.forEach((u) => u.then((f) => f()));
  });

  /** Esc with no field in focus: the window closes (a field gives up the focus first). */
  function key(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    const field = document.activeElement;
    if (field instanceof HTMLElement && field !== document.body) field.blur();
    else getCurrentWindow().close();
  }
</script>

<svelte:window onkeydown={key} />

<main>
  {#if settings}
    <label class="option">
      <input type="checkbox" checked={settings.project_bar} onchange={change} />
      <span>Open projects in one window, with a bar to switch between them</span>
    </label>
    <p class="note">Off: each project opens in a window of its own.</p>

    <h2>New reviews</h2>
    <div class="field">
      <label for="reviews-folder">Reviews folder</label>
      <span id="reviews-folder" class="folder" class:unset={!settings.reviews_folder} title={settings.reviews_folder ?? ""}>{settings.reviews_folder ?? "Not chosen yet"}</span>
      <button onclick={chooseReviews}>Choose…</button>
    </div>
    <p class="note first">New reviews from a manuscript are made in it, a folder each.</p>

    <h2>Dates</h2>
    <div class="field">
      <label for="date-format">Date format</label>
      <select id="date-format" bind:value={settings.date_format} onchange={save}>
        {#each FORMATS as [format, example] (format)}<option value={format}>{example}</option>{/each}
      </select>
    </div>

    <h2>Editor</h2>
    <div class="field">
      <label for="editor-font">Font</label>
      <select id="editor-font" bind:value={settings.editor_font} onchange={save}>
        {#each FONTS as [font, name] (font)}<option value={font}>{name}</option>{/each}
      </select>
    </div>
    <div class="field">
      <label for="editor-size">Size</label>
      <input id="editor-size" type="range" min="11" max="20" step="0.5" bind:value={settings.editor_size} onchange={save} />
      <span class="size">{settings.editor_size} pt</span>
    </div>
    <div class="field">
      <label for="indent">Tab inserts</label>
      <select id="indent" bind:value={settings.indent} onchange={save}>
        {#each INDENTS as [indent, name] (indent)}<option value={indent}>{name}</option>{/each}
      </select>
    </div>
    <label class="option">
      <input type="checkbox" bind:checked={settings.readable_width} onchange={save} />
      <span>Readable line length</span>
    </label>
    <p class="note">The text a column in the middle, with a margin on each side; off: as wide as the window.</p>
    <label class="option">
      <input type="checkbox" bind:checked={settings.line_numbers} onchange={save} />
      <span>Line numbers</span>
    </label>
    <p class="note first">The review and the text files of a project.</p>

    <h2>Export</h2>
    <div class="field">
      <label for="reviewer-name">Reviewer</label>
      <input id="reviewer-name" class="name" bind:value={settings.reviewer_name} onchange={save} placeholder="Your name" autocomplete="off" spellcheck="false" />
    </div>
    <label class="option">
      <input type="checkbox" bind:checked={settings.sign} onchange={save} />
      <span>Sign the reviews with my name</span>
    </label>
    <p class="note">Off: the PDF is anonymous.</p>
    <label class="option">
      <input type="checkbox" bind:checked={settings.header_details} onchange={save} />
      <span>Header with journal, manuscript ID and date</span>
    </label>
    <p class="note">Off: the PDF is headed by the project's heading alone.</p>

    <h2 id="updates">Updates</h2>
    <div class="field">
      <span class="version">Version {updates.version}</span>
      <span class="status" class:error={updates.updating === "failed"}>{updates.status}</span>
      {#if updates.update}
        <button disabled={updates.updating === "installing"} onclick={updates.install}>
          Install Version {updates.update.version} and Restart
        </button>
      {:else}
        <button disabled={updates.updating === "checking"} onclick={updates.check}>Check Now</button>
      {/if}
    </div>
    {#if updates.updating === "installing"}
      <progress value={updates.progress.done} max={updates.progress.total ?? undefined}></progress>
    {/if}
    {#if updates.installed}
      <p class="note first">Updated from {updates.installed.from}.</p>
    {/if}
    <label class="option">
      <input type="checkbox" bind:checked={settings.check_updates} onchange={save} />
      <span>Check for updates automatically</span>
    </label>
    <p class="note">At start and twice a day; a new version is installed only when you ask.</p>

    <h2>Typed symbols</h2>
    <p class="note first">
      In the review, a sequence typed becomes its symbol as its last character is typed; ⌘Z gives back what was typed.
      Tab after a LaTeX name (\alpha) gives its symbol; ⌘/ lists them.
    </p>

    <h3>Your own</h3>
    <ul>
      {#each settings.symbols.filter(([, v]) => v) as [sequence, symbol] (sequence)}
        <li>
          <code>{sequence}</code>
          <span class="arrow">→</span>
          <span class="symbol">{symbol}</span>
          <button onclick={() => remove(sequence)} title="Remove" aria-label="Remove {sequence}">✕</button>
        </li>
      {/each}
    </ul>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        add();
      }}
    >
      <input bind:value={keys} placeholder="Typed" spellcheck="false" autocomplete="off" />
      <span class="arrow">→</span>
      <input bind:value={insert} placeholder="Becomes" spellcheck="false" autocomplete="off" />
      <button type="submit" disabled={!addable}>Add</button>
    </form>
    <p class="note first">At least two characters. A sequence that is a default gives it another symbol.</p>

    <h3>Defaults</h3>
    <ul>
      {#each DEFAULTS as rule (rule.keys)}
        <li>
          <label>
            <input type="checkbox" checked={!off(rule.keys)} onchange={(e) => turn(rule.keys, e.currentTarget.checked)} />
            <code>{rule.keys}</code>
            <span class="arrow">→</span>
            <span class="symbol">{rule.insert}</span>
          </label>
        </li>
      {/each}
    </ul>
    <p class="note first">Powers (<code>^2</code>) follow a letter, a digit or <code>)</code>; indices (<code>_2</code>) a letter.</p>
  {/if}
</main>

<style>
  main {
    padding: 28px 32px;
  }
  .option,
  li label {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .field + .option,
  .note + .option {
    margin-top: 10px;
  }
  input[type="checkbox"] {
    margin: 0;
    flex: none;
  }
  .note {
    color: var(--muted);
    margin: 6px 0 0 22px;
  }
  .note.first {
    margin-left: 0;
  }
  h2 {
    font-size: 13px;
    margin: 26px 0 0;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
  }
  .field label {
    width: 100px;
    color: var(--muted);
  }
  .folder {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  .folder.unset {
    color: var(--muted);
    direction: ltr;
  }
  .version {
    white-space: nowrap;
  }
  .status {
    flex: 1;
    min-width: 0;
    color: var(--muted);
  }
  .status.error {
    color: var(--error);
  }
  progress {
    width: 100%;
    margin-top: 8px;
  }
  select {
    font: inherit;
    color: var(--fg);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 4px;
  }
  input[type="range"] {
    width: 160px;
    margin: 0;
  }
  .size {
    min-width: 42px;
    font-variant-numeric: tabular-nums;
  }
  h3 {
    font-size: 12px;
    color: var(--muted);
    font-weight: 600;
    margin: 16px 0 4px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    columns: 2;
  }
  li {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 1px 0;
    break-inside: avoid;
  }
  code {
    font: 12px ui-monospace, Menlo, monospace;
  }
  .arrow {
    color: var(--muted);
  }
  .symbol {
    font-size: 14px;
  }
  li button {
    margin-left: auto;
  }
  form {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-top: 6px;
  }
  .field .name {
    width: 220px;
    -webkit-user-select: text;
    user-select: text;
  }
  form input {
    width: 90px;
    -webkit-user-select: text;
    user-select: text;
  }
  button {
    font: inherit;
    color: var(--fg);
    background: none;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 8px;
  }
  button:disabled {
    color: var(--muted);
  }
  li button {
    border: none;
    color: var(--muted);
    padding: 0 6px;
  }
  li button:hover {
    background: var(--chip);
  }
</style>
