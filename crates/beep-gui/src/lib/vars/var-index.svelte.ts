// Variable index for the editors: every known value of every {{var}} for the active
// request, with its scope. Feeds highlight, unknown check, autocomplete and hover.
import { invoke } from "@tauri-apps/api/core";
import type { DynamicVarInfo, ParsedFileVariable, ParsedRequest, VarSource } from "$lib/types";

/** One-char scope icon. S = req.vars (script), C = client.vars, R = request @var,
 *  F = file @var, D = folder .beep json, $ = dynamic. */
export type VarIcon = "S" | "C" | "R" | "F" | "D" | "$";

export interface VarEntry {
  key: string;
  icon: VarIcon;
  /** Value, or null when only known from a script (not run yet). */
  value: string | null;
  /** Short note: origin file, "set by pre-script", ... */
  note: string | null;
  /** Full origin path (folder json file), for navigate + edit. */
  origin?: string | null;
}

/** Navigate / edit handlers for scopes that live in the open tab. Set by the viewer. */
export interface VarActions {
  openRequestVars?: () => void;
  openFileVars?: () => void;
  editRequestVar?: (key: string, value: string) => Promise<void>;
  editFileVar?: (key: string, value: string) => Promise<void>;
  /** Open a file in a tab (folder json). */
  openFile?: (path: string) => void;
}

export const SCOPE_LABEL: Record<VarIcon, string> = {
  S: "req.vars (script)",
  C: "client.vars (session)",
  R: "request @var",
  F: "file @var",
  D: "folder vars",
  $: "dynamic",
};

export interface VarContext {
  requestVars: ParsedFileVariable[];
  fileVars: ParsedFileVariable[];
  sourceFileDir: string | null;
  projectRoot: string | null;
  /** Active request scripts (inline code only). */
  preScript: string | null;
  postScript: string | null;
  /** Every inline script in the file (client.vars may be set by any request). */
  fileScripts: string[];
}

const ICON_OF: Record<VarSource["scope"], VarIcon> = {
  client: "C",
  request: "R",
  file: "F",
  folder: "D",
};

let _ctx: VarContext | null = null;
let _sources: VarSource[] = [];
let _dynamic: DynamicVarInfo[] = [];
let _gen = 0;
let _timer: ReturnType<typeof setTimeout> | undefined;
const _listeners = new Set<() => void>();
// Template text of the active request, for the status bar unknown-var count.
let _activeText = $state("");

/** Every template string of a request that gets {{vars}} resolved on send. */
export function requestTemplates(r: ParsedRequest | undefined): string {
  if (!r) return "";
  const parts: string[] = [r.url, r.body ?? "", r.multipart_boundary ?? ""];
  for (const h of r.headers) if (h.enabled && !h.auto) parts.push(h.key, h.value);
  for (const q of r.query_params) if (q.enabled) parts.push(q.key, q.value);
  for (const f of [...r.form_urlencoded, ...r.form_multipart]) if (f.enabled) parts.push(f.key, f.value);
  for (const v of r.variables) parts.push(v.value);
  return parts.join("\n");
}

/** Keys set by `<obj>.vars.set("key", ...)` in a script. */
function scriptKeys(code: string | null, obj: "req" | "client"): string[] {
  if (!code) return [];
  const re = new RegExp(`${obj}\\.vars\\.set\\(\\s*["'\`]([^"'\`]+)["'\`]`, "g");
  return [...code.matchAll(re)].map((m) => m[1].trim());
}

function buildIndex(): Map<string, VarEntry[]> {
  const map = new Map<string, VarEntry[]>();
  const push = (e: VarEntry) => {
    const list = map.get(e.key);
    if (list) list.push(e);
    else map.set(e.key, [e]);
  };

  // Highest precedence first: req.vars > client.vars > request > file > folder.
  for (const key of new Set(scriptKeys(_ctx?.preScript ?? null, "req"))) {
    push({ key, icon: "S", value: null, note: "set by pre-script at send" });
  }
  const clientSources = _sources.filter((s) => s.scope === "client");
  for (const s of clientSources) push({ key: s.key, icon: "C", value: s.value, note: null });
  const clientKnown = new Set(clientSources.map((s) => s.key));
  const scripts = _ctx?.fileScripts ?? [];
  for (const key of new Set(scripts.flatMap((c) => scriptKeys(c, "client")))) {
    if (!clientKnown.has(key)) push({ key, icon: "C", value: null, note: "set by a script, not run yet" });
  }
  for (const s of _sources) {
    if (s.scope === "client") continue;
    push({
      key: s.key,
      icon: ICON_OF[s.scope],
      value: s.value,
      note: s.origin ? shortPath(s.origin) : null,
      origin: s.origin,
    });
  }
  return map;
}

function shortPath(p: string): string {
  const root = _ctx?.projectRoot;
  if (root && p.startsWith(root)) return p.slice(root.length).replace(/^[/\\]/, "");
  return p;
}

let _index = new Map<string, VarEntry[]>();
let _actions: VarActions = {};

/** Set `key` in a folder vars json file. Keeps other keys and the value type when it still fits. */
async function writeFolderVar(path: string, key: string, value: string) {
  const text = await invoke<string>("read_file_content", { path });
  const obj = JSON.parse(text) as Record<string, unknown>;
  const old = obj[key];
  let next: unknown = value;
  if (typeof old === "number" || typeof old === "boolean") {
    try {
      const parsed = JSON.parse(value);
      if (typeof parsed === typeof old) next = parsed;
    } catch {
      // not a number/bool anymore -> string
    }
  }
  obj[key] = next;
  const eol = text.includes("\r\n") ? "\r\n" : "\n";
  await invoke("save_file_content", { path, content: JSON.stringify(obj, null, 2).replace(/\n/g, eol) + eol });
}

// Bumped on every index change, so components can react to it.
let _version = $state(0);

function notify() {
  _index = buildIndex();
  _version++;
  for (const fn of _listeners) fn();
}

async function refresh() {
  const ctx = _ctx;
  const gen = ++_gen;
  if (!ctx) {
    _sources = [];
    notify();
    return;
  }
  try {
    const sources = await invoke<VarSource[]>("get_var_sources", {
      requestVars: ctx.requestVars,
      fileVars: ctx.fileVars,
      sourceFileDir: ctx.sourceFileDir,
      projectRoot: ctx.projectRoot,
    });
    if (gen !== _gen) return;
    _sources = sources;
  } catch (e) {
    console.error("Failed to load var sources:", e);
  }
  notify();
}

function sameCtx(a: VarContext | null, b: VarContext | null): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

export const varIndex = {
  get version() {
    return _version;
  },

  get dynamic(): DynamicVarInfo[] {
    return _dynamic;
  },

  setDynamic(list: DynamicVarInfo[]) {
    _dynamic = list;
    notify();
  },

  /** Set the active request context; reloads sources (debounced) when it changed. */
  setContext(ctx: VarContext | null) {
    if (sameCtx(_ctx, ctx)) return;
    _ctx = ctx ? $state.snapshot(ctx) : null;
    clearTimeout(_timer);
    _timer = setTimeout(refresh, 150);
  },

  refresh,

  /** Active request template text (cheap, no IPC). */
  setActiveText(text: string) {
    _activeText = text;
  },

  /** Unknown {{keys}} in the active request (reactive). */
  get unknown(): string[] {
    void _version;
    return this.unknownIn(_activeText);
  },

  /** All entries for `key`, highest precedence first (first one wins). */
  lookup(key: string): VarEntry[] {
    return _index.get(key) ?? [];
  },

  /** Every key with its winning entry. */
  all(): VarEntry[] {
    return [..._index.values()].map((list) => list[0]);
  },

  dynamicInfo(expr: string): DynamicVarInfo | undefined {
    const name = "$" + expr.trim().replace(/^\$/, "").split(/\s+/)[0];
    return _dynamic.find((d) => d.name === name);
  },

  /** Known variable, or a valid dynamic name. */
  isKnown(expr: string): boolean {
    const key = expr.trim();
    if (key.startsWith("$")) return !!this.dynamicInfo(key);
    return _index.has(key);
  },

  /** Unknown {{keys}} in `text`, unique, in order. */
  unknownIn(text: string): string[] {
    const out: string[] = [];
    for (const m of text.matchAll(VAR_RE)) {
      const key = m[1].trim();
      if (key && !this.isKnown(key) && !out.includes(key)) out.push(key);
    }
    return out;
  },

  /** Resolve `text` over static scopes + client.vars (Rust), for hover previews. */
  async preview(text: string): Promise<string> {
    const ctx = _ctx;
    if (!ctx) return text;
    return invoke<string>("resolve_preview", {
      text,
      requestVars: ctx.requestVars,
      fileVars: ctx.fileVars,
      sourceFileDir: ctx.sourceFileDir,
      projectRoot: ctx.projectRoot,
    });
  },

  /** Register navigate / edit handlers (merged; pass undefined to clear one). */
  setActions(actions: VarActions) {
    _actions = { ..._actions, ...actions };
  },

  /** Can `entry` be opened somewhere? */
  canNavigate(entry: VarEntry): boolean {
    switch (entry.icon) {
      case "R": return !!_actions.openRequestVars;
      case "F": return !!_actions.openFileVars;
      case "D": return !!_actions.openFile && !!entry.origin;
      default: return false;
    }
  },

  navigate(entry: VarEntry) {
    switch (entry.icon) {
      case "R": _actions.openRequestVars?.(); break;
      case "F": _actions.openFileVars?.(); break;
      case "D": if (entry.origin) _actions.openFile?.(entry.origin); break;
    }
  },

  /** Can the value of `entry` be edited from the popup? */
  canEdit(entry: VarEntry): boolean {
    switch (entry.icon) {
      case "C": return entry.value !== null;
      case "R": return !!_actions.editRequestVar;
      case "F": return !!_actions.editFileVar;
      case "D": return !!entry.origin;
      default: return false;
    }
  },

  /** Write a new value into the scope of `entry`, then reload the index. */
  async edit(entry: VarEntry, value: string) {
    switch (entry.icon) {
      case "C": await invoke("set_client_var", { key: entry.key, value }); break;
      case "R": await _actions.editRequestVar?.(entry.key, value); break;
      case "F": await _actions.editFileVar?.(entry.key, value); break;
      case "D": if (entry.origin) await writeFolderVar(entry.origin, entry.key, value); break;
      default: return;
    }
    await refresh();
  },

  /** Called after any index change (editors redraw). Returns unsubscribe. */
  subscribe(fn: () => void): () => void {
    _listeners.add(fn);
    return () => _listeners.delete(fn);
  },
};

/** `{{ ... }}` placeholder; group 1 = inner text. */
export const VAR_RE = /\{\{([^{}]*?)\}\}/g;
