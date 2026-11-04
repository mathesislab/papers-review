<script lang="ts">
  // The Settings window (⌘,), set out as Papers Desktop's: a bar of tabs (⌘1…⌘3), the tab shown, which scrolls, and
  // the line at its foot. General: the colour of the windows' bar, whether the projects open in one window with a bar
  // to switch between them, the folder new reviews are made in, how dates are written; the exports of the review (the
  // reviewer's name, whether the PDF is signed with it, whether its header has the journal, the manuscript ID and the
  // date); the updates (`#updates`: the version running, Check Now, the one found to install and its download, whether
  // to look for them). Editor: the font and size of the editors, what Tab puts in, the readable line length, the line
  // numbers. Symbols: the sequences typed in the review that become symbols, the defaults each turned off if not
  // wanted, and the user's own. Kept in the app's data folder (settings.json); the windows are told as it changes.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, tick } from "svelte";
  import "$lib/theme.css";
  import "$lib/settings.css";
  import { COLOURS } from "$lib/colour";
  import { DEFAULTS } from "$lib/symbols";
  import { Updates } from "$lib/updates.svelte";
  import type { Colour, DateFormat, EditorFont, Indent, Settings } from "$lib/types";

  /** the tabs, in the order of the bar and of their keys */
  const TABS = [
    { id: "general", name: "General" },
    { id: "editor", name: "Editor" },
    { id: "symbols", name: "Symbols" },
  ] as const;
  type Tab = (typeof TABS)[number]["id"];
  /** the sections of the General tab the app may name (`settings#updates`): shown scrolled to */
  const GENERAL_SECTIONS = ["updates"];

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
  let tab = $state<Tab>("general");
  /** the sequence and the replacement of a pair being added */
  let keys = $state("");
  let insert = $state("");
  const addable = $derived(keys.length >= 2 && insert !== "");

  function save() {
    if (settings) invoke("set_settings", { settings: $state.snapshot(settings) });
  }

  function setColour(colour: Colour) {
    if (!settings) return;
    settings.window_colour = colour;
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

  /** Show a section the app names: a tab, or a section of the General tab, scrolled to; the updates also look for
   *  one unless found (Check for Updates… of the menu). */
  async function show(section: string) {
    if (GENERAL_SECTIONS.includes(section)) {
      tab = "general";
      await tick();
      document.getElementById(section)?.scrollIntoView({ block: "start" });
      if (section === "updates") updates.lookFor();
      return;
    }
    const named = TABS.find((t) => t.id === section);
    if (named) tab = named.id;
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

  /** ⌘1…⌘3 show a tab. Esc with no field in focus closes the window (a field gives up the focus first). */
  function key(e: KeyboardEvent) {
    const n = e.metaKey && !e.shiftKey && !e.altKey && !e.ctrlKey ? Number(e.code.replace("Digit", "")) : NaN;
    if (n >= 1 && n <= TABS.length) {
      e.preventDefault();
      tab = TABS[n - 1].id;
      return;
    }
    if (e.key !== "Escape") return;
    const field = document.activeElement;
    if (field instanceof HTMLElement && field !== document.body) field.blur();
    else getCurrentWindow().close();
  }
</script>

<svelte:window onkeydown={key} />

{#if settings}
  <main class="settings">
    <div class="tabs" role="tablist" aria-label="Settings">
      {#each TABS as t, i (t.id)}
        <button
          role="tab"
          id="tab-{t.id}"
          aria-selected={tab === t.id}
          aria-controls="panel-{t.id}"
          class:on={tab === t.id}
          title="⌘{i + 1}"
          onclick={() => (tab = t.id)}>{t.name}</button
        >
      {/each}
    </div>

    <div class="panel" role="tabpanel" id="panel-general" aria-labelledby="tab-general" hidden={tab !== "general"}>
      <section>
        <h2>General</h2>
        <div class="row">
          <span>
            <span class="label">Window colour</span>
            <span class="hint">A thin bar at the top of the windows, to tell them apart; also a right-click on the bar</span>
          </span>
          <span class="swatches" role="radiogroup" aria-label="Window colour">
            {#each COLOURS as c (c.colour)}
              <button
                class="swatch"
                class:on={settings.window_colour === c.colour}
                class:none={!c.css}
                style:--c={c.css}
                role="radio"
                aria-checked={settings.window_colour === c.colour}
                title={c.name}
                aria-label={c.name}
                onclick={() => setColour(c.colour)}
              ></button>
            {/each}
          </span>
        </div>
        <label class="row">
          <span>
            <span class="label">Projects in one window</span>
            <span class="hint">With a bar to switch between them; off: each in a window of its own</span>
          </span>
          <input type="checkbox" class="switch" bind:checked={settings.project_bar} onchange={save} />
        </label>
        <div class="row">
          <span class="grow">
            <span class="label">Reviews folder</span>
            <span class="hint folder" class:unset={!settings.reviews_folder} title={settings.reviews_folder ?? ""}>
              {settings.reviews_folder ?? "Not chosen yet: asked for with the first review"}
            </span>
          </span>
          <button onclick={chooseReviews}>Choose…</button>
        </div>
        <label class="row">
          <span>
            <span class="label">Date format</span>
            <span class="hint">The due dates of the projects</span>
          </span>
          <select bind:value={settings.date_format} onchange={save}>
            {#each FORMATS as [format, example] (format)}<option value={format}>{example}</option>{/each}
          </select>
        </label>
      </section>

      <section>
        <h2>Export</h2>
        <label class="row">
          <span class="label">Reviewer</span>
          <input class="text name" bind:value={settings.reviewer_name} onchange={save} placeholder="Your name" autocomplete="off" spellcheck="false" />
        </label>
        <label class="row">
          <span>
            <span class="label">Sign the reviews with my name</span>
            <span class="hint">Off: the PDF is anonymous</span>
          </span>
          <input type="checkbox" class="switch" bind:checked={settings.sign} onchange={save} />
        </label>
        <label class="row">
          <span>
            <span class="label">Header with journal, manuscript ID and date</span>
            <span class="hint">Off: the PDF is headed by the project's heading alone</span>
          </span>
          <input type="checkbox" class="switch" bind:checked={settings.header_details} onchange={save} />
        </label>
      </section>

      <section id="updates">
        <h2>Updates</h2>
        <div class="row">
          <span>
            <span class="label">Version {updates.version}</span>
            <span class="hint" class:error={updates.updating === "failed"}>{updates.status}</span>
          </span>
          {#if updates.update}
            <button class="primary" disabled={updates.updating === "installing"} onclick={updates.install}>
              Install {updates.update.version} and Restart
            </button>
          {:else}
            <button disabled={updates.updating === "checking"} onclick={updates.check}>Check for Updates Now</button>
          {/if}
        </div>
        {#if updates.updating === "installing"}
          <progress value={updates.progress.done} max={updates.progress.total ?? undefined}></progress>
        {/if}
        {#if updates.installed}
          <p class="news">Updated from {updates.installed.from}.</p>
        {/if}
        <label class="row">
          <span>
            <span class="label">Check for updates automatically</span>
            <span class="hint">At start and twice a day; installed only when you ask</span>
          </span>
          <input type="checkbox" class="switch" bind:checked={settings.check_updates} onchange={save} />
        </label>
      </section>
    </div>

    <div class="panel" role="tabpanel" id="panel-editor" aria-labelledby="tab-editor" hidden={tab !== "editor"}>
      <section>
        <h2>Editor</h2>
        <label class="row">
          <span>
            <span class="label">Font</span>
            <span class="hint">The review and the text files of a project</span>
          </span>
          <select bind:value={settings.editor_font} onchange={save}>
            {#each FONTS as [font, name] (font)}<option value={font}>{name}</option>{/each}
          </select>
        </label>
        <label class="row">
          <span class="label">Size</span>
          <span class="size">
            <input type="range" min="11" max="20" step="0.5" bind:value={settings.editor_size} onchange={save} />
            <span class="points">{settings.editor_size} pt</span>
          </span>
        </label>
        <label class="row">
          <span>
            <span class="label">Tab inserts</span>
            <span class="hint">With lines selected, Tab indents them and ⇧Tab takes it out</span>
          </span>
          <select bind:value={settings.indent} onchange={save}>
            {#each INDENTS as [indent, name] (indent)}<option value={indent}>{name}</option>{/each}
          </select>
        </label>
        <label class="row">
          <span>
            <span class="label">Readable line length</span>
            <span class="hint">The text a column in the middle, with a margin on each side; off: as wide as the window</span>
          </span>
          <input type="checkbox" class="switch" bind:checked={settings.readable_width} onchange={save} />
        </label>
        <label class="row">
          <span class="label">Line numbers</span>
          <input type="checkbox" class="switch" bind:checked={settings.line_numbers} onchange={save} />
        </label>
      </section>
    </div>

    <div class="panel" role="tabpanel" id="panel-symbols" aria-labelledby="tab-symbols" hidden={tab !== "symbols"}>
      <section>
        <h2>Typed symbols</h2>
        <p class="intro">
          In the review, a sequence typed becomes its symbol as its last character is typed; ⌘Z gives back what was
          typed. Tab after a LaTeX name (<code>\alpha</code>) gives its symbol; ⌘/ or the Ω button lists them.
        </p>
        <h3>Your own</h3>
        <ul class="pairs">
          {#each settings.symbols.filter(([, v]) => v) as [sequence, symbol] (sequence)}
            <li>
              <code>{sequence}</code>
              <span class="arrow">→</span>
              <span class="symbol">{symbol}</span>
              <button class="remove" onclick={() => remove(sequence)} title="Remove" aria-label="Remove {sequence}">✕</button>
            </li>
          {:else}
            <li class="none">None yet</li>
          {/each}
        </ul>
        <form
          onsubmit={(e) => {
            e.preventDefault();
            add();
          }}
        >
          <input class="text" bind:value={keys} placeholder="Typed" spellcheck="false" autocomplete="off" />
          <span class="arrow">→</span>
          <input class="text" bind:value={insert} placeholder="Becomes" spellcheck="false" autocomplete="off" />
          <button type="submit" disabled={!addable}>Add</button>
        </form>
        <p class="hint">At least two characters. A sequence that is a default gives it another symbol.</p>
      </section>

      <section>
        <h2>Defaults</h2>
        <ul class="pairs">
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
        <p class="hint">Powers (<code>^2</code>) follow a letter, a digit or <code>)</code>; indices (<code>_2</code>) a letter.</p>
      </section>
    </div>

    <footer>Changes apply at once.</footer>
  </main>
{/if}

<style>
  :global(html, body) {
    margin: 0;
    background: var(--bg);
    color: var(--fg);
    font-size: 13px;
    cursor: default;
    -webkit-user-select: none;
    user-select: none;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  /* the bar of tabs: the one shown lifted, in the accent */
  .tabs {
    flex: none;
    display: flex;
    justify-content: center;
    gap: 2px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--line);
    background: var(--sidebar);
  }
  .tabs button {
    padding: 4px 12px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: none;
    color: var(--muted);
  }
  .tabs button:hover {
    color: var(--fg);
  }
  .tabs button.on {
    border-color: var(--line);
    background: var(--menu-bg);
    color: var(--accent);
  }

  /* a tab: its cards, scrolled on their own */
  .panel {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 22px;
  }
  .panel[hidden] {
    display: none;
  }

  footer {
    flex: none;
    padding: 8px 22px 10px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 11.5px;
    text-align: center;
  }

  .grow {
    flex: 1;
    min-width: 0;
  }
  /* the folder's path, its end kept in view */
  .folder {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  .folder.unset {
    direction: ltr;
  }
  .text.name {
    font-family: inherit;
    font-size: inherit;
    -webkit-user-select: text;
    user-select: text;
  }
  .size {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  input[type="range"] {
    width: 150px;
    margin: 0;
  }
  .points {
    min-width: 42px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  progress {
    width: 100%;
    margin: 2px 0 8px;
    accent-color: var(--accent);
  }
  .news {
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--muted);
  }

  /* the colours of the windows' bar: a dot each, the one chosen ringed; none crossed */
  .swatches {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }
  .swatch {
    width: 16px;
    height: 16px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 50%;
    background: var(--c);
    box-shadow: 0 0 0 1px var(--border);
    cursor: pointer;
  }
  .swatch.none {
    background: linear-gradient(135deg, transparent 45%, var(--muted) 45%, var(--muted) 55%, transparent 55%);
  }
  .swatch.on {
    border-color: var(--bg);
    box-shadow: 0 0 0 2px var(--fg);
  }

  /* the symbols */
  .intro {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
  }
  .pairs {
    list-style: none;
    margin: 4px 0 8px;
    padding: 0;
    columns: 2;
  }
  .pairs li,
  .pairs label {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 1px 0;
    break-inside: avoid;
  }
  .pairs .none {
    color: var(--muted);
  }
  .pairs input[type="checkbox"] {
    margin: 0;
  }
  .pairs .remove {
    margin-left: auto;
  }
  .arrow {
    color: var(--muted);
  }
  .symbol {
    font-size: 14px;
  }
  form {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 0 6px;
  }
  form .text {
    width: 90px;
    -webkit-user-select: text;
    user-select: text;
  }
</style>
