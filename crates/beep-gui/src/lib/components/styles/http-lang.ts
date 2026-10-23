/**
 * CodeMirror language for Beep's .http / .rest files (Monokai colors).
 *
 * Two layers:
 *   1. StreamLanguage tokenizer (this file) - request structure: `###`, comments,
 *      `@var`, request line, headers, multiline query, `{{variables}}`, script
 *      markers, redirects, multipart boundaries.
 *   2. Embedded highlighter (`body-highlighter.ts`) - real Lezer parsers for the
 *      body (JSON/XML/HTML) and `{% ... %}` script blocks (JavaScript).
 *
 * The tokenizer leaves body text and script code untokenized (null) so the
 * embedded highlighter owns their colors.
 *
 * Usage: `extensions.push(httpLanguage())`.
 */

import {
  StreamLanguage,
  HighlightStyle,
  syntaxHighlighting,
} from "@codemirror/language";
import type { StringStream } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { tags, Tag } from "@lezer/highlight";
import { embeddedHighlighter } from "./body-highlighter";

// ---------------------------------------------------------------------------
// Custom tags (no built-in lezer equivalent)
// ---------------------------------------------------------------------------

export const httpMethodGET = Tag.define();
export const httpMethodPOST = Tag.define();
export const httpMethodPUT = Tag.define();
export const httpMethodDELETE = Tag.define();
export const httpMethodPATCH = Tag.define();
export const httpMethodHEAD = Tag.define();
export const httpMethodOPTIONS = Tag.define();
export const httpMethodOther = Tag.define();
export const httpUrlQueryTag = Tag.define();

// ---------------------------------------------------------------------------
// Token names
// ---------------------------------------------------------------------------

const S = {
  separator: "httpSeparator",
  comment: "httpComment",
  disabled: "httpDisabled",
  methodGET: "httpMethodGET",
  methodPOST: "httpMethodPOST",
  methodPUT: "httpMethodPUT",
  methodDELETE: "httpMethodDELETE",
  methodPATCH: "httpMethodPATCH",
  methodHEAD: "httpMethodHEAD",
  methodOPTIONS: "httpMethodOPTIONS",
  methodOther: "httpMethodOther",
  url: "httpUrl",
  urlQuery: "httpUrlQuery",
  httpVer: "httpVersion",
  headerKey: "httpHeaderKey",
  headerColon: "httpHeaderColon",
  headerVal: "httpHeaderValue",
  varKey: "httpVarKey",
  varEq: "httpVarEq",
  varVal: "httpVarVal",
  variable: "httpVariable",
  scriptTag: "httpScriptTag",
  redirect: "httpRedirect",
  fileRef: "httpFileRef",
  boundary: "httpBoundary",
} as const;

const METHOD_STYLE: Record<string, string> = {
  GET: S.methodGET,
  POST: S.methodPOST,
  PUT: S.methodPUT,
  DELETE: S.methodDELETE,
  PATCH: S.methodPATCH,
  HEAD: S.methodHEAD,
  OPTIONS: S.methodOPTIONS,
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/** Section of the current request block. Mirrors `parse_request_block` in core. */
type Section = "pre" | "headers" | "body";

/** What the current line is, decided at line start. */
type LineKind = "none" | "request" | "header" | "query" | "var" | "body" | "path";

interface State {
  section: Section;
  line: LineKind;
  /** Inside `{% ... %}` script code. */
  inScript: boolean;
  /** Request line: method seen. */
  seenMethod: boolean;
  /** Request line: `?` seen, rest of the URL is query. */
  inQuery: boolean;
  /** Header line: `:` seen. */
  seenColon: boolean;
  /** `@var` line: `=` seen. */
  seenEq: boolean;
}

function startState(): State {
  return {
    section: "pre",
    line: "none",
    inScript: false,
    seenMethod: false,
    inQuery: false,
    seenColon: false,
    seenEq: false,
  };
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

/** `{{name}}` / `{{$dynamic}}`. Returns true when consumed. */
function eatVariable(stream: StringStream): boolean {
  if (!stream.match("{{")) return false;
  while (!stream.eol() && !stream.match("}}")) stream.next();
  return true;
}

/** Script code up to `%}` (left for the embedded JS highlighter). */
function scriptToken(stream: StringStream, state: State): string | null {
  if (stream.match("%}")) {
    state.inScript = false;
    return S.scriptTag;
  }
  if (!stream.match(/^.*?(?=%\})/)) stream.skipToEnd();
  return null;
}

/** Line-start dispatch. Returns a token when the whole line is handled. */
function lineStart(stream: StringStream, state: State): string | null | undefined {
  state.line = "none";
  state.seenMethod = false;
  state.inQuery = false;
  state.seenColon = false;
  state.seenEq = false;

  if (stream.match("###")) {
    state.section = "pre";
    stream.skipToEnd();
    return S.separator;
  }
  if (stream.match("//-")) {
    stream.skipToEnd();
    return S.disabled;
  }
  if (stream.match("//")) {
    stream.skipToEnd();
    return S.comment;
  }

  // Post-request script / redirect: any section after the request line.
  if (state.section !== "pre") {
    if (stream.match(/^>>!?/)) {
      state.line = "path";
      return S.redirect;
    }
    if (stream.match(/^>[ \t]*\{%/)) {
      state.inScript = true;
      return S.scriptTag;
    }
    if (stream.match(/^>[ \t]+(?=\S)/)) {
      stream.skipToEnd();
      return S.fileRef;
    }
  }

  if (state.section === "pre") {
    if (stream.match(/^<[ \t]*\{%/)) {
      state.inScript = true;
      return S.scriptTag;
    }
    if (stream.match(/^<[ \t]*(?=\S)/)) {
      stream.skipToEnd();
      return S.fileRef;
    }
    if (stream.peek() === "@") {
      state.line = "var";
      return undefined;
    }
    // Anything else is the request line.
    state.section = "headers";
    state.line = "request";
    return undefined;
  }

  if (state.section === "headers") {
    state.line = stream.peek() === "?" || stream.peek() === "&" ? "query" : "header";
    return undefined;
  }

  // Body
  if (stream.match(/^<[ \t]+(?=\S)/)) {
    stream.skipToEnd();
    return S.fileRef;
  }
  if (stream.match(/^--\S+/)) {
    stream.skipToEnd();
    return S.boundary;
  }
  state.line = "body";
  return undefined;
}

function token(stream: StringStream, state: State): string | null {
  if (state.inScript) return scriptToken(stream, state);

  if (stream.sol()) {
    stream.eatSpace();
    if (stream.eol()) return null;
    const handled = lineStart(stream, state);
    if (handled !== undefined) return handled;
  }

  if (stream.eatSpace()) return null;

  switch (state.line) {
    case "request":
      return requestLineToken(stream, state);
    case "header":
      return headerToken(stream, state);
    case "query":
      return queryToken(stream);
    case "var":
      return varToken(stream, state);
    case "path":
      stream.skipToEnd();
      return S.fileRef;
    default:
      // Body text: only variables are tokens; the rest belongs to the embedded highlighter.
      if (eatVariable(stream)) return S.variable;
      if (!stream.match(/^[^{]+/)) stream.next();
      return null;
  }
}

function requestLineToken(stream: StringStream, state: State): string | null {
  if (!state.seenMethod) {
    state.seenMethod = true;
    const word = stream.match(/^[A-Za-z]+(?=\s)/) as RegExpMatchArray | null;
    if (word) return METHOD_STYLE[word[0].toUpperCase()] ?? S.methodOther;
  }
  if (eatVariable(stream)) return S.variable;
  if (stream.match(/^HTTP\/\d+(\.\d+)?(?=\s|$)/i)) return S.httpVer;
  if (stream.peek() === "?") state.inQuery = true;
  if (state.inQuery) {
    if (!stream.match(/^[^\s{]+/)) stream.next();
    return S.urlQuery;
  }
  if (!stream.match(/^[^\s?{]+/)) stream.next();
  return S.url;
}

function headerToken(stream: StringStream, state: State): string | null {
  if (!state.seenColon) {
    if (stream.match(":")) {
      state.seenColon = true;
      return S.headerColon;
    }
    if (eatVariable(stream)) return S.variable;
    if (!stream.match(/^[^:{]+/)) stream.next();
    return S.headerKey;
  }
  if (eatVariable(stream)) return S.variable;
  if (!stream.match(/^[^{]+/)) stream.next();
  return S.headerVal;
}

function queryToken(stream: StringStream): string | null {
  if (eatVariable(stream)) return S.variable;
  if (!stream.match(/^[^{\s]+/)) stream.next();
  return S.urlQuery;
}

function varToken(stream: StringStream, state: State): string | null {
  if (!state.seenEq) {
    if (stream.match("=")) {
      state.seenEq = true;
      return S.varEq;
    }
    if (!stream.match(/^[^=\s]+/)) stream.next();
    return S.varKey;
  }
  // Trailing ` // comment` (space before `//`, so `https://` stays a value).
  if (stream.match(/^\/\/.*/)) return S.comment;
  if (eatVariable(stream)) return S.variable;
  if (!stream.match(/^.+?(?=\s\/\/|\{\{|$)/)) stream.next();
  return S.varVal;
}

const httpStreamLanguage = StreamLanguage.define<State>({
  name: "http",
  startState,
  copyState: (s) => ({ ...s }),
  token,
  blankLine(state) {
    // Blank line after the request line / headers starts the body.
    if (state.section === "headers" && !state.inScript) state.section = "body";
  },
  languageData: {
    commentTokens: { line: "//" },
  },
  tokenTable: {
    [S.separator]: tags.heading,
    [S.comment]: tags.comment,
    [S.disabled]: tags.contentSeparator,
    [S.methodGET]: httpMethodGET,
    [S.methodPOST]: httpMethodPOST,
    [S.methodPUT]: httpMethodPUT,
    [S.methodDELETE]: httpMethodDELETE,
    [S.methodPATCH]: httpMethodPATCH,
    [S.methodHEAD]: httpMethodHEAD,
    [S.methodOPTIONS]: httpMethodOPTIONS,
    [S.methodOther]: httpMethodOther,
    [S.url]: tags.url,
    [S.urlQuery]: httpUrlQueryTag,
    [S.httpVer]: tags.typeName,
    [S.headerKey]: tags.propertyName,
    [S.headerColon]: tags.punctuation,
    [S.headerVal]: tags.string,
    [S.varKey]: tags.definition(tags.variableName),
    [S.varEq]: tags.operator,
    [S.varVal]: tags.string,
    [S.variable]: tags.special(tags.variableName),
    [S.scriptTag]: tags.controlKeyword,
    [S.redirect]: tags.operatorKeyword,
    [S.fileRef]: tags.string,
    [S.boundary]: tags.contentSeparator,
  },
});

// ---------------------------------------------------------------------------
// Monokai additions for http-specific tags
// ---------------------------------------------------------------------------

export const httpHighlight = HighlightStyle.define([
  { tag: httpMethodGET, color: "#A6E22E", fontWeight: "bold" },
  { tag: httpMethodPOST, color: "#E6DB74", fontWeight: "bold" },
  { tag: httpMethodPUT, color: "#66D9EF", fontWeight: "bold" },
  { tag: httpMethodDELETE, color: "#F92672", fontWeight: "bold" },
  { tag: httpMethodPATCH, color: "#AE81FF", fontWeight: "bold" },
  { tag: httpMethodHEAD, color: "#88846C", fontWeight: "bold" },
  { tag: httpMethodOPTIONS, color: "#88846C", fontWeight: "bold" },
  { tag: httpMethodOther, color: "#F8F8F2", fontWeight: "bold" },
  { tag: httpUrlQueryTag, color: "#FD971F" },
  { tag: tags.heading, color: "#75715E", fontWeight: "bold" },
  { tag: tags.contentSeparator, color: "#75715E", fontStyle: "italic", opacity: "0.7" },
  { tag: tags.special(tags.variableName), color: "#AE81FF" },
  { tag: tags.punctuation, color: "#75715E" },
]);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Full .http support: tokenizer + http colors + embedded body/script highlighting. */
export function httpLanguage(): Extension {
  return [httpStreamLanguage, syntaxHighlighting(httpHighlight), embeddedHighlighter];
}
