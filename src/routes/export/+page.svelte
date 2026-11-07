<script lang="ts">
  // The page a PDF is printed from (src-tauri), in a hidden window: the HTML of the export (src/lib/export.ts,
  // pdfDocument) set in the page and laid out as a sheet of paper, A4; when the fonts and the layout are settled the
  // app is told, which prints it. The margins are the printing's own, so the page has none.

  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";

  let html = $state("");

  onMount(async () => {
    html = await invoke<string>("export_html");
    await tick();
    await document.fonts.ready;
    // the layout done now: a window never shown may get no animation frames to wait for
    void document.body.offsetHeight;
    await invoke("export_rendered");
  });
</script>

{@html html}

<style>
  @page {
    size: A4;
  }
  :global(html, body) {
    margin: 0;
    background: #fff;
    color: #111;
  }
  :global(.page) {
    font: 11pt/1.45 "New York", Georgia, serif;
    hyphens: auto;
    -webkit-hyphens: auto;
    orphans: 3;
    widows: 3;
  }

  /* the header: the heading, and below it the details */
  :global(.page > header) {
    margin-bottom: 1.6em;
    padding-bottom: 0.7em;
    border-bottom: 0.5pt solid #999;
  }
  :global(.page .title) {
    margin: 0;
    font: 600 18pt/1.2 -apple-system, "Helvetica Neue", Helvetica, sans-serif;
  }
  :global(.page .details) {
    margin: 0.35em 0 0;
    font: 9.5pt/1.3 -apple-system, "Helvetica Neue", Helvetica, sans-serif;
    color: #666;
  }

  /* the review */
  :global(.page .review h1, .page .review h2, .page .review h3, .page .review h4, .page .review h5, .page .review h6) {
    font-family: -apple-system, "Helvetica Neue", Helvetica, sans-serif;
    font-weight: 600;
    line-height: 1.25;
    margin: 1.4em 0 0.5em;
    break-after: avoid;
    page-break-after: avoid;
  }
  :global(.page .review > :first-child) {
    margin-top: 0;
  }
  :global(.page .review h1) {
    font-size: 15pt;
  }
  :global(.page .review h2) {
    font-size: 13pt;
  }
  :global(.page .review h3) {
    font-size: 11.5pt;
  }
  :global(.page .review h4, .page .review h5, .page .review h6) {
    font-size: 11pt;
  }
  :global(.page .review p) {
    margin: 0 0 0.75em;
  }
  /* what a line of the review starts with: a location, a label */
  :global(.page .lead) {
    color: #777;
  }
  :global(.page .review blockquote) {
    margin: 0 0 0.75em;
    padding-left: 1em;
    border-left: 2pt solid #ccc;
    color: #333;
  }
  /* a quote or a labelled paragraph (one with a lead) is not split between two pages, if it fits on one */
  :global(.page .review p:has(.lead), .page .review blockquote) {
    break-inside: avoid-page;
    page-break-inside: avoid;
  }
  :global(.page .review ul, .page .review ol) {
    margin: 0 0 0.75em;
    padding-left: 1.5em;
  }
  :global(.page .review li) {
    margin: 0.15em 0;
  }
  :global(.page .review li > p) {
    margin: 0.3em 0;
  }
  :global(.page .review li > ul, .page .review li > ol) {
    margin: 0.15em 0 0;
  }
  :global(.page .review code) {
    font: 0.9em ui-monospace, Menlo, monospace;
  }
  :global(.page .review pre) {
    margin: 0 0 0.75em;
    padding: 0.5em 0.75em;
    background: #f4f4f4;
    font: 9.5pt/1.35 ui-monospace, Menlo, monospace;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    break-inside: avoid-page;
    page-break-inside: avoid;
  }
  :global(.page .review pre code) {
    font: inherit;
  }
  :global(.page .review a) {
    color: inherit;
    text-decoration: underline;
    text-decoration-color: #aaa;
    overflow-wrap: anywhere;
  }
  :global(.page .review hr) {
    margin: 1.2em 0;
    border: none;
    border-top: 0.5pt solid #bbb;
  }

  /* the name at the end */
  :global(.page footer) {
    margin-top: 2em;
    break-inside: avoid;
    page-break-inside: avoid;
  }
  :global(.page .signature) {
    margin: 0;
  }
</style>
