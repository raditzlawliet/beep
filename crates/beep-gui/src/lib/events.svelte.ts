import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// Map of active listeners so we can tear them all down at once.
const _active = new Map<string, UnlistenFn>();

// Event handlers (Placeholder)

// Listen for backend-driven history changes (Placeholder)
export async function onHistoryChanged(cb: () => void): Promise<UnlistenFn> {
  return _register("history-changed", cb);
}

// Listen for backend status updates (Placeholder)
export async function onStatusChanged(
  cb: (payload: unknown) => void,
): Promise<UnlistenFn> {
  return _register("status-changed", cb);
}

// Helpers
async function _register(
  event: string,
  cb: (payload: unknown) => void,
): Promise<UnlistenFn> {
  // Avoid double-registration for the same key.
  _active.get(event)?.();

  const unlisten = await listen(event, (e) => cb(e.payload));
  _active.set(event, unlisten);

  return () => {
    unlisten();
    _active.delete(event);
  };
}

// Tear down all registered listeners
export function destroyAll() {
  for (const unlisten of _active.values()) unlisten();
  _active.clear();
}
