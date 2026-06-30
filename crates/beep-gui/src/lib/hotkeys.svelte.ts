import { createHotkey, createHotkeySequence } from "@tanstack/svelte-hotkeys";
import { getCurrentWindow } from "@tauri-apps/api/window";

// --- Send handler ref (for Ctrl+Enter)
// MainEditorViewer sets this when its form request is ready.
let _sendHandler: (() => void) | null = $state(null);

export function setSendHandler(fn: (() => void) | null) {
  _sendHandler = fn;
}

// --- Mode handler ref (for Ctrl+1/2/3)
// MainEditorViewer sets this to switch view modes (code/file/request).
let _modeHandler: ((mode: string) => void) | null = $state(null);

export function setModeHandler(fn: ((mode: string) => void) | null) {
  _modeHandler = fn;
}

// --- Tab switcher toggle ref (for Ctrl+Tab)
// +page.svelte sets this to toggle the tab switcher popup.
let _tabSwitcherToggle: (() => void) | null = $state(null);

export function setTabSwitcherToggle(fn: (() => void) | null) {
  _tabSwitcherToggle = fn;
}

// --- New request popup ref (for Ctrl+N)
// TitleBar sets this to open the NewRequestPopup.
let _newPopupHandler: (() => void) | null = $state(null);

export function setNewPopupHandler(fn: (() => void) | null) {
  _newPopupHandler = fn;
}

// --- Hotkey handler interface
export interface HotkeyHandlers {
  // Ctrl+S - save current tab
  onSave: () => void;
  // Ctrl+K S - save all tabs
  onSaveAll: () => void;
  // Ctrl+Shift+N - smart new (in-file if http, else untitled)
  onNewRequest: () => void;
  // Ctrl+O - open project
  onOpenProject: () => void;
  // Ctrl+W - close current tab
  onCloseTab: () => void;
  // Ctrl+Shift+E - toggle project panel focus
  onToggleProjectFocus: () => void;
  // Ctrl+Shift+H - toggle history panel focus
  onToggleHistoryFocus: () => void;
  // Ctrl+B - toggle sidebar open/close (keep active panel)
  onToggleSidebar: () => void;
}

/**
 * Register all global application hotkeys.
 * Call once from the root component (+page.svelte).
 *
 * Uses `Mod` for cross-platform: Cmd on macOS, Ctrl on Windows/Linux.
 * Future: extract key bindings to a configurable keymap object.
 */
export function registerHotkeys(handlers: HotkeyHandlers) {
  // --- File operations
  // ref: guard so Mod+S bails out if the Ctrl+K S sequence already fired
  // This somehow working correctly, but it mention has issue on
  // https://github.com/TanStack/hotkeys/issues/116
  // Let leave this as comments for now.
  // let _saveAllFired = false;

  // Register sequence FIRST (order matters; its callback runs before Mod+S)
  createHotkeySequence(["Mod+K", "S"], () => {
    // _saveAllFired = true;
    handlers.onSaveAll();
  });

  createHotkey("Mod+S", (e) => {
    // if (_saveAllFired) {
    //   _saveAllFired = false;
    //   return;
    // }
    e.preventDefault();
    handlers.onSave();
  });

  // --- New
  // Ctrl+N opens the NewRequestPopup (TitleBar registers the handler)
  createHotkey("Mod+N", (e) => {
    e.preventDefault();
    _newPopupHandler?.();
  });

  // Ctrl+Shift+N is smart new (in-file if http tab active, else untitled)
  createHotkey("Mod+Shift+N", (e) => {
    e.preventDefault();
    handlers.onNewRequest();
  });

  // --- Project
  createHotkey("Mod+O", (e) => {
    e.preventDefault();
    handlers.onOpenProject();
  });

  // --- Close tab
  createHotkey("Mod+W", (e) => {
    e.preventDefault();
    handlers.onCloseTab();
  });

  // --- Sidebar toggles
  createHotkey("Mod+Shift+E", (e) => {
    e.preventDefault();
    handlers.onToggleProjectFocus();
  });

  createHotkey("Mod+Shift+H", (e) => {
    e.preventDefault();
    handlers.onToggleHistoryFocus();
  });

  createHotkey("Mod+B", (e) => {
    e.preventDefault();
    handlers.onToggleSidebar();
  });

  // --- Quit
  createHotkey("Mod+Q", async (e) => {
    e.preventDefault();
    await getCurrentWindow().close();
  });

  // --- Send request (only when MainEditorViewer has registered a handler)
  createHotkey("Mod+Enter", (e) => {
    if (_sendHandler) {
      e.preventDefault();
      _sendHandler();
    }
  });

  // --- View mode switch (Ctrl+1/2/3, only for http-file tabs)
  createHotkey("Mod+1", (e) => {
    if (_modeHandler) {
      e.preventDefault();
      _modeHandler("code");
    }
  });

  createHotkey("Mod+2", (e) => {
    if (_modeHandler) {
      e.preventDefault();
      _modeHandler("file");
    }
  });

  createHotkey("Mod+3", (e) => {
    if (_modeHandler) {
      e.preventDefault();
      _modeHandler("request");
    }
  });

  // --- Tab switcher (Ctrl+Tab)
  createHotkey("Control+Tab", (e) => {
    e.preventDefault();
    _tabSwitcherToggle?.();
  });
}
