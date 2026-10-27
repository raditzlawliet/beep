// CodeMirror extensions for {{variables}}: highlight, unknown-var error mark,
// autocomplete and hover with every scope value. Data comes from varIndex.
import {
  Decoration,
  EditorView,
  ViewPlugin,
  hoverTooltip,
  tooltips,
  type TooltipView,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import { RangeSetBuilder, StateEffect, type Extension } from "@codemirror/state";
import {
  autocompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { SCOPE_LABEL, VAR_RE, varIndex, type VarEntry, type VarIcon } from "./var-index.svelte";

const refreshEffect = StateEffect.define<null>();

type VarState = "known" | "unknown" | "dynamic" | "env";

function varState(inner: string): VarState {
  const key = inner.trim();
  if (key.startsWith("$")) {
    const info = varIndex.dynamicInfo(key);
    if (!info) return "unknown";
    return info.name === "$processEnv" ? "env" : "dynamic";
  }
  return varIndex.isKnown(key) ? "known" : "unknown";
}

const MARKS: Record<VarState, Decoration> = {
  known: Decoration.mark({ class: "cm-var" }),
  unknown: Decoration.mark({ class: "cm-var-unknown" }),
  dynamic: Decoration.mark({ class: "cm-var-dyn" }),
  env: Decoration.mark({ class: "cm-var-env" }),
};

// Script key: req.vars.get("key") / client.vars.set('key', ..) -> only the key is marked.
const SCRIPT_KEY_RE = /\b(?:req|client)\.vars\.(?:get|set|reset)\(\s*(["'`])([\w.-]+)\1/g;

interface VarHit {
  start: number;
  end: number;
  /** Text between the braces (or the script key). */
  inner: string;
  /** `{{inner}}` form, used for the resolved preview. */
  template: string;
}

/** Vars in `text` (offset by `base`), sorted by start. `js` adds script keys. */
function findVars(text: string, base: number, js: boolean): VarHit[] {
  const hits: VarHit[] = [];
  for (const m of text.matchAll(VAR_RE)) {
    if (!m[1].trim()) continue;
    const start = base + (m.index ?? 0);
    hits.push({ start, end: start + m[0].length, inner: m[1], template: m[0] });
  }
  if (js) {
    for (const m of text.matchAll(SCRIPT_KEY_RE)) {
      const start = base + (m.index ?? 0) + m[0].length - m[2].length - 1;
      hits.push({ start, end: start + m[2].length, inner: m[2], template: `{{${m[2]}}}` });
    }
    hits.sort((a, b) => a.start - b.start);
  }
  return hits;
}

function buildMarks(view: EditorView, js: boolean): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    for (const h of findVars(view.state.sliceDoc(from, to), from, js)) {
      builder.add(h.start, h.end, MARKS[varState(h.inner)]);
    }
  }
  return builder.finish();
}

const highlightPlugin = (js: boolean) => ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    unsubscribe: () => void;

    constructor(view: EditorView) {
      this.decorations = buildMarks(view, js);
      // Index changed (send, tab switch, vars file edit) -> redraw marks.
      this.unsubscribe = varIndex.subscribe(() => {
        view.dispatch({ effects: refreshEffect.of(null) });
      });
    }

    update(u: ViewUpdate) {
      const refreshed = u.transactions.some((tr) => tr.effects.some((e) => e.is(refreshEffect)));
      if (u.docChanged || u.viewportChanged || refreshed) {
        this.decorations = buildMarks(u.view, js);
      }
    }

    destroy() {
      this.unsubscribe();
    }
  },
  { decorations: (v) => v.decorations },
);

// --- Autocomplete ---

const ICON_TYPE: Record<VarIcon, string> = {
  S: "var-s",
  C: "var-c",
  R: "var-r",
  F: "var-f",
  D: "var-d",
  $: "var-dyn",
};

function keyOptions(close: string): Completion[] {
  return varIndex.all().map((e) => ({
    label: e.key,
    type: ICON_TYPE[e.icon],
    detail: e.value ?? e.note ?? "",
    apply: e.key + close,
  }));
}

function varCompletions(js: boolean) {
  return (ctx: CompletionContext): CompletionResult | null => {
    // {{ key  (any editor)
    const m = ctx.matchBefore(/\{\{\s*\$?[\w.-]*$/);
    if (m) {
      const word = /\$?[\w.-]*$/.exec(m.text)?.[0] ?? "";
      const from = ctx.pos - word.length;
      const close = ctx.state.sliceDoc(ctx.pos, ctx.pos + 2) === "}}" ? "" : "}}";
      const dynamic: Completion[] = varIndex.dynamic.map((d) => ({
        label: d.name,
        type: ICON_TYPE.$,
        detail: d.description,
        apply: d.usage + close,
      }));
      return { from, options: [...keyOptions(close), ...dynamic], validFor: /^\$?[\w.-]*$/ };
    }
    // req.vars.get("key") / client.vars.set("key", ...)  (scripts)
    if (js) {
      const s = ctx.matchBefore(/vars\.(get|set|reset)\(\s*["'`][\w.-]*$/);
      if (s) {
        const word = /[\w.-]*$/.exec(s.text)?.[0] ?? "";
        return { from: ctx.pos - word.length, options: keyOptions(""), validFor: /^[\w.-]*$/ };
      }
    }
    return null;
  };
}

// --- Hover ---
// Postman / Bruno style: line 1 = name + winning scope (click to open it),
// line 2 = value input (edits the winning scope) + copy, then overridden scopes.

const SVG_COPY =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>';
const SVG_CHECK =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>';
const SVG_OPEN =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 7h10v10"/><path d="M7 17 17 7"/></svg>';

function el(tag: string, cls: string, text?: string): HTMLElement {
  const e = document.createElement(tag);
  e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

/** Scope chip: icon + label (+ origin). A button when the scope can be opened. */
function scopeChip(e: VarEntry, cls: string): HTMLElement {
  const nav = varIndex.canNavigate(e);
  const chip = el(nav ? "button" : "span", cls + (nav ? " nav" : ""));
  chip.append(el("span", "cm-var-icon", e.icon), el("span", "cm-var-scope", SCOPE_LABEL[e.icon]));
  if (e.note) chip.append(el("span", "cm-var-note", e.note));
  if (nav) {
    const open = el("span", "cm-var-open");
    open.innerHTML = SVG_OPEN;
    chip.append(open);
    chip.title = `Open ${SCOPE_LABEL[e.icon]}`;
    chip.addEventListener("click", (ev) => {
      ev.preventDefault();
      varIndex.navigate(e);
    });
  }
  return chip;
}

/** Value input + copy button. `onCommit` is set when the value is editable. */
function valueField(
  value: string,
  placeholder: string,
  onCommit: ((v: string) => Promise<void>) | null,
): { dom: HTMLElement; flush: () => void } {
  const field = el("div", "cm-var-field");
  const input = document.createElement("input");
  input.className = "cm-var-input";
  input.value = value;
  input.placeholder = placeholder;
  input.spellcheck = false;
  input.readOnly = !onCommit;
  if (!onCommit) input.title = "Read-only";

  const copy = el("button", "cm-var-copy");
  copy.innerHTML = SVG_COPY;
  copy.title = "Copy value";
  copy.addEventListener("mousedown", (ev) => ev.preventDefault()); // keep input focus
  copy.addEventListener("click", () => {
    navigator.clipboard
      .writeText(input.value)
      .then(() => {
        copy.innerHTML = SVG_CHECK;
        setTimeout(() => (copy.innerHTML = SVG_COPY), 1000);
      })
      .catch((e) => console.error("Copy failed:", e));
  });
  field.append(input, copy);

  let saved = value;
  const error = el("div", "cm-var-hover-error");
  const flush = () => {
    if (!onCommit || input.value === saved) return;
    const next = input.value;
    saved = next;
    error.remove();
    onCommit(next).catch((e) => {
      console.error("Failed to set var:", e);
      error.textContent = `Not saved: ${e}`;
      field.after(error);
    });
  };
  if (onCommit) {
    input.addEventListener("keydown", (ev) => {
      ev.stopPropagation();
      if (ev.key === "Enter") {
        ev.preventDefault();
        flush();
      } else if (ev.key === "Escape") {
        ev.preventDefault();
        input.value = saved;
        input.blur();
      }
    });
    input.addEventListener("blur", flush);
  }
  return { dom: field, flush };
}

function hoverView(inner: string, placeholder: string): TooltipView {
  const key = inner.trim();
  const dom = el("div", "cm-var-hover");
  const head = el("div", "cm-var-hover-head");
  const name = el("span", "cm-var-name", key);
  head.append(name);
  dom.append(head);
  const unknown = (info: string) => {
    name.classList.add("unknown");
    dom.append(el("div", "cm-var-desc", info));
    return view;
  };
  const view: TooltipView = { dom };

  if (key.startsWith("$")) {
    const info = varIndex.dynamicInfo(key);
    if (!info) return unknown("Unknown dynamic variable. Sent as text.");
    const chip = el("span", "cm-var-chip");
    chip.append(el("span", "cm-var-icon", "$"), el("span", "cm-var-scope", SCOPE_LABEL.$));
    head.append(chip);
    dom.append(valueField(placeholder, "", null).dom, el("div", "cm-var-desc", info.description));
    if (info.name === "$processEnv") {
      dom.append(el("div", "cm-var-hover-warn", "Reads an OS environment variable. Beep asks before sending."));
    }
    return view;
  }

  const entries = varIndex.lookup(key);
  if (entries.length === 0) return unknown("Unknown variable. Not defined in any scope, sent as text.");

  const [winner, ...rest] = entries;
  head.append(scopeChip(winner, "cm-var-chip"));
  const field = valueField(
    winner.value ?? "",
    winner.value === null ? "(set at runtime)" : "",
    varIndex.canEdit(winner) ? (v) => varIndex.edit(winner, v) : null,
  );
  dom.append(field.dom);
  view.destroy = field.flush;

  // Final value with nested vars resolved (static scopes + client.vars, no scripts).
  const resolved = el("div", "cm-var-hover-resolved");
  dom.append(resolved);
  varIndex
    .preview(placeholder)
    .then((v) => {
      if (v !== placeholder && v !== winner.value) resolved.textContent = `= ${v}`;
    })
    .catch(() => {});

  if (rest.length > 0) {
    const list = el("div", "cm-var-others");
    list.append(el("div", "cm-var-others-title", "Overridden"));
    for (const e of rest) {
      const row = el("div", "cm-var-other");
      row.append(scopeChip(e, "cm-var-chip small"), el("span", "cm-var-value", e.value ?? "(runtime)"));
      list.append(row);
    }
    dom.append(list);
  }
  return view;
}

// Opens below the line (CodeMirror flips it above when there is no room),
// so it does not cover the toolbar over top-of-page inputs.
const varHover = (js: boolean) =>
  hoverTooltip((view, pos) => {
    const line = view.state.doc.lineAt(pos);
    const hit = findVars(line.text, line.from, js).find((h) => pos >= h.start && pos <= h.end);
    if (!hit) return null;
    return {
      pos: hit.start,
      end: hit.end,
      create: () => hoverView(hit.inner, hit.template),
    };
  });

// --- Theme ---

const ICON_CSS: Record<string, string> = {
  "var-s": "S",
  "var-c": "C",
  "var-r": "R",
  "var-f": "F",
  "var-d": "D",
  "var-dyn": "$",
};

const varTheme = EditorView.baseTheme({
  ".cm-var": { color: "#AE81FF" },
  ".cm-var-dyn": { color: "#66D9EF" },
  ".cm-var-env": { color: "#FD971F", textDecoration: "underline dotted #FD971F" },
  ".cm-var-unknown": {
    color: "#F92672",
    textDecoration: "underline wavy #F92672",
    textUnderlineOffset: "3px",
  },
  // Syntax spans inside a var (JS string token) must not repaint it.
  ".cm-var *, .cm-var-dyn *, .cm-var-env *, .cm-var-unknown *": { color: "inherit" },
  ".cm-var-hover": {
    padding: "6px 8px",
    fontSize: "12px",
    width: "340px",
    maxWidth: "80vw",
    display: "flex",
    flexDirection: "column",
    gap: "4px",
  },
  ".cm-var-hover-head": { display: "flex", alignItems: "center", gap: "8px", minWidth: "0" },
  ".cm-var-name": { fontFamily: "monospace", fontWeight: "bold", color: "#AE81FF" },
  ".cm-var-name.unknown": { color: "#F92672" },
  ".cm-var-chip": {
    display: "inline-flex",
    alignItems: "center",
    gap: "4px",
    minWidth: "0",
    marginLeft: "auto",
    padding: "1px 6px 1px 2px",
    border: "1px solid color-mix(in srgb, var(--color-base-content) 15%, transparent)",
    borderRadius: "var(--radius-field)",
    background: "transparent",
    color: "color-mix(in srgb, var(--color-base-content) 60%, transparent)",
    font: "inherit",
    fontSize: "11px",
  },
  ".cm-var-chip.nav": { cursor: "pointer" },
  ".cm-var-chip.nav:hover": { background: "var(--color-base-300)", color: "var(--color-base-content)" },
  ".cm-var-chip.nav:hover .cm-var-open": { opacity: "1" },
  ".cm-var-chip.small": { marginLeft: "0", flexShrink: "0", fontSize: "10px" },
  ".cm-var-icon": {
    display: "inline-flex",
    justifyContent: "center",
    width: "16px",
    borderRadius: "3px",
    background: "#3E3D32",
    fontFamily: "monospace",
    fontWeight: "bold",
    flexShrink: "0",
  },
  ".cm-var-scope": { flexShrink: "0" },
  ".cm-var-note": { opacity: "0.5", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" },
  ".cm-var-open": { display: "inline-flex", width: "11px", height: "11px", opacity: "0.6", flexShrink: "0" },
  ".cm-var-field": {
    display: "flex",
    alignItems: "center",
    gap: "2px",
    border: "1px solid color-mix(in srgb, var(--color-base-content) 20%, transparent)",
    borderRadius: "var(--radius-field)",
    background: "var(--color-base-100)",
    padding: "0 2px 0 6px",
  },
  ".cm-var-field:focus-within": { borderColor: "var(--color-primary)" },
  ".cm-var-input": {
    flex: "1",
    minWidth: "0",
    height: "24px",
    background: "transparent",
    border: "none",
    outline: "none",
    color: "inherit",
    fontFamily: "monospace",
    fontSize: "12px",
  },
  ".cm-var-input:read-only": { opacity: "0.7" },
  ".cm-var-copy": {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    width: "20px",
    height: "20px",
    padding: "4px",
    border: "none",
    borderRadius: "var(--radius-field)",
    background: "transparent",
    color: "inherit",
    opacity: "0.6",
    cursor: "pointer",
  },
  ".cm-var-copy:hover": { opacity: "1", background: "var(--color-base-300)" },
  ".cm-var-desc": { opacity: "0.6" },
  ".cm-var-others": {
    display: "flex",
    flexDirection: "column",
    gap: "2px",
    marginTop: "2px",
    paddingTop: "4px",
    borderTop: "1px solid color-mix(in srgb, var(--color-base-content) 10%, transparent)",
    opacity: "0.6",
  },
  ".cm-var-others-title": { fontSize: "10px", textTransform: "uppercase", letterSpacing: "0.05em" },
  ".cm-var-other": { display: "flex", alignItems: "center", gap: "6px", minWidth: "0" },
  ".cm-var-value": {
    fontFamily: "monospace",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    textDecoration: "line-through",
  },
  ".cm-var-hover-error": { color: "#F92672" },
  ".cm-var-hover-warn": { color: "#FD971F" },
  ".cm-var-hover-resolved": { fontFamily: "monospace", opacity: "0.6", wordBreak: "break-all" },
  ...Object.fromEntries(
    Object.entries(ICON_CSS).map(([type, ch]) => [
      `.cm-completionIcon-${type}::after`,
      { content: `"${ch}"`, fontFamily: "monospace", fontWeight: "bold" },
    ]),
  ),
});

// Popup look like the app dropdowns (bg-base-200, border-base-content/20, rounded-box).
// Two classes per selector so it beats the CodeMirror light base theme.
const popupTheme = EditorView.theme({
  ".cm-tooltip.cm-tooltip-hover, .cm-tooltip.cm-tooltip-autocomplete, .cm-tooltip.cm-completionInfo": {
    backgroundColor: "var(--color-base-200)",
    color: "var(--color-base-content)",
    border: "1px solid color-mix(in srgb, var(--color-base-content) 20%, transparent)",
    borderRadius: "var(--radius-box)",
    boxShadow: "0 10px 15px -3px rgb(0 0 0 / 0.3)",
    fontSize: "12px",
  },
  ".cm-tooltip.cm-tooltip-autocomplete": { padding: "2px 0" },
  ".cm-tooltip.cm-tooltip-autocomplete > ul": { fontFamily: "monospace", maxHeight: "12rem" },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li": { padding: "1px 8px" },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "var(--color-base-300)",
    color: "var(--color-base-content)",
    outline: "1px solid var(--color-primary)",
    outlineOffset: "-1px",
  },
  ".cm-tooltip.cm-tooltip-autocomplete .cm-completionMatchedText": {
    color: "var(--color-primary)",
    textDecoration: "none",
  },
  ".cm-tooltip.cm-tooltip-autocomplete .cm-completionDetail": { opacity: "0.5", marginLeft: "8px" },
  ".cm-tooltip.cm-tooltip-autocomplete .cm-completionIcon": { opacity: "0.7" },
});

/** Var support for an editor. `js` adds `vars.get("key")` keys (completion, marks, hover). */
export function varExtensions(opts: { js?: boolean } = {}): Extension {
  const js = !!opts.js;
  return [
    highlightPlugin(js),
    autocompletion({ override: [varCompletions(js)] }),
    varHover(js),
    // Render tooltips on body so table cells / scrollers do not clip them.
    tooltips({ parent: document.body }),
    varTheme,
    popupTheme,
  ];
}
