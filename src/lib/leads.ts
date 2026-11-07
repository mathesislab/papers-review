// What a line of the review starts with up to its colon, coloured: the page and line of a PDF quote (`p5, l136–138:`),
// a label (`title:`, `Figure 3:`), or what has one of the lead words of the settings in it (`p4, Fig. 1, caption:`).
// The editor of papers-write knows nothing of quotes: this goes to it in `extensions` (makeEditor, FileEditor), for
// Markdown only.

import { Prec, StateEffect } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from "@codemirror/view";
import { leadPrefix } from "$lib/quote";

/** The lead words of the settings, the same in all the editors of the window (setLeadWords). */
let words: readonly string[] = [];
/** The editors open with the leads marked, marked again when the words change. */
const open = new Set<EditorView>();
const rewords = StateEffect.define<null>();

/** The lead words of the settings, for the editors of the window, those open marked again. */
export function setLeadWords(next: readonly string[]) {
  if (next.length === words.length && next.every((w, i) => w === words[i])) return;
  words = next;
  for (const view of open) view.dispatch({ effects: rewords.of(null) });
}

/** The lead of each line marked, in the lines in view. */
const marker = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(readonly view: EditorView) {
      this.decorations = mark(view);
      open.add(view);
    }
    update(update: ViewUpdate) {
      const reworded = update.transactions.some((tr) => tr.effects.some((e) => e.is(rewords)));
      if (update.docChanged || update.viewportChanged || reworded) this.decorations = mark(update.view);
    }
    destroy() {
      open.delete(this.view);
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
      const length = leadPrefix(line.text, words);
      if (length) ranges.push(Decoration.mark({ class: "cm-lead" }).range(line.from, line.from + length));
    }
  }
  return Decoration.set(ranges);
}

/** The extensions of the editor that colour the leads (low precedence: below the Markdown highlighting). */
export const leads = [Prec.low(marker), EditorView.theme({ ".cm-lead": { color: "var(--md-lead)", fontWeight: "550" } })];
