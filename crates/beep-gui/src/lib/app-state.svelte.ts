import { invoke } from "@tauri-apps/api/core";
import type {
  AppConstants,
  HttpRequest,
  HttpResponse,
  HistoryEntry,
} from "./types";
import { defaultRequest } from "./types";

// Internal state
let _request = $state<HttpRequest>(defaultRequest());
let _response = $state<HttpResponse | null>(null);
let _history = $state<HistoryEntry[]>([]);
let _constants = $state<AppConstants | null>(null);

// Eagerly load constants at module init (SSR is disabled so invoke is safe).
invoke<AppConstants>("get_app_constants").then((c) => {
  _constants = c;
});

// request domain
export const request = {
  // current draft request
  get current() {
    return _request;
  },

  // last response
  get response() {
    return _response;
  },

  // execute an HTTP request via the Tauri
  async send(req: HttpRequest): Promise<HttpResponse> {
    try {
      const res = await invoke<HttpResponse>("execute_request", {
        payload: req,
      });
      _response = res;
      history.refresh();
      return res;
    } catch (e) {
      _response = null;
      throw e;
    }
  },

  // Update the draft request (called on every form edit).
  update(req: HttpRequest) {
    _request = req;
  },

  // Reset the form to a blank request.
  reset() {
    _request = defaultRequest();
    _response = null;
  },

  // Populate the form from a history entry.
  loadFromHistory(entry: HistoryEntry) {
    _request = { ...entry.request };
    _response = entry.response ? { ...entry.response } : null;
  },
};

// history domain
export const history = {
  get entries() {
    return _history;
  },

  // Fetch all latest history
  async refresh(): Promise<HistoryEntry[]> {
    _history = await invoke<HistoryEntry[]>("get_history");
    return _history;
  },

  // Clear all history entries
  async clear() {
    await invoke("clear_history");
    _history = [];
  },

  // Delete a single history entry
  async delete(id: number) {
    await invoke("delete_history_entry", { id });
    _history = await invoke<HistoryEntry[]>("get_history");
  },
};

// app domain
export const app = {
  // App-wide constants loaded once from the Tauri
  get constants() {
    return _constants;
  },

  // Default request headers from the Tauri (for auto-generated headers in the form).
  get defaultHeaders(): [string, string][] {
    return _constants?.default_headers ?? [];
  },
};
