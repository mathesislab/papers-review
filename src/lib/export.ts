// The review, or a round of it, for export: as plain text for a journal's web form (the Markdown marks dropped, the
// symbols and the locations kept), and as HTML for the page of the PDF. Both are made from the same parse of the
// Markdown (the editor's own parser), so they read the same.

import { markdownLanguage } from "@codemirror/lang-markdown";
import { leadPrefix } from "./quote.ts";

type Node = ReturnType<typeof markdownLanguage.parser.parse>["topNode"];

type Inline =
  | { t: "text" | "code"; text: string }
  | { t: "em" | "strong" | "del"; children: Inline[] }
  | { t: "link"; children: Inline[]; url: string }
  | { t: "br" };

type Block =
  | { t: "heading"; level: number; children: Inline[] }
  | { t: "p"; children: Inline[] }
  | { t: "quote"; blocks: Block[] }
  | { t: "list"; ordered: boolean; start: number; items: Block[][] }
  | { t: "code" | "raw"; text: string }
  | { t: "hr" };

/** What a PDF is headed with when the project sets no heading. */
export const DEFAULT_HEADING = "Manuscript review";

/** The text of a section (one of `sections` of the text, from the editor), or all of it (none). */
export function section(text: string, found?: { from: number; to: number }): string {
  return found ? text.slice(found.from, found.to) : text;
}

// The parse

type Context = { src: string; refs: Map<string, string> };

const children = (node: Node) => {
  const out: Node[] = [];
  for (let c = node.firstChild; c; c = c.nextSibling) out.push(c);
  return out;
};

/** The marks of the Markdown that carry no text. */
const MARKS = new Set(["HeaderMark", "EmphasisMark", "StrikethroughMark", "QuoteMark", "TaskMarker"]);

const ENTITIES: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };

/** An entity (`&amp;`, `&#176;`) as its character; as it was written if not known. */
function entity(source: string): string {
  const name = source.slice(1, -1);
  const code = name.startsWith("#x") ? parseInt(name.slice(2), 16) : name.startsWith("#") ? parseInt(name.slice(1), 10) : NaN;
  if (Number.isInteger(code) && code > 0 && code <= 0x10ffff) return String.fromCodePoint(code);
  return ENTITIES[name] ?? source;
}

const text = (text: string): Inline => ({ t: "text", text });

/** A reference to a link (`[text][ref]`, `[ref]`) as its label is written in the definition. */
const label = (name: string) => name.replace(/^\[|\]$/g, "").trim().replace(/\s+/g, " ").toLowerCase();

/** The inline content of `node`: the text between its children and what its children are. */
function inlines(node: Node, ctx: Context, from = node.from, to = node.to, among = children(node)): Inline[] {
  const out: Inline[] = [];
  const push = (...items: Inline[]) => {
    for (const item of items) {
      const last = out[out.length - 1];
      if (item.t === "text" && last?.t === "text") last.text += item.text;
      else out.push(item);
    }
  };
  let at = from;
  for (const c of among) {
    if (c.from < at || c.to > to) continue;
    if (c.from > at) push(text(ctx.src.slice(at, c.from)));
    at = c.to;
    const source = ctx.src.slice(c.from, c.to);
    if (MARKS.has(c.name)) continue;
    switch (c.name) {
      case "Emphasis":
        push({ t: "em", children: inlines(c, ctx) });
        break;
      case "StrongEmphasis":
        push({ t: "strong", children: inlines(c, ctx) });
        break;
      case "Strikethrough":
        push({ t: "del", children: inlines(c, ctx) });
        break;
      case "InlineCode": {
        const marks = children(c).filter((m) => m.name === "CodeMark");
        const code = ctx.src.slice(marks[0]?.to ?? c.from, marks[marks.length - 1]?.from ?? c.to).replace(/\n/g, " ");
        push({ t: "code", text: /^ .*\S.* $/.test(code) ? code.slice(1, -1) : code });
        break;
      }
      case "Link":
      case "Image": {
        const parts = children(c);
        const marks = parts.filter((m) => m.name === "LinkMark");
        const inner = marks.length >= 2 ? inlines(c, ctx, marks[0].to, marks[1].from, parts) : [];
        if (c.name === "Image") {
          push(...inner);
          break;
        }
        const url = parts.find((m) => m.name === "URL");
        const ref = parts.find((m) => m.name === "LinkLabel");
        const key = ref ? label(ctx.src.slice(ref.from, ref.to)) || label(plain(inner)) : label(plain(inner));
        const href = url ? ctx.src.slice(url.from, url.to) : ctx.refs.get(key);
        if (href) push({ t: "link", children: inner, url: href });
        else push(...inner);
        break;
      }
      case "Autolink":
      case "URL": {
        const url = source.replace(/^<|>$/g, "");
        push({ t: "link", children: [text(url)], url: /^www\./i.test(url) ? `https://${url}` : url });
        break;
      }
      case "Escape":
        push(text(source.slice(1)));
        break;
      case "Entity":
        push(text(entity(source)));
        break;
      case "HardBreak":
        push({ t: "br" });
        break;
      default:
        push(text(source));
    }
  }
  if (at < to) push(text(ctx.src.slice(at, to)));
  // the lines of a paragraph carry no indent, and no space ends one
  for (const item of out) if (item.t === "text") item.text = item.text.replace(/[ \t]*\n[ \t]*/g, "\n");
  return out;
}

/** The inlines without the space around them. */
function tidy(list: Inline[]): Inline[] {
  const first = list[0];
  const last = list[list.length - 1];
  if (first?.t === "text") first.text = first.text.trimStart();
  if (last?.t === "text") last.text = last.text.trimEnd();
  return list.filter((i) => i.t !== "text" || i.text);
}

function blocks(node: Node, ctx: Context): Block[] {
  const out: Block[] = [];
  for (const c of children(node)) {
    const heading = /^(?:ATX|Setext)Heading(\d)$/.exec(c.name);
    if (heading) {
      out.push({ t: "heading", level: Number(heading[1]), children: tidy(inlines(c, ctx)) });
      continue;
    }
    switch (c.name) {
      case "Paragraph":
        out.push({ t: "p", children: tidy(inlines(c, ctx)) });
        break;
      case "Task": {
        // a list item with a box: kept as the text of it
        const box = children(c).find((m) => m.name === "TaskMarker");
        const marker = box ? ctx.src.slice(box.from, box.to) : "";
        out.push({ t: "p", children: tidy([text(`${marker} `), ...tidy(inlines(c, ctx))]) });
        break;
      }
      case "Blockquote":
        out.push({ t: "quote", blocks: blocks(c, ctx) });
        break;
      case "BulletList":
      case "OrderedList": {
        const items = children(c).filter((m) => m.name === "ListItem");
        const mark = children(items[0] ?? c).find((m) => m.name === "ListMark");
        const start = parseInt(mark ? ctx.src.slice(mark.from, mark.to) : "1", 10);
        out.push({ t: "list", ordered: c.name === "OrderedList", start: Number.isNaN(start) ? 1 : start, items: items.map((i) => blocks(i, ctx)) });
        break;
      }
      case "FencedCode":
      case "CodeBlock": {
        const code = children(c)
          .filter((m) => m.name === "CodeText")
          .map((m) => ctx.src.slice(m.from, m.to))
          .join("");
        out.push({ t: "code", text: code.replace(/\n$/, "") });
        break;
      }
      case "HorizontalRule":
        out.push({ t: "hr" });
        break;
      // marks, comments and the definitions of links are not text
      case "QuoteMark":
      case "ListMark":
      case "CommentBlock":
      case "LinkReference":
        break;
      // what the review is not expected to hold (a table, a block of HTML) stays as it was written
      default:
        out.push({ t: "raw", text: ctx.src.slice(c.from, c.to) });
    }
  }
  return out;
}

function parse(markdown: string): Block[] {
  const tree = markdownLanguage.parser.parse(markdown);
  const ctx: Context = { src: markdown, refs: new Map() };
  for (const c of children(tree.topNode)) {
    if (c.name !== "LinkReference") continue;
    const name = children(c).find((m) => m.name === "LinkLabel");
    const url = children(c).find((m) => m.name === "URL");
    if (name && url) ctx.refs.set(label(markdown.slice(name.from, name.to)), markdown.slice(url.from, url.to));
  }
  return blocks(tree.topNode, ctx);
}

// Plain text

/** The text of inlines with no marks. */
function plain(list: Inline[]): string {
  return list
    .map((i) => {
      switch (i.t) {
        case "text":
        case "code":
          return i.text;
        case "br":
          return "\n";
        case "link": {
          const label = plain(i.children);
          return !label || label === i.url ? i.url : `${label} (${i.url})`;
        }
        default:
          return plain(i.children);
      }
    })
    .join("");
}

function plainBlocks(list: Block[]): string {
  return list.map(plainBlock).join("\n\n");
}

function plainBlock(block: Block): string {
  switch (block.t) {
    case "heading":
    case "p":
      return plain(block.children);
    case "quote":
      return plainBlocks(block.blocks)
        .split("\n")
        .map((line) => (line ? `> ${line}` : ">"))
        .join("\n");
    case "list":
      return block.items
        .map((item, i) => {
          const mark = block.ordered ? `${block.start + i}. ` : "- ";
          // a list within the item follows it at once, any other block after a blank line
          const body = item.reduce((all, b, k) => (k === 0 ? plainBlock(b) : `${all}${b.t === "list" ? "\n" : "\n\n"}${plainBlock(b)}`), "");
          const [first, ...rest] = body.split("\n");
          return [mark + first, ...rest.map((line) => (line ? " ".repeat(mark.length) + line : line))].join("\n");
        })
        .join("\n");
    case "code":
    case "raw":
      return block.text;
    case "hr":
      return "---";
  }
}

/** The Markdown as plain text for a form: the marks dropped (`#`, `*`, `_`, backticks), each heading a line of its own
 *  with a blank line around it, list items kept as "- " (or their number), a link as "text (url)". */
export function toPlainText(markdown: string): string {
  return plainBlocks(parse(markdown));
}

// HTML

const escape = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** Whether a link may be one in the page: a web or mail address, or a relative one. */
const safe = (url: string) => (/^[a-z][a-z0-9+.-]*:/i.test(url) ? /^(?:https?|mailto):/i.test(url) : true);

/** Text with the lead a line of a review starts with (`p5, l136:`, `Figure 3:`) in a span of class "lead"; `start`: the text
 *  starts a line. */
function htmlText(text: string, start: boolean): string {
  return text
    .split("\n")
    .map((line, i) => {
      const length = i > 0 || start ? leadPrefix(line) : 0;
      return length ? `<span class="lead">${escape(line.slice(0, length))}</span>${escape(line.slice(length))}` : escape(line);
    })
    .join("\n");
}

function html(list: Inline[], start = false): string {
  let out = "";
  // whether the next text starts a line of the review
  let line = start;
  for (const i of list) {
    switch (i.t) {
      case "text":
        out += htmlText(i.text, line);
        line = i.text.endsWith("\n");
        continue;
      case "code":
        out += `<code>${escape(i.text)}</code>`;
        break;
      case "br":
        out += "<br>";
        line = true;
        continue;
      case "link":
        out += safe(i.url) ? `<a href="${escape(i.url)}">${html(i.children)}</a>` : `${html(i.children)} (${escape(i.url)})`;
        break;
      default:
        out += `<${i.t}>${html(i.children)}</${i.t}>`;
    }
    line = false;
  }
  return out;
}

function htmlBlocks(list: Block[]): string {
  return list.map(htmlBlock).join("\n");
}

function htmlBlock(block: Block): string {
  switch (block.t) {
    case "heading":
      return `<h${block.level}>${html(block.children)}</h${block.level}>`;
    case "p":
      return `<p>${html(block.children, true)}</p>`;
    case "quote":
      return `<blockquote>\n${htmlBlocks(block.blocks)}\n</blockquote>`;
    case "list": {
      const tag = block.ordered ? "ol" : "ul";
      const start = block.ordered && block.start !== 1 ? ` start="${block.start}"` : "";
      // the first paragraph of an item is its text, without a paragraph of its own
      const items = block.items.map(([first, ...rest]) => {
        const lead = first?.t === "p" ? html(first.children, true) : first ? `\n${htmlBlock(first)}` : "";
        return `<li>${lead}${rest.length ? `\n${htmlBlocks(rest)}\n` : ""}</li>`;
      });
      return `<${tag}${start}>\n${items.join("\n")}\n</${tag}>`;
    }
    case "code":
      return `<pre><code>${escape(block.text)}</code></pre>`;
    case "raw":
      return `<pre class="raw">${escape(block.text)}</pre>`;
    case "hr":
      return "<hr>";
  }
}

/** The Markdown as HTML (a fragment): headings, paragraphs, emphasis, code, quotes, lists, links, breaks and rules;
 *  all text escaped. The lead a line starts with (a location, a label) is in a span of class "lead". */
export function toHtml(markdown: string): string {
  return htmlBlocks(parse(markdown));
}

// Names and the page

/** The stem of the file an export is saved as: the review file's name and the section's title, as in
 *  "review-round-2" (`reviewFile` may be a path; no section: the review alone). */
export function exportName(reviewFile: string, title: string | null): string {
  const file = reviewFile.slice(reviewFile.lastIndexOf("/") + 1).replace(/\.[^.]*$/, "");
  const stem = file.replace(/[^\p{L}\p{N}._ -]+/gu, "-").replace(/^[.\- ]+|[. ]+$/g, "") || "review";
  const slug = (title ?? "")
    .normalize("NFKD")
    .replace(/\p{M}+/gu, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40)
    .replace(/-+$/, "");
  return slug ? `${stem}-${slug}` : stem;
}

/** What an export is headed and signed with: the heading, the details below it (journal, manuscript, date, as far
 *  as there are any), and the reviewer's name at the end, if signed. */
export type Frame = {
  heading: string;
  details: (string | null | undefined)[];
  signature: string | null;
};

/** The parts of a frame as they are written: the heading (the default without one), the details joined by " · ",
 *  the name. */
function framed({ heading, details, signature }: Frame) {
  return {
    title: heading.trim() || DEFAULT_HEADING,
    line: details.map((d) => d?.trim()).filter(Boolean).join(" · "),
    name: signature?.trim() ?? "",
  };
}

/** The page of the PDF: the header, the body (HTML, from `toHtml`), the name at the end. */
export function pdfDocument({ body, ...frame }: Frame & { body: string }): string {
  const { title, line, name } = framed(frame);
  return [
    '<article class="page">',
    "<header>",
    `<h1 class="title">${escape(title)}</h1>`,
    ...(line ? [`<p class="details">${escape(line)}</p>`] : []),
    "</header>",
    '<section class="review">',
    body,
    "</section>",
    ...(name ? ['<footer>', `<p class="signature">${escape(name)}</p>`, "</footer>"] : []),
    "</article>",
  ].join("\n");
}

/** The text for a journal's form, headed and signed as the PDF: the heading, the details, the body (from
 *  `toPlainText`), the name. */
export function textDocument({ body, ...frame }: Frame & { body: string }): string {
  const { title, line, name } = framed(frame);
  return [[title, line].filter(Boolean).join("\n"), body.trim(), name].filter(Boolean).join("\n\n") + "\n";
}
