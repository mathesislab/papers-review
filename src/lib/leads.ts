// What a line of the review starts with up to its colon, coloured: the page and line of a PDF quote (`p5, l136–138:`),
// or a label (`title:`, `Figure 3:`). The editor of papers-write knows nothing of quotes: this goes to it in `extensions` (makeEditor, FileEditor), for Markdown only.

import { Prec } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from "@codemirror/view";
import { leadPrefix } from "$lib/quote";

/** The lead of each line marked, in the lines in view. */
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
      const length = leadPrefix(line.text);
      if (length) ranges.push(Decoration.mark({ class: "cm-lead" }).range(line.from, line.from + length));
    }
  }
  return Decoration.set(ranges);
}

/** The extensions of the editor that colour the leads (low precedence: below the Markdown highlighting). */
export const leads = [Prec.low(marker), EditorView.theme({ ".cm-lead": { color: "var(--md-lead)", fontWeight: "550" } })];
