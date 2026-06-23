import { invoke } from "@tauri-apps/api/core";
import type {
  AppConstants,
  HttpRequest,
  HttpResponse,
  HistoryEntry,
  HistoryEntrySummary,
  ProjectNode,
} from "./types";
import { defaultRequest } from "./types";

function mergeChildren(
  tree: ProjectNode[],
  targetPath: string | null,
  newChildren: ProjectNode[],
): ProjectNode[] {
  if (targetPath === null) {
    // root level - merge into top-level tree entries
    return newChildren.map((nc) => {
      if (nc.is_dir && !nc.children) {
        const old = tree.find((c) => c.path === nc.path);
        if (old?.children) return { ...nc, children: old.children };
      }
      return nc;
    });
  }

  return tree.map((node) => {
    if (node.path === targetPath) {
      const merged = newChildren.map((nc) => {
        if (nc.is_dir && !nc.children) {
          const old = node.children?.find((c) => c.path === nc.path);
          if (old?.children) return { ...nc, children: old.children };
        }
        return nc;
      });
      return { ...node, children: merged };
    }
    if (node.children) {
      return {
        ...node,
        children: mergeChildren(node.children, targetPath, newChildren),
      };
    }
    return node;
  });
}

// Internal state
let _request = $state<HttpRequest>(defaultRequest());
let _response = $state<HttpResponse | null>(null);
let _history = $state<HistoryEntrySummary[]>([]);
let _constants = $state<AppConstants | null>(null);
let _projectPath = $state<string | null>(null);
let _projectName = $state<string | null>(null);
let _projectTree = $state<ProjectNode[]>([]);

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
      history.refresh().catch(() => {});
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
  async loadFromHistory(summary: HistoryEntrySummary) {
    try {
      const entry = await invoke<HistoryEntry>("get_history_entry", {
        id: summary.id,
      });
      _request = { ...entry.request };
      _response = entry.response ? { ...entry.response } : null;
    } catch (e) {
      throw e;
    }
  },
};

// history domain
export const history = {
  get entries() {
    return _history;
  },

  // Fetch all latest history
  async refresh(): Promise<HistoryEntrySummary[]> {
    _history = await invoke<HistoryEntrySummary[]>("get_history");
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
    _history = await invoke<HistoryEntrySummary[]>("get_history");
  },
};

// project domain
export const project = {
  get path() {
    return _projectPath;
  },

  get name() {
    return _projectName;
  },

  get tree() {
    return _projectTree;
  },

  async open(folderPath: string) {
    _projectPath = folderPath;
    _projectName = folderPath.split(/[/\\]/).pop() || folderPath;
    _projectTree = await invoke<ProjectNode[]>("open_project_folder", {
      path: folderPath,
      recursive: true,
    });
  },

  applyNode(parentPath: string, newChildren: ProjectNode[]) {
    if (!_projectPath) return;
    _projectTree = mergeChildren(
      _projectTree,
      parentPath === _projectPath ? null : parentPath,
      newChildren,
    );
  },

  close() {
    _projectPath = null;
    _projectName = null;
    _projectTree = [];
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
