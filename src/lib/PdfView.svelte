<script lang="ts">
  // A PDF in PDFKit, as Preview shows it (tauri-plugin-pdfkit): a native view over the area of this element, kept in
  // place as the area changes; the page in view told back (`onpage`), the outline as the PDF opens. The commands go
  // to the view in order: a search typed letter by letter must not land after the next one.

  import { onMount } from "svelte";
  import * as pdfkit from "tauri-plugin-pdfkit-api";
  import type { Entry } from "tauri-plugin-pdfkit-api";

  let {
    path,
    page = 1,
    onpage,
    onopen,
  }: {
    path: string;
    /** the page to open at (from 1) */
    page?: number;
    /** the page in view of the PDF of `path` */
    onpage: (path: string, page: number) => void;
    /** the PDF open: its page count and outline */
    onopen: (count: number, outline: Entry[]) => void;
  } = $props();

  let area: HTMLDivElement;
  let error = $state("");
  let queue: Promise<unknown> = Promise.resolve();

  function send<T>(command: () => Promise<T>): Promise<T> {
    const sent = queue.then(command);
    queue = sent.catch(() => {});
    return sent;
  }

  /** In (1), out (-1), the width of the area (0). */
  export function zoom(step: number) {
    send(() => pdfkit.zoom(step));
  }

  /** The keys to the PDF. */
  export function focus() {
    send(() => pdfkit.cover(path, false));
  }

  /** To an entry of the outline, by its `target`. */
  export function go(target: number[]) {
    send(() => pdfkit.go(target));
  }

  /** The text selected in the PDF, line by line, with what lies in the margin beside each line; null if none. */
  export function selection(): Promise<pdfkit.Selection | null> {
    return send(() => pdfkit.selection());
  }

  /** Find `text`: the first place, or `again` the next (the one before, `backwards`); an empty text clears. */
  export function find(text: string, again = false, backwards = false): Promise<boolean> {
    return send(() => pdfkit.find(text, again, backwards));
  }

  onMount(() => {
    // the PDF this view opened: as the tab changes, `path` is already the next file's when the view goes
    const shown = path;
    let opened = false;
    // the area changes with the files panel, the tabs, the window
    const resize = new ResizeObserver(() => {
      if (opened) send(() => pdfkit.setFrame(pdfkit.frameOf(area)));
    });
    resize.observe(area);
    const unlisten = pdfkit.onPage((p) => onpage(shown, p));
    send(() => pdfkit.open(shown, shown, page, pdfkit.frameOf(area)))
      .then(({ count, outline }) => {
        opened = true;
        onopen(count, outline);
      })
      .catch((e) => (error = String(e)));
    return () => {
      resize.disconnect();
      unlisten.then((f) => f());
      send(() => pdfkit.close(shown));
    };
  });
</script>

<div class="pdf" bind:this={area}>
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .pdf {
    position: absolute;
    inset: 0;
  }
  .error {
    color: var(--error);
    text-align: center;
    margin-top: 40px;
  }
</style>
