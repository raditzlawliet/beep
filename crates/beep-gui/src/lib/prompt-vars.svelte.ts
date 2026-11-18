import type { ParsedPrompt } from "./types";

// `// @prompt` values asked before a send. One dialog at a time (PromptVarsDialog).
interface Pending {
  prompts: ParsedPrompt[];
  resolve: (values: Record<string, string> | null) => void;
}

let _pending = $state<Pending | null>(null);
// Last non-secret value per name, app session only. `@prompt:secret` values are never kept.
const _last = new Map<string, string>();

export const promptVars = {
  get pending() {
    return _pending;
  },
  initial(p: ParsedPrompt): string {
    return p.secret ? "" : (_last.get(p.name) ?? "");
  },
  submit(values: Record<string, string>) {
    for (const p of _pending?.prompts ?? []) {
      if (!p.secret) _last.set(p.name, values[p.name] ?? "");
    }
    _pending?.resolve(values);
    _pending = null;
  },
  cancel() {
    _pending?.resolve(null);
    _pending = null;
  },
};

/** Ask the user for `prompts`; null when cancelled. */
export function askPromptVars(prompts: ParsedPrompt[]): Promise<Record<string, string> | null> {
  _pending?.resolve(null);
  return new Promise((resolve) => (_pending = { prompts, resolve }));
}
