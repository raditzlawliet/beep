// `.beep` env / collection vars JSON files: detection + table <-> JSON text.

export type BeepJsonKind = "env" | "vars";

/** `.beep/env[.<name>].json` -> "env", `.beep/vars[.<part>].json` -> "vars", else null. */
export function beepJsonKind(path: string): BeepJsonKind | null {
  const parts = path.split(/[/\\]/);
  if (parts.length < 2 || parts[parts.length - 2] !== ".beep") return null;
  const name = parts[parts.length - 1];
  if (/^env(\..+)?\.json$/.test(name)) return "env";
  if (/^vars(\..+)?\.json$/.test(name)) return "vars";
  return null;
}

/** Env name of an env file: `env.json` -> null (Default), `env.dev.json` -> "dev". */
export function envNameOf(path: string): string | null {
  const name = path.split(/[/\\]/).pop() ?? "";
  const m = /^env\.(.+)\.json$/.exec(name);
  return m ? m[1] : null;
}

/** Is `path` a file of the selected env (null = Default) in any `.beep` folder? */
export function isActiveEnvFile(path: string, selected: string | null): boolean {
  return beepJsonKind(path) === "env" && envNameOf(path) === selected;
}

export type VarKind = "string" | "number" | "boolean" | "object";
export const VAR_KINDS: VarKind[] = ["string", "number", "boolean", "object"];

export interface VarsRow {
  key: string;
  /** Text shown in the table (object = compact JSON). */
  value: string;
  enabled: boolean;
  description: string;
  /** Value type; "other" (null, object without `value`) is read-only and written back as-is. */
  kind: VarKind | "other";
  raw?: unknown;
  /** Long form (`{ "value": ... }`) in the file; extra fields are kept on write. */
  extra?: Record<string, unknown>;
}

type Value = string | number | boolean | object;

function kindOf(v: unknown): VarKind | null {
  if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") return typeof v as VarKind;
  if (typeof v === "object" && v !== null) return "object";
  return null;
}

function valueRow(key: string, v: Value, kind: VarKind): VarsRow {
  return { key, value: kind === "object" ? JSON.stringify(v) : String(v), enabled: true, description: "", kind };
}

/** Rows of a vars/env JSON object, or an error message. */
export function parseVarsJson(text: string): VarsRow[] | string {
  if (text.trim() === "") return [];
  let obj: unknown;
  try {
    obj = JSON.parse(text);
  } catch (e) {
    return (e as Error).message;
  }
  if (typeof obj !== "object" || obj === null || Array.isArray(obj)) {
    return "File must contain a JSON object";
  }
  // shortcut: JS objects list integer-like keys first, so their file position is lost; fine for var names
  return Object.entries(obj as Record<string, unknown>).map(([key, v]): VarsRow => {
    // bare object = long form (needs `value`); bare array = object value
    const isLong = typeof v === "object" && v !== null && !Array.isArray(v);
    const inner = isLong ? (v as Record<string, unknown>).value : v;
    const kind = kindOf(inner);
    if (!kind) return { key, value: JSON.stringify(v), enabled: true, description: "", kind: "other", raw: v };
    if (!isLong) return valueRow(key, inner as Value, kind);
    const { value, enabled, description, ...extra } = v as Record<string, unknown>;
    return {
      ...valueRow(key, value as Value, kind),
      enabled: enabled !== false,
      description: typeof description === "string" ? description : "",
      extra,
    };
  });
}

/** Typed value of a row; null when the text does not fit its kind (then written as a string). */
export function typedValue(r: VarsRow): Value | null {
  if (r.kind === "string" || r.kind === "other") return r.value;
  try {
    const parsed: unknown = JSON.parse(r.value);
    return kindOf(parsed) === r.kind ? (parsed as Value) : null;
  } catch {
    return null;
  }
}

/** Short form when enabled, no description, no extra fields and not a `{}` value; else `{ value, description?, enabled?, ...extra }`. */
function rowEntry(r: VarsRow): unknown {
  if (r.kind === "other") return r.raw;
  const value = typedValue(r) ?? r.value;
  const extra = r.extra ?? {};
  // a bare `{...}` would read as the long form, so object values (not arrays) stay long
  const bareOk = typeof value !== "object" || Array.isArray(value);
  if (r.enabled && !r.description && Object.keys(extra).length === 0 && bareOk) return value;
  return {
    value,
    ...(r.description ? { description: r.description } : {}),
    ...(r.enabled ? {} : { enabled: false }),
    ...extra,
  };
}

/** Pretty JSON (2 spaces) in row order; empty keys dropped, `eol` used for line breaks. */
export function serializeVarsJson(rows: VarsRow[], eol = "\n"): string {
  const lines = rows
    .filter((r) => r.key.trim() !== "")
    .map((r) => `  ${JSON.stringify(r.key.trim())}: ${JSON.stringify(rowEntry(r), null, 2).replace(/\n/g, "\n  ")}`);
  const body = lines.length ? `{\n${lines.join(",\n")}\n}` : "{}";
  return body.replace(/\n/g, eol) + eol;
}
