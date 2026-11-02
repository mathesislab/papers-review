<script lang="ts">
  // The review pane: a web view of its own over the viewer of a project's window (src-tauri/src/pane.rs), at the
  // bottom or on the right. Its inner edge, dragged, sets its size; ⌘J shows or hides it, ⌘⇧J moves it.

  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import "$lib/theme.css";

  type Side = "bottom" | "right";
  let side = $state<Side>("bottom");
  /** its height at the bottom, its width on the right (points) */
  let size = $state(280);

  /** The inner edge dragged: the size follows, in points of the screen (the pane moves as it grows). */
  function drag(e: PointerEvent) {
    const edge = e.currentTarget as HTMLElement;
    edge.setPointerCapture(e.pointerId);
    const [startX, startY, start] = [e.screenX, e.screenY, size];
    let frame = 0;
    const to = (m: PointerEvent) => (side === "bottom" ? start - (m.screenY - startY) : start - (m.screenX - startX));
    const move = (m: PointerEvent) => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => invoke("pane_size", { size: to(m), done: false }));
    };
    const up = (m: PointerEvent) => {
      cancelAnimationFrame(frame);
      invoke("pane_size", { size: to(m), done: true });
      edge.removeEventListener("pointermove", move);
      edge.removeEventListener("pointerup", up);
    };
    edge.addEventListener("pointermove", move);
    edge.addEventListener("pointerup", up);
  }

  onMount(() => {
    const unlisten = listen<{ side: Side; size: number }>("pane-placed", (e) => {
      side = e.payload.side;
      size = e.payload.size;
    });
    // ⌘J hides the pane when it has the keys, else brings it forward with them
    const focused = () => invoke("pane_focus", { focused: true });
    const blurred = () => invoke("pane_focus", { focused: false });
    window.addEventListener("focus", focused);
    window.addEventListener("blur", blurred);
    return () => {
      unlisten.then((f) => f());
      window.removeEventListener("focus", focused);
      window.removeEventListener("blur", blurred);
    };
  });
</script>

<div class="pane {side}">
  <div class="edge" onpointerdown={drag} role="separator" aria-orientation={side === "bottom" ? "horizontal" : "vertical"}></div>
  <header>
    <span class="name">Review</span>
    <span class="meta"></span>
    <button onclick={() => invoke("pane_side")} title={side === "bottom" ? "To the Right (⌘⇧J)" : "To the Bottom (⌘⇧J)"}>
      {side === "bottom" ? "⇥" : "⤓"}
    </button>
    <button onclick={() => invoke("pane_hide")} title="Hide (⌘J)">✕</button>
  </header>
  <main></main>
</div>

<style>
  :global(html, body) {
    background: transparent;
  }
  .pane {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    /* square: the web view is not transparent, round corners would show its background */
    border: 1px solid var(--border);
    overflow: hidden;
    box-sizing: border-box;
  }
  .edge {
    position: absolute;
    z-index: 2;
  }
  .bottom .edge {
    top: 0;
    left: 0;
    right: 0;
    height: 6px;
    cursor: row-resize;
  }
  .right .edge {
    top: 0;
    bottom: 0;
    left: 0;
    width: 6px;
    cursor: col-resize;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 6px 12px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .name {
    font-weight: 600;
  }
  .meta {
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
  }
  header button {
    font: inherit;
    color: var(--muted);
    background: none;
    border: none;
    border-radius: 5px;
    padding: 1px 6px;
  }
  header button:hover {
    background: var(--chip);
  }
  main {
    flex: 1;
    min-height: 0;
  }
</style>
