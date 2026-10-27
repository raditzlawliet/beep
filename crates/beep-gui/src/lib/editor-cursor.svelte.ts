// Cursor of the active code editor (FileEditor) for the status bar and Go To Line (Ctrl+G).
// Only the active tab's FileEditor is mounted, so a set target = code mode is active.

export interface EditorCursor {
  /** 1-based line. */
  line: number;
  /** 1-based column (chars). */
  col: number;
  /** Total lines in the doc. */
  lines: number;
}

/** What Go To Line drives. Set by the mounted FileEditor. */
export interface GotoTarget {
  /** Move the cursor and scroll there (clamped). `focus` = commit and give focus back. */
  goto(line: number, col: number | null, focus: boolean): void;
  /** Selection + scroll before the preview started. */
  snapshot(): unknown;
  /** Undo the preview. `focus` = give focus back to the editor. */
  restore(snap: unknown, focus: boolean): void;
}

let _cursor = $state<EditorCursor | null>(null);
// raw: identity check on unmount must see the same object (no proxy).
let _target = $state.raw<GotoTarget | null>(null);
let _gotoOpen = $state(false);

export const editorCursor = {
  get current() {
    return _cursor;
  },
  set(c: EditorCursor | null) {
    _cursor = c;
  },
  get target() {
    return _target;
  },
  setTarget(t: GotoTarget | null) {
    _target = t;
    if (!t) _gotoOpen = false;
  },
  get gotoOpen() {
    return _gotoOpen;
  },
  /** Open Go To Line. False when no code editor is active. */
  openGoto(): boolean {
    if (!_target) return false;
    _gotoOpen = true;
    return true;
  },
  closeGoto() {
    _gotoOpen = false;
  },
};

/** Parse `9`, `9:44`, `9,44` (spaces ok). Null when not a line number. */
export function parseLineCol(text: string): { line: number; col: number | null } | null {
  const m = text.trim().match(/^(\d+)\s*(?:[:,]\s*(\d*))?$/);
  if (!m) return null;
  const line = Number(m[1]);
  const col = m[2] ? Number(m[2]) : null;
  return { line, col };
}
