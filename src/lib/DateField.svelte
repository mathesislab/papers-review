<script lang="ts">
  // A date field: the date written in the format of the settings (2026-11-14, 14.11.2026 or 11/14/2026), read as typed
  // when the field is left or Enter is pressed (a date it cannot read is marked and the value stays), and a calendar
  // button that opens a month below it, weeks from Monday, today marked and the value highlighted. The value is the
  // ISO date (`2026-11-14`) or null. Esc closes the calendar.

  import { formatDate, parseDate } from "./due";
  import type { DateFormat } from "./types";

  let {
    value = $bindable(null),
    format,
    placeholder,
    id,
  }: {
    /** the date, `2026-11-14`, or null */
    value?: string | null;
    format: DateFormat;
    /** shown when empty; the format by default */
    placeholder?: string;
    id?: string;
  } = $props();

  const HINTS = { ymd: "YYYY-MM-DD", dmy: "DD.MM.YYYY", mdy: "MM/DD/YYYY" };

  /** what is in the field */
  let text = $state("");
  /** what was typed is not a date */
  let invalid = $state(false);
  let open = $state(false);
  /** the month shown (month 0 to 11) */
  let year = $state(0);
  let month = $state(0);
  let root: HTMLElement;

  // the value, or the format, changed: written again
  $effect(() => {
    text = value ? formatDate(value, format) : "";
    invalid = false;
  });

  const pad = (n: number) => String(n).padStart(2, "0");
  const iso = (y: number, m: number, d: number) => `${y}-${pad(m + 1)}-${pad(d)}`;

  /** Today, as `2026-11-14`. */
  const today = () => {
    const now = new Date();
    return iso(now.getFullYear(), now.getMonth(), now.getDate());
  };

  /** The text read: the value set, or the field marked if it is not a date. Nothing: no date. */
  function commit() {
    if (!text.trim()) {
      value = null;
      invalid = false;
      return;
    }
    const date = parseDate(text, format);
    invalid = date === null;
    if (date === null) return;
    value = date;
    text = formatDate(date, format);
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      commit();
      // not a date: the form is not submitted
      if (invalid) e.preventDefault();
    } else if (e.key === "Escape" && open) {
      open = false;
      e.stopPropagation();
    }
  }

  function toggle() {
    if (!open) {
      const [y, m] = (value ?? today()).split("-").map(Number);
      [year, month] = [y, m - 1];
    }
    open = !open;
  }

  /** The month `by` months on. */
  function step(by: number) {
    const date = new Date(Date.UTC(year, month + by, 1));
    [year, month] = [date.getUTCFullYear(), date.getUTCMonth()];
  }

  function pick(day: number) {
    value = iso(year, month, day);
    open = false;
  }

  const title = $derived(new Date(Date.UTC(year, month, 1)).toLocaleDateString(undefined, { month: "long", year: "numeric", timeZone: "UTC" }));
  // 2024-01-01 was a Monday
  const weekdays = Array.from({ length: 7 }, (_, i) =>
    new Date(Date.UTC(2024, 0, 1 + i)).toLocaleDateString(undefined, { weekday: "narrow", timeZone: "UTC" }),
  );
  /** The days of the month, after the empty places of the week it starts in. */
  const days = $derived.by(() => {
    const lead = (new Date(Date.UTC(year, month, 1)).getUTCDay() + 6) % 7;
    const count = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
    return [...Array<null>(lead).fill(null), ...Array.from({ length: count }, (_, i) => i + 1)];
  });
</script>

<svelte:window onpointerdown={(e) => open && !root.contains(e.target as Node) && (open = false)} />

<div class="date" bind:this={root}>
  <div class="field" class:invalid>
    <input
      {id}
      bind:value={text}
      placeholder={placeholder ?? HINTS[format]}
      autocomplete="off"
      spellcheck="false"
      aria-invalid={invalid}
      onblur={commit}
      onkeydown={keydown}
    />
    {#if value}<button type="button" class="plain" onclick={() => (value = null)} title="Clear" aria-label="Clear">✕</button>{/if}
    <button type="button" class="plain" onclick={toggle} title="Calendar" aria-label="Calendar" aria-expanded={open}>
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
        <rect x="1.5" y="2.5" width="10" height="9" rx="1.5" />
        <path d="M1.5 5.5h10M4 1v3M9 1v3" />
      </svg>
    </button>
  </div>
  {#if open}
    <div class="calendar" role="dialog" aria-label="Calendar">
      <header>
        <button type="button" class="plain" onclick={() => step(-1)} aria-label="Previous Month">‹</button>
        <span class="month">{title}</span>
        <button type="button" class="plain" onclick={() => step(1)} aria-label="Next Month">›</button>
      </header>
      <div class="days">
        {#each weekdays as w, i (i)}<span class="weekday">{w}</span>{/each}
        {#each days as day, i (i)}
          {#if day}
            {@const date = iso(year, month, day)}
            <button type="button" class="day" class:today={date === today()} class:on={date === value} onclick={() => pick(day)}>{day}</button>
          {:else}
            <span></span>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .date {
    position: relative;
    display: inline-block;
  }
  .field {
    display: flex;
    align-items: center;
    box-sizing: border-box;
    width: 190px;
    height: 26px;
    padding-right: 2px;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .field:focus-within {
    border-color: var(--accent);
  }
  .field.invalid {
    border-color: var(--error);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 8px;
    border: none;
    background: transparent;
    color: var(--fg);
    font: inherit;
    outline: none;
    -webkit-user-select: text;
    user-select: text;
  }
  button {
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
  }
  button.plain {
    display: flex;
    align-items: center;
    padding: 0 5px;
  }
  button.plain:hover {
    color: var(--fg);
  }
  .calendar {
    position: absolute;
    z-index: 10;
    top: calc(100% + 4px);
    left: 0;
    width: 214px;
    box-sizing: border-box;
    padding: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }
  header button {
    font-size: 16px;
    line-height: 1;
    padding: 2px 8px;
  }
  .month {
    font-weight: 600;
  }
  .days {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
    text-align: center;
  }
  .weekday {
    color: var(--muted);
    font-size: 11px;
    padding-bottom: 2px;
  }
  .day {
    height: 24px;
    padding: 0;
    border-radius: 5px;
    color: var(--fg);
  }
  .day:hover {
    background: var(--chip);
  }
  .day.today {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .day.on {
    background: var(--accent);
    color: var(--on-accent);
  }
</style>
