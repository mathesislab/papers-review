// The review for export: plain text and HTML from the Markdown, the section, the file name, the page of the PDF
// (src/lib/export.ts).

import { strict as assert } from "node:assert";
import { test } from "node:test";
import { exportName, pdfDocument, section, textDocument, toHtml, toPlainText } from "../src/lib/export.ts";

test("plain text: the marks dropped, the headings lines of their own", () => {
  const md = "# Review\nIntro with *emphasis*, **strong**, _under_ and `code`.\n## Round 1\nText.";
  assert.equal(toPlainText(md), "Review\n\nIntro with emphasis, strong, under and code.\n\nRound 1\n\nText.");
});

test("plain text: the symbols, the locations and the escapes kept", () => {
  const md = 'p5, l136: "CO₂ rose by 3 ± 1 °C → more" \\* not a mark & more\n\nl153: see Fig. 2';
  assert.equal(toPlainText(md), 'p5, l136: "CO₂ rose by 3 ± 1 °C → more" * not a mark & more\n\nl153: see Fig. 2');
});

test("plain text: lists", () => {
  assert.equal(toPlainText("- a\n- b *x*\n  - c\n- d"), "- a\n- b x\n  - c\n- d");
  assert.equal(toPlainText("1. one\n2. two"), "1. one\n2. two");
  assert.equal(toPlainText("3. three\n4. four"), "3. three\n4. four");
  assert.equal(toPlainText("- [ ] open\n- [x] done"), "- [ ] open\n- [x] done");
  assert.equal(toPlainText("- a\n\n  more\n- b"), "- a\n\n  more\n- b");
});

test("plain text: links as text and address", () => {
  assert.equal(toPlainText("See [the data](https://x.org/d) and <https://y.org>."), "See the data (https://x.org/d) and https://y.org.");
  assert.equal(toPlainText("[https://x.org](https://x.org)"), "https://x.org");
  assert.equal(toPlainText("[ref text][r]\n\n[r]: https://r.org"), "ref text (https://r.org)");
});

test("plain text: quotes, code, rules, breaks", () => {
  assert.equal(toPlainText("> quoted\n> twice\n\nafter"), "> quoted\n> twice\n\nafter");
  assert.equal(toPlainText("```js\nlet a = *1*;\n```"), "let a = *1*;");
  assert.equal(toPlainText("    indented\n    code"), "indented\ncode");
  assert.equal(toPlainText("a\n\n---\n\nb"), "a\n\n---\n\nb");
  assert.equal(toPlainText("one  \ntwo\\\nthree"), "one\ntwo\nthree");
});

test("plain text: a table or a block of HTML as written, a comment dropped", () => {
  assert.equal(toPlainText("| a | b |\n|---|---|\n| 1 | 2 |"), "| a | b |\n|---|---|\n| 1 | 2 |");
  assert.equal(toPlainText("a\n\n<!-- note -->\n\nb"), "a\n\nb");
});

test("plain text: empty", () => {
  assert.equal(toPlainText(""), "");
  assert.equal(toPlainText("\n\n"), "");
});

test("html: headings, paragraphs and emphasis", () => {
  assert.equal(toHtml("## Round 1\n\nSome *em*, **strong**, ~~gone~~ and `code`."), "<h2>Round 1</h2>\n<p>Some <em>em</em>, <strong>strong</strong>, <del>gone</del> and <code>code</code>.</p>");
  assert.equal(toHtml("Setext\n======"), "<h1>Setext</h1>");
});

test("html: text escaped", () => {
  assert.equal(toHtml('a < b & c > "d" <b>x</b>'), "<p>a &lt; b &amp; c &gt; &quot;d&quot; &lt;b&gt;x&lt;/b&gt;</p>");
  assert.equal(toHtml("`<script>`"), "<p><code>&lt;script&gt;</code></p>");
  assert.equal(toHtml("```\n<div>&</div>\n```"), "<pre><code>&lt;div&gt;&amp;&lt;/div&gt;</code></pre>");
  assert.equal(toHtml("<div>block</div>"), '<pre class="raw">&lt;div&gt;block&lt;/div&gt;</pre>');
  assert.equal(toHtml("&amp; &copy; &#176;"), "<p>&amp; &amp;copy; °</p>");
});

test("html: lists", () => {
  assert.equal(toHtml("- a\n- b\n  - c"), "<ul>\n<li>a</li>\n<li>b\n<ul>\n<li>c</li>\n</ul>\n</li>\n</ul>");
  assert.equal(toHtml("1. one\n2. two"), "<ol>\n<li>one</li>\n<li>two</li>\n</ol>");
  assert.equal(toHtml("3. three"), '<ol start="3">\n<li>three</li>\n</ol>');
});

test("html: links, only to addresses that are safe", () => {
  assert.equal(toHtml("[a](https://x.org?a=1&b=2)"), '<p><a href="https://x.org?a=1&amp;b=2">a</a></p>');
  assert.equal(toHtml("[a](javascript:alert(1))"), "<p>a (javascript:alert(1))</p>");
  assert.equal(toHtml("<https://y.org>"), '<p><a href="https://y.org">https://y.org</a></p>');
});

test("html: quotes, rules, breaks", () => {
  assert.equal(toHtml("> q1\n> q2"), "<blockquote>\n<p>q1\nq2</p>\n</blockquote>");
  assert.equal(toHtml("a\n\n---\n\nb"), "<p>a</p>\n<hr>\n<p>b</p>");
  assert.equal(toHtml("one  \ntwo"), "<p>one<br>two</p>");
});

test("html: the location a line starts with in a span", () => {
  assert.equal(toHtml('p5, l136: "text"'), '<p><span class="loc">p5, l136:</span> &quot;text&quot;</p>');
  assert.equal(toHtml("first\np. iii: second"), '<p>first\n<span class="loc">p. iii:</span> second</p>');
  assert.equal(toHtml("see p5, l136: here"), "<p>see p5, l136: here</p>");
  assert.equal(toHtml("- l153: in a list"), '<ul>\n<li><span class="loc">l153:</span> in a list</li>\n</ul>');
});

test("section: the text of a round, or all", () => {
  const text = "# Review\n\nintro\n\n## Round 1\n\na\n\n## Round 2\n\nb\n";
  assert.equal(section(text, 1), "## Round 2\n\nb\n");
  assert.equal(section(text, 0), "## Round 1\n\na\n\n");
  assert.equal(section(text, null), text);
  assert.equal(section(text, 5), text);
  assert.equal(section("no sections\n", 0), "no sections\n");
});

test("names: the review file and the section", () => {
  assert.equal(exportName("review.md", "Round 2"), "review-round-2");
  assert.equal(exportName("/Users/me/paper/review.md", null), "review");
  assert.equal(exportName("/x/My Review.md", "Réponse: ¡Ronde 3!"), "My Review-reponse-ronde-3");
  assert.equal(exportName("/x/a:b*c.md", "Round/2"), "a-b-c-round-2");
  assert.equal(exportName("/x/.md", "???"), "review");
  assert.equal(exportName("review.md", "x".repeat(80)), `review-${"x".repeat(40)}`);
});

test("the page: header, body, signature", () => {
  const body = toHtml("Text.");
  const page = pdfDocument({ heading: "Review of <it>", details: ["Nature", null, "2026-11-14"], body, signature: null });
  assert.match(page, /<h1 class="title">Review of &lt;it&gt;<\/h1>/);
  assert.match(page, /<p class="details">Nature · 2026-11-14<\/p>/);
  assert.match(page, /<section class="review">\n<p>Text\.<\/p>\n<\/section>/);
  assert.doesNotMatch(page, /signature/);
});

test("the page: the default heading, no details line, a signature", () => {
  const page = pdfDocument({ heading: "  ", details: [], body: "", signature: " A. Reviewer " });
  assert.match(page, /<h1 class="title">Manuscript review<\/h1>/);
  assert.doesNotMatch(page, /details/);
  assert.match(page, /<p class="signature">A\. Reviewer<\/p>/);
});

test("the text headed and signed as the PDF", () => {
  const body = "## Round 1\n\nGood.";
  assert.equal(
    textDocument({ heading: "", details: ["The Cryosphere", null, "2026-11-14"], body, signature: " A. Reviewer " }),
    "Manuscript review\nThe Cryosphere · 2026-11-14\n\n## Round 1\n\nGood.\n\nA. Reviewer\n",
  );
  assert.equal(textDocument({ heading: "Report", details: [], body, signature: null }), "Report\n\n## Round 1\n\nGood.\n");
});
