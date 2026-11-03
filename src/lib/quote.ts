// A quote from a paper, for a review: the part the reader selected in the PDF, with where it lies (page, and the line
// numbers printed in the margin when the paper has them). The types mirror those of tauri-plugin-pdfkit.

/** A rectangle in page points. */
export type Rect = { x: number; y: number; width: number; height: number };

/** A word on the same band of the page outside the selected part; x is its centre. */
export type Run = { text: string; x: number };

/** A line of the selection: its page (from 1), the printed page label ("" if none), its text and place, and the
 *  words beside it. */
export type Line = { page: number; label: string; text: string; bounds: Rect; margin: Run[] };

/** What the reader selected: its text, and the lines it spans. */
export type Selection = { text: string; lines: Line[] };

const NUMBER = /^\d{1,5}$/;
const HYPHEN = /[-‐­]$/;
const LOWER = /^\p{Ll}/u;

/** The line number printed in the margin beside a line: the whole number nearest the selected part, one to the
 *  left before one to the right. */
export function lineNumber(line: Line): number | null {
  const { x, width } = line.bounds;
  const side = (r: Run) => (r.x < x ? 0 : r.x > x + width ? 1 : 2);
  const gap = (r: Run) => (r.x < x ? x - r.x : r.x > x + width ? r.x - x - width : 0);
  const found = line.margin
    .filter((r) => NUMBER.test(r.text.trim()))
    .sort((a, b) => side(a) - side(b) || gap(a) - gap(b))[0];
  return found ? Number(found.text.trim()) : null;
}

const pageOf = (line: Line) => line.label || String(line.page);

/** Where a selection lies: "p. 5", "p. 5, l. 136", "p. 5, ll. 136–138", "pp. 5–6", "pp. 5–6, ll. 210–228", or
 *  "p. 5, l. 38 – p. 6, l. 4" where each page is numbered from 1. */
export function location(selection: Selection): string {
  const { lines } = selection;
  if (lines.length === 0) return "";
  const first = lines[0];
  const last = lines[lines.length - 1];
  const samePage = first.page === last.page;
  const pages = samePage ? `p. ${pageOf(first)}` : `pp. ${pageOf(first)}–${pageOf(last)}`;
  const a = lineNumber(first);
  const b = lineNumber(last);
  if (a === null || b === null || !consistent(lines, a, b, samePage)) return pages;
  // numbered again from the next page: each end with its page
  if (b < a) return `p. ${pageOf(first)}, l. ${a} – p. ${pageOf(last)}, l. ${b}`;
  return a === b ? `${pages}, l. ${a}` : `${pages}, ll. ${a}–${b}`;
}

// Whether the numbers of the first and the last line fit the lines between them (a line or two may have none).
// Across pages the numbering either goes on or starts again on each page.
function consistent(lines: Line[], a: number, b: number, samePage: boolean): boolean {
  const near = (span: number, count: number) => Math.abs(span - count) <= 2;
  if (samePage) return b >= a && near(b - a + 1, lines.length);
  const lastPage = lines[lines.length - 1].page;
  const onLast = lines.filter((l) => l.page === lastPage).length;
  const continues = b >= a && near(b - a + 1, lines.length);
  const restarts = near(b, onLast);
  return continues || restarts;
}

/** The text of the selection as a quotation: its lines joined, words split by a hyphen at a line end made whole,
 *  margin numbers dropped. */
export function quoteText(selection: Selection): string {
  const { lines } = selection;
  if (lines.length === 0) return clean(selection.text);
  let out = "";
  for (const line of lines) {
    const text = clean(withoutNumber(line.text.normalize("NFKC"), lineNumber(line)));
    if (!text) continue;
    if (HYPHEN.test(out) && LOWER.test(text)) out = out.slice(0, -1) + text;
    else out = out ? `${out} ${text}` : text;
  }
  return clean(out);
}

/** The selection as a line of a review: where it lies, then its text in quotation marks. */
export function quote(selection: Selection): string {
  const where = location(selection);
  const text = `"${quoteText(selection)}"`;
  return where ? `**${where}:** ${text}` : text;
}

const clean = (text: string) => text.normalize("NFKC").replace(/\s+/g, " ").trim();

// The text of a line without its margin number, when the selection reached over it to the start or the end.
function withoutNumber(text: string, n: number | null): string {
  if (n === null) return text;
  const tokens = text.trim().split(/\s+/);
  if (tokens[0] === String(n)) tokens.shift();
  if (tokens[tokens.length - 1] === String(n)) tokens.pop();
  return tokens.join(" ");
}
