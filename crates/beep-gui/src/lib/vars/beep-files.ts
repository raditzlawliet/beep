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

export interface VarsRow {
  key: string;
  /** Text shown in the table. */
  value: string;
  enabled: boolean;
  description: string;
  /** Original JSON type of the value; "other" (null/array/object without `value`) is read-only and written back as-is. */
  kind: "string" | "number" | "boolean" | "other";
  raw?: unknown;
  /** Long form (`{ "value": ... }`) in the file; extra fields are kept on write. */
  extra?: Record<string, unknown>;
}

type Scalar = string | number | boolean;
const isScalar = (v: unknown): v is Scalar =>
  typeof v === "string" || typeof v === "number" || typeof v === "boolean";

function scalarRow(key: string, v: Scalar): VarsRow {
  return { key, value: String(v), enabled: true, description: "", kind: typeof v as VarsRow["kind"] };
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
    if (isScalar(v)) return scalarRow(key, v);
    if (typeof v === "object" && v !== null && !Array.isArray(v) && isScalar((v as Record<string, unknown>).value)) {
      const { value, enabled, description, ...extra } = v as Record<string, unknown>;
      return {
        ...scalarRow(key, value as Scalar),
        enabled: enabled !== false,
        description: typeof description === "string" ? description : "",
        extra,
      };
    }
    return { key, value: JSON.stringify(v), enabled: true, description: "", kind: "other", raw: v };
  });
}

/** Short form when enabled, no description and no extra fields; else `{ value, description?, enabled?, ...extra }`. */
function rowEntry(r: VarsRow): unknown {
  if (r.kind === "other") return r.raw;
  const value = rowValue(r);
  const extra = r.extra ?? {};
  if (r.enabled && !r.description && Object.keys(extra).length === 0) return value;
  return {
    value,
    ...(r.description ? { description: r.description } : {}),
    ...(r.enabled ? {} : { enabled: false }),
    ...extra,
  };
}

function rowValue(r: VarsRow): Scalar {
  if (r.kind === "number" || r.kind === "boolean") {
    try {
      const parsed = JSON.parse(r.value);
      if (typeof parsed === r.kind) return parsed;
    } catch {
      // not a number/bool anymore -> string
    }
  }
  return r.value;
}

/** Pretty JSON (2 spaces) in row order; empty keys dropped, `eol` used for line breaks. */
export function serializeVarsJson(rows: VarsRow[], eol = "\n"): string {
  const lines = rows
    .filter((r) => r.key.trim() !== "")
    .map((r) => `  ${JSON.stringify(r.key.trim())}: ${JSON.stringify(rowEntry(r), null, 2).replace(/\n/g, "\n  ")}`);
  const body = lines.length ? `{\n${lines.join(",\n")}\n}` : "{}";
  return body.replace(/\n/g, eol) + eol;
}
