export type EditorLanguage = "text" | "json" | "html" | "xml" | "http" | "javascript";

/** Language from a known file extension, or null when unknown / no extension. */
export function languageFromName(name: string): EditorLanguage | null {
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return null;
  const ext = name.slice(dot + 1).toLowerCase();
  if (ext === "json") return "json";
  if (ext === "html" || ext === "htm") return "html";
  if (ext === "xml") return "xml";
  if (ext === "http" || ext === "rest") return "http";
  if (ext === "js" || ext === "mjs" || ext === "cjs") return "javascript";
  return null;
}

const HTTP_REQUEST_LINE = /^(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)\s+\S/;

/**
 * Guess the language from content. Returns null when there is not enough
 * text to decide (caller keeps "text" and may ask again later).
 * JavaScript is only detected via a node shebang - plain JS is too easy to misread.
 */
export function sniffLanguage(raw: string): EditorLanguage | null {
  const text = raw.trimStart();
  if (!text) return null;

  if (text.startsWith("#!") && /^#!.*\bnode\b/.test(text)) return "javascript";

  if (text.startsWith("{") || text.startsWith("[")) {
    try {
      JSON.parse(text);
      return "json";
    } catch {
      return null;
    }
  }

  if (text.startsWith("<")) {
    const head = text.slice(0, 200).toLowerCase();
    if (head.startsWith("<?xml")) return "xml";
    if (head.startsWith("<!doctype html") || head.startsWith("<html")) return "html";
    if (/^<[a-z_][\w.:-]*[\s>/]/.test(head)) return "xml";
    return null;
  }

  // .http: a `###` separator or a request line among the first lines.
  for (const line of text.split("\n", 20)) {
    const t = line.trim();
    if (t.startsWith("###") || HTTP_REQUEST_LINE.test(t)) return "http";
  }
  return null;
}
