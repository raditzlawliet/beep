/**
 * Embedded highlighter for .http files.
 *
 * Finds regions inside the document that hold another language and colors
 * them with the real Lezer parser for that language:
 *   - request body: JSON / XML / HTML
 *   - `{% ... %}` pre/post script blocks: JavaScript
 *
 * Body kind follows core `effective_body_kind`: `// @body` directive >
 * `Content-Type` header > sniff (`{`/`[` json, `<` xml). Only for coloring;
 * sniffing never changes what is sent.
 *
 * `{{variable}}` spans are cut out of the embedded marks so the http
 * tokenizer's variable color stays visible inside JSON strings.
 */

import { ViewPlugin, Decoration, EditorView } from "@codemirror/view";
import type { DecorationSet, ViewUpdate } from "@codemirror/view";
import type { Range } from "@codemirror/state";
import type { Parser } from "@lezer/common";
import { highlightTree } from "@lezer/highlight";
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { html } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { monokaiHighlight } from "./monokai";

type EmbeddedLang = "json" | "xml" | "html" | "javascript";

export interface EmbeddedRegion {
  from: number;
  to: number;
  lang: EmbeddedLang;
}

let _parsers: Record<EmbeddedLang, Parser> | null = null;

function parserFor(lang: EmbeddedLang): Parser {
  _parsers ??= {
    json: json().language.parser,
    xml: xml().language.parser,
    html: html().language.parser,
    javascript: javascript().language.parser,
  };
  return _parsers[lang];
}

// ---------------------------------------------------------------------------
// Region scanner (line based, mirrors core parse_request_block sections)
// ---------------------------------------------------------------------------

function kindFromDirective(kind: string): EmbeddedLang | null {
  if (kind === "raw/json") return "json";
  if (kind === "raw/xml") return "xml";
  if (kind === "raw/html") return "html";
  return null;
}

function kindFromContentType(ct: string): EmbeddedLang | null {
  if (ct.includes("json")) return "json";
  if (ct.includes("html")) return "html";
  if (ct.includes("xml")) return "xml";
  return null;
}

function sniff(body: string): EmbeddedLang | null {
  const t = body.trimStart();
  if (t.startsWith("{") || t.startsWith("[")) return "json";
  if (t.startsWith("<")) return "xml";
  return null;
}

/** Find embedded-language regions in `.http` text (LF or CRLF). */
export function scanEmbeddedRegions(doc: string): EmbeddedRegion[] {
  const regions: EmbeddedRegion[] = [];

  let section: "pre" | "headers" | "body" = "pre";
  let directive: string | null = null;
  let contentType: string | null = null;
  let bodyFrom = -1;
  let bodyTo = -1;
  let scriptFrom = -1;

  const closeBody = () => {
    if (bodyFrom >= 0 && bodyTo > bodyFrom) {
      const text = doc.slice(bodyFrom, bodyTo);
      let lang: EmbeddedLang | null;
      if (directive !== null) lang = kindFromDirective(directive);
      else if (contentType !== null) lang = kindFromContentType(contentType);
      else lang = sniff(text);
      if (lang && text.trim()) regions.push({ from: bodyFrom, to: bodyTo, lang });
    }
    bodyFrom = -1;
    bodyTo = -1;
  };

  let pos = 0;
  while (pos <= doc.length) {
    const nl = doc.indexOf("\n", pos);
    const lineEnd = nl === -1 ? doc.length : nl;
    const contentEnd = lineEnd > pos && doc[lineEnd - 1] === "\r" ? lineEnd - 1 : lineEnd;
    const line = doc.slice(pos, contentEnd);
    const trimmed = line.trim();
    const indent = line.length - line.trimStart().length;
    const next = nl === -1 ? doc.length + 1 : nl + 1;

    // Inside a script block: look for the closing `%}`.
    if (scriptFrom >= 0) {
      const close = line.indexOf("%}");
      if (close >= 0) {
        regions.push({ from: scriptFrom, to: pos + close, lang: "javascript" });
        scriptFrom = -1;
      }
      pos = next;
      continue;
    }

    if (trimmed.startsWith("###")) {
      closeBody();
      section = "pre";
      directive = null;
      contentType = null;
      pos = next;
      continue;
    }

    // Script start: `< {%` before the request line, `> {%` after it.
    const scriptStart =
      section === "pre" ? /^<[ \t]*\{%/.exec(trimmed) : /^>[ \t]*\{%/.exec(trimmed);
    if (scriptStart) {
      if (section === "body") closeBody();
      const codeFrom = pos + indent + scriptStart[0].length;
      const close = doc.slice(codeFrom, contentEnd).indexOf("%}");
      if (close >= 0) regions.push({ from: codeFrom, to: codeFrom + close, lang: "javascript" });
      else scriptFrom = codeFrom;
      pos = next;
      continue;
    }

    if (section === "pre") {
      if (trimmed && !trimmed.startsWith("//") && !trimmed.startsWith("@") && !trimmed.startsWith("<")) {
        section = "headers";
      }
    } else if (section === "headers") {
      if (!trimmed) {
        section = "body";
      } else {
        const dir = /^\/\/[ \t]*@body[ \t]+(\S+)/.exec(trimmed);
        if (dir) directive = dir[1].toLowerCase();
        const ct = /^content-type[ \t]*:[ \t]*(.+)$/i.exec(trimmed);
        if (ct) contentType = ct[1].toLowerCase();
        // A disabled Content-Type (`//`) is ignored, same as core.
      }
    } else {
      // Body: post-script file ref / redirect ends it.
      if (trimmed.startsWith(">>") || /^>[ \t]+\S/.test(trimmed)) {
        closeBody();
      } else if (bodyFrom < 0 && !trimmed) {
        // Leading blank lines before the body text.
      } else {
        if (bodyFrom < 0) bodyFrom = pos;
        if (trimmed) bodyTo = contentEnd;
      }
    }

    pos = next;
  }

  if (scriptFrom >= 0) regions.push({ from: scriptFrom, to: doc.length, lang: "javascript" });
  closeBody();
  return regions;
}

// ---------------------------------------------------------------------------
// Decorations
// ---------------------------------------------------------------------------

const VAR_RE = /\{\{[\s\S]*?\}\}/g;

export interface EmbeddedMark {
  from: number;
  to: number;
  classes: string;
}

/** Push `[from, to)` minus any `{{...}}` spans in `holes` (sorted). */
function pushWithoutHoles(
  out: EmbeddedMark[],
  from: number,
  to: number,
  classes: string,
  holes: Array<[number, number]>,
) {
  let start = from;
  for (const [hf, ht] of holes) {
    if (ht <= start) continue;
    if (hf >= to) break;
    if (hf > start) out.push({ from: start, to: hf, classes });
    start = Math.max(start, ht);
  }
  if (start < to) out.push({ from: start, to, classes });
}

/** Highlight marks (Monokai classes) for every embedded region in `doc`. */
export function collectEmbeddedMarks(doc: string): EmbeddedMark[] {
  const out: EmbeddedMark[] = [];
  for (const r of scanEmbeddedRegions(doc)) {
    const text = doc.slice(r.from, r.to);
    const holes: Array<[number, number]> = [];
    for (const m of text.matchAll(VAR_RE)) {
      holes.push([r.from + m.index, r.from + m.index + m[0].length]);
    }
    try {
      const tree = parserFor(r.lang).parse(text);
      highlightTree(tree, monokaiHighlight, (from, to, classes) => {
        pushWithoutHoles(out, r.from + from, r.from + to, classes, holes);
      });
    } catch {
      // Unparsable region: leave it plain.
    }
  }
  return out;
}

function buildDecorations(view: EditorView): DecorationSet {
  const marks = collectEmbeddedMarks(view.state.doc.toString());
  if (!marks.length) return Decoration.none;
  const ranges: Range<Decoration>[] = marks.map((m) =>
    Decoration.mark({ class: m.classes }).range(m.from, m.to),
  );
  return Decoration.set(ranges, true);
}

export const embeddedHighlighter = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: ViewUpdate) {
      if (update.docChanged) this.decorations = buildDecorations(update.view);
    }
  },
  { decorations: (v) => v.decorations },
);
