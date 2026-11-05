// The page and line of a PDF quote at the start of a line (`p5, l136–138:`), coloured in the review. The editor of
// papers-write knows nothing of quotes: this goes to it in `extensions` (makeEditor, FileEditor), for Markdown only.

import { Prec } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from "@codemirror/view";
import { locationPrefix } from "$lib/quote";

/** The page and line of a quote at the start of a line marked, in the lines in view. */
const marker = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = mark(view);
    }
    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged) this.decorations = mark(update.view);
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

function mark(view: EditorView): DecorationSet {
  const ranges = [];
  let last = -1;
  for (const { from, to } of view.visibleRanges) {
    for (let pos = from; pos <= to; ) {
      const line = view.state.doc.lineAt(pos);
      pos = line.to + 1;
      if (line.from <= last) continue;
      last = line.from;
      const length = locationPrefix(line.text);
      if (length) ranges.push(Decoration.mark({ class: "cm-location" }).range(line.from, line.from + length));
    }
  }
  return Decoration.set(ranges);
}

/** The extensions of the editor that colour the locations (low precedence: below the Markdown highlighting). */
export const locations = [Prec.low(marker), EditorView.theme({ ".cm-location": { color: "var(--md-location)", fontWeight: "550" } })];
