// The quote of a selection in a PDF, with its page and line numbers (src/lib/quote.ts).

import { strict as assert } from "node:assert";
import { test } from "node:test";
import { leadPrefix, lineNumber, location, numberAt, quote, quoteText, type Line, type Mark, type Selection } from "../src/lib/quote.ts";

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

const sel = (lines: Line[], numbers: Mark[] = []): Selection => ({ text: lines.map((l) => l.text).join(" "), lines, numbers });

// A page whose lines lie 12 points apart from the top (line k centred at 800 - 12 k), selected from x 100 to 300 (or
// from `x`), with nothing printed beside them.
const SPACING = 12;
const centre = (k: number) => 800 - SPACING * k;
function lineAt(page: number, k: number, x = 100): Line {
  return { page, label: "", text: `line ${k}`, bounds: { x, y: centre(k) - 5, width: 300 - x, height: 10 }, margin: [] };
}
// The numbers printed every fifth line, `from` to `to`, at x (left margin 80), each centred on its line; `shift`: the
// lines from `after` on lie further down by that many lines (an equation between).
function every5(page: number, from: number, to: number, x = 80, after = Infinity, shift = 0): Mark[] {
  const marks: Mark[] = [];
  for (let n = from; n <= to; n += 5) marks.push({ page, n, x, y: centre(n + (n >= after ? shift : 0)) });
  return marks;
}

test("the number in the left margin", () => {
  assert.equal(lineNumber(line(5, "text", [["136", 80]])), 136);
});

test("the number nearest the selected part, left before right", () => {
  assert.equal(lineNumber(line(5, "t", [["12", 40], ["13", 90], ["14", 310]])), 13);
  // two columns, numbers in both margins: the left one even when the right is nearer
  assert.equal(lineNumber(line(5, "t", [["7", 50], ["8", 301]])), 7);
  assert.equal(lineNumber(line(5, "t", [["8", 301], ["9", 400]])), 8);
});

test("a number in the text is no line number: words lie beyond it", () => {
  // "… 3000, and 10000 m), see details in [25]. The ECs", ECs selected at the end of a line without numbers
  const text: [string, number][] = [["400,", 20], ["3000,", 40], ["and", 50], ["10000", 60], ["m),", 70], ["see", 80]];
  assert.equal(lineNumber(line(7, "ECs", text)), null);
  assert.equal(location(sel([line(7, "ECs", text)])), "p7");
  // with a number in the margin beyond the words, that one
  assert.equal(lineNumber(line(7, "ECs", [["212", 5], ...text])), 212);
});

test("words that are not whole numbers are no line numbers", () => {
  assert.equal(lineNumber(line(5, "t", [["the", 80], ["3.5", 85], ["123456", 90]])), null);
});

test("no numbers: the page only", () => {
  assert.equal(location(sel([line(5, "a"), line(5, "b")])), "p5");
});

test("numbered every fifth line: the lines between counted from the nearest number", () => {
  const numbers = every5(5, 5, 30);
  assert.equal(location(sel([lineAt(5, 11), lineAt(5, 12), lineAt(5, 13)], numbers)), "p5, l11–13");
  assert.equal(location(sel([lineAt(5, 7)], numbers)), "p5, l7");
  assert.equal(location(sel([lineAt(5, 9), lineAt(5, 10), lineAt(5, 11)], numbers)), "p5, l9–11");
});

test("numbered every fifth line: above the first number and below the last", () => {
  const numbers = every5(5, 5, 20);
  assert.equal(numberAt(lineAt(5, 1), numbers), 1);
  assert.equal(numberAt(lineAt(5, 23), numbers), 23);
});

test("numbered every fifth line: an equation that widens a gap leaves the spacing", () => {
  // an equation three lines high between lines 14 and 15
  const numbers = every5(5, 5, 25, 80, 15, 3);
  assert.equal(numberAt(lineAt(5, 14), numbers), 14);
  // line 16 lies where 19 would without the equation
  assert.equal(numberAt(lineAt(5, 19), numbers), 16);
});

test("numbered every fifth line: numbers in the text, a page number and other pages are no column", () => {
  const others: Mark[] = [
    { page: 5, n: 10000, x: 150, y: centre(8) },
    { page: 5, n: 1, x: 200, y: 40 },
    { page: 5, n: 1, x: 95, y: centre(1) },
    ...every5(6, 5, 30, 90),
  ];
  const numbers = [...every5(5, 5, 30), ...others];
  // a selection from the middle of a line, a number in the text left of it
  assert.equal(numberAt(lineAt(5, 12, 180), numbers), 12);
  assert.equal(numberAt(lineAt(5, 12), others), null);
  // a column of one number only: no spacing
  assert.equal(numberAt(lineAt(5, 12), every5(5, 10, 10)), null);
});

test("numbered every fifth line: a manuscript of LaTeX (lineno, \\modulolinenumbers[5]) as the plugin reads it", () => {
  // page 1: the section number, 5, 10000 in the text, 10, an equation between 14 and 15, 15, the page number
  const numbers: Mark[] = [
    { page: 1, n: 1, x: 74.9, y: 763.65 },
    { page: 1, n: 5, x: 59.32, y: 696.56 },
    { page: 1, n: 10000, x: 418.35, y: 670.69 },
    { page: 1, n: 10, x: 57.73, y: 628.81 },
    { page: 1, n: 15, x: 57.73, y: 518.57 },
    { page: 1, n: 1, x: 297.64, y: 43.71 },
  ];
  const at = (y: number): Line => ({ page: 1, label: "", text: "t", bounds: { x: 70.87, y, width: 453.54, height: 9.69 }, margin: [] });
  assert.equal(numberAt(at(611.65), numbers), 11);
  assert.equal(numberAt(at(571.01), numbers), 14);
  assert.equal(numberAt(at(501.41), numbers), 16);
});

test("numbered every fifth line in the right margin", () => {
  assert.equal(numberAt(lineAt(5, 13), every5(5, 5, 30, 320)), 13);
});

test("a number printed beside the line before the one counted", () => {
  const numbers = every5(5, 5, 30);
  const printed = { ...lineAt(5, 13), margin: [{ text: "40", x: 80 }] };
  assert.equal(location(sel([printed], numbers)), "p5, l40");
});

test("one line, many lines", () => {
  assert.equal(location(sel(numbered(5, 136, ["a"]))), "p5, l136");
  assert.equal(location(sel(numbered(5, 136, ["a", "b", "c"]))), "p5, l136–138");
});

test("two columns with numbers in both margins", () => {
  const lines = [0, 1, 2].map((i) => line(5, "x", [[String(20 + i), 80], [String(70 + i), 320]]));
  assert.equal(location(sel(lines)), "p5, l20–22");
});

test("across pages, numbering that goes on", () => {
  const lines = [...numbered(5, 227, ["a", "b", "c"]), ...numbered(6, 230, ["d", "e"])];
  assert.equal(location(sel(lines)), "p5–6, l227–231");
});

test("across pages, numbering that starts again", () => {
  const lines = [...numbered(5, 38, ["a", "b", "c"]), ...numbered(6, 1, ["d", "e", "f", "g"])];
  assert.equal(location(sel(lines)), "p5, l38 – p6, l4");
});

test("inconsistent numbers: the page only", () => {
  assert.equal(location(sel([line(5, "a", [["136", 80]]), line(5, "b", [["130", 80]])])), "p5");
  assert.equal(location(sel([line(5, "a", [["10", 80]]), line(5, "b", [["90", 80]])])), "p5");
  assert.equal(location(sel([line(5, "a", [["10", 80]]), line(5, "b")])), "p5");
  assert.equal(location(sel([line(5, "a", [["40", 80]]), line(6, "b", [["90", 80]])])), "p5–6");
});

test("a line or two without a number is fine", () => {
  const lines = [line(5, "a", [["10", 80]]), line(5, "b"), line(5, "c", [["12", 80]])];
  assert.equal(location(sel(lines)), "p5, l10–12");
});

test("the page label before the page number", () => {
  assert.equal(location(sel([line(3, "a", [], "iii")])), "p. iii");
  assert.equal(location(sel([line(3, "a", [], "iii"), line(4, "b", [], "iv")])), "p. iii–iv");
  assert.equal(location(sel([line(3, "a", [], ""), line(4, "b", [], "4")])), "p3–4");
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
  assert.equal(quoteText({ text: " ﬁx   it ", lines: [], numbers: [] }), "fix it");
});

test("the quote", () => {
  const lines = numbered(5, 136, ["the ice re-", "cently lost"]);
  assert.equal(quote(sel(lines)), 'p5, l136–137: "the ice recently lost"');
  assert.equal(quote(sel([line(5, "a")])), 'p5: "a"');
  assert.equal(quote(sel(lines), false), "p5, l136–137: the ice recently lost");
});

test("the location a line of a review starts with, up to its colon", () => {
  assert.equal(leadPrefix('p5, l136–138: "the ice"'), "p5, l136–138:".length);
  assert.equal(leadPrefix('p5–6: "the ice"'), "p5–6:".length);
  assert.equal(leadPrefix("l153: the ice"), "l153:".length);
  assert.equal(leadPrefix('p. iii: "the ice"'), "p. iii:".length);
  assert.equal(leadPrefix('p5, l38 – p6, l4: "the ice"'), "p5, l38 – p6, l4:".length);
  assert.equal(leadPrefix("P5, L3: x"), "P5, L3:".length);
});

test("a label a line starts with, up to its colon: a word, with a number after it if any", () => {
  assert.equal(leadPrefix("title: Ice sheets"), "title:".length);
  assert.equal(leadPrefix("Abstract: the second sentence"), "Abstract:".length);
  assert.equal(leadPrefix("Figure 3: the colours"), "Figure 3:".length);
  assert.equal(leadPrefix("Table S1: units"), "Table S1:".length);
  assert.equal(leadPrefix("Section 2.1: the method"), "Section 2.1:".length);
  assert.equal(leadPrefix("Méthodes: x"), "Méthodes:".length);
});

test("other lines have no lead", () => {
  assert.equal(leadPrefix("p5 is fine"), 0);
  assert.equal(leadPrefix("p5:x"), 0);
  assert.equal(leadPrefix("title:x"), 0);
  assert.equal(leadPrefix("The main point: x"), 0);
  assert.equal(leadPrefix("see p5: x"), 0);
  assert.equal(leadPrefix(' p5: "x"'), 0);
  assert.equal(leadPrefix(" title: x"), 0);
  assert.equal(leadPrefix("**p. 5, l. 136:** x"), 0);
  assert.equal(leadPrefix("**Title:** x"), 0);
  assert.equal(leadPrefix("https://example.org x"), 0);
});
