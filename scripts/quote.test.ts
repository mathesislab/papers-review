// The quote of a selection in a PDF, with its page and line numbers (src/lib/quote.ts).

import { strict as assert } from "node:assert";
import { test } from "node:test";
import { lineNumber, location, quote, quoteText, type Line, type Selection } from "../src/lib/quote.ts";

// A line of text at x 100–300, with the numbers given printed in the margin at the given centres.
function line(page: number, text: string, margin: [string, number][] = [], label = ""): Line {
  return {
    page,
    label,
    text,
    bounds: { x: 100, y: 0, width: 200, height: 10 },
    margin: margin.map(([t, x]) => ({ text: t, x })),
  };
}

// Lines numbered from `from` on a page, each with its number in the left margin.
function numbered(page: number, from: number, texts: string[], label = ""): Line[] {
  return texts.map((t, i) => line(page, t, [[String(from + i), 80]], label));
}

const sel = (lines: Line[]): Selection => ({ text: lines.map((l) => l.text).join(" "), lines });

test("the number in the left margin", () => {
  assert.equal(lineNumber(line(5, "text", [["136", 80]])), 136);
});

test("the number nearest the selected part, left before right", () => {
  assert.equal(lineNumber(line(5, "t", [["12", 40], ["13", 90], ["14", 310]])), 13);
  // two columns, numbers in both margins: the left one even when the right is nearer
  assert.equal(lineNumber(line(5, "t", [["7", 50], ["8", 301]])), 7);
  assert.equal(lineNumber(line(5, "t", [["8", 301], ["9", 400]])), 8);
});

test("words that are not whole numbers are no line numbers", () => {
  assert.equal(lineNumber(line(5, "t", [["the", 80], ["3.5", 85], ["123456", 90]])), null);
});

test("no numbers: the page only", () => {
  assert.equal(location(sel([line(5, "a"), line(5, "b")])), "p. 5");
});

test("one line, many lines", () => {
  assert.equal(location(sel(numbered(5, 136, ["a"]))), "p. 5, l. 136");
  assert.equal(location(sel(numbered(5, 136, ["a", "b", "c"]))), "p. 5, ll. 136–138");
});

test("two columns with numbers in both margins", () => {
  const lines = [0, 1, 2].map((i) => line(5, "x", [[String(20 + i), 80], [String(70 + i), 320]]));
  assert.equal(location(sel(lines)), "p. 5, ll. 20–22");
});

test("across pages, numbering that goes on", () => {
  const lines = [...numbered(5, 227, ["a", "b", "c"]), ...numbered(6, 230, ["d", "e"])];
  assert.equal(location(sel(lines)), "pp. 5–6, ll. 227–231");
});

test("across pages, numbering that starts again", () => {
  const lines = [...numbered(5, 38, ["a", "b", "c"]), ...numbered(6, 1, ["d", "e", "f", "g"])];
  assert.equal(location(sel(lines)), "p. 5, l. 38 – p. 6, l. 4");
});

test("inconsistent numbers: the page only", () => {
  assert.equal(location(sel([line(5, "a", [["136", 80]]), line(5, "b", [["130", 80]])])), "p. 5");
  assert.equal(location(sel([line(5, "a", [["10", 80]]), line(5, "b", [["90", 80]])])), "p. 5");
  assert.equal(location(sel([line(5, "a", [["10", 80]]), line(5, "b")])), "p. 5");
  assert.equal(location(sel([line(5, "a", [["40", 80]]), line(6, "b", [["90", 80]])])), "pp. 5–6");
});

test("a line or two without a number is fine", () => {
  const lines = [line(5, "a", [["10", 80]]), line(5, "b"), line(5, "c", [["12", 80]])];
  assert.equal(location(sel(lines)), "p. 5, ll. 10–12");
});

test("the page label before the page number", () => {
  assert.equal(location(sel([line(3, "a", [], "iii")])), "p. iii");
  assert.equal(location(sel([line(3, "a", [], "iii"), line(4, "b", [], "iv")])), "pp. iii–iv");
  assert.equal(location(sel([line(3, "a", [], ""), line(4, "b", [], "4")])), "pp. 3–4");
});

test("the words of a line split by a hyphen are joined", () => {
  assert.equal(quoteText(sel([line(5, "as shown re-"), line(5, "cently by")])), "as shown recently by");
  // before a capital or a number the hyphen stays
  assert.equal(quoteText(sel([line(5, "the ice-"), line(5, "Sheet 3")])), "the ice- Sheet 3");
});

test("ligatures are normalised", () => {
  assert.equal(quoteText(sel([line(5, "the ﬁrst ﬂow")])), "the first flow");
});

test("a margin number the selection reached over is removed", () => {
  const lines = [
    line(5, "136 the ice sheet", [["136", 80]]),
    line(5, "retreats 137", [["137", 80]]),
    line(5, "in 2100 and 138 later", [["138", 80]]),
  ];
  assert.equal(quoteText(sel(lines)), "the ice sheet retreats in 2100 and 138 later");
});

test("whitespace is collapsed, and the text of the selection used without lines", () => {
  assert.equal(quoteText(sel([line(5, "  a \t b  "), line(5, "c\n d")])), "a b c d");
  assert.equal(quoteText({ text: " ﬁx   it ", lines: [] }), "fix it");
});

test("the quote", () => {
  const lines = numbered(5, 136, ["the ice re-", "cently lost"]);
  assert.equal(quote(sel(lines)), '**p. 5, ll. 136–137:** "the ice recently lost"');
  assert.equal(quote(sel([line(5, "a")])), '**p. 5:** "a"');
});
