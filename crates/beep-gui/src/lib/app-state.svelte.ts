import { invoke } from "@tauri-apps/api/core";
import type {
  AppConstants,
  HttpRequest,
  HistoryEntry,
  HistoryEntrySummary,
  ParsedRequest,
  ParsedFileVariable,
  ParsedHttpFileResult,
  ProjectNode,
  RequestResult,
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
let _result = $state<RequestResult | null>(null);
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

  // Full result (request echo + response), null before first send.
  get result() {
    return _result;
  },

  // Convenience: just the response part, for code that only needs response data.
  get response() {
    return _result?.response ?? null;
  },

  async send(req: HttpRequest): Promise<RequestResult> {
    try {
      const res = await invoke<RequestResult>("execute_request", {
        payload: req,
      });
      _result = res;
      history.refresh().catch(() => {});
      return res;
    } catch (e) {
      _result = null;
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
    _result = null;
  },

  // Populate the form from a history entry.
  async loadFromHistory(summary: HistoryEntrySummary) {
    try {
      const entry = await invoke<HistoryEntry>("get_history_entry", {
        id: summary.id,
      });
      _request = { ...entry.request };
      _result = entry.result ? { ...entry.result } : null;
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
let _loadingDirs = $state<Set<string>>(new Set());

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

  get loadingDirs() {
    return _loadingDirs;
  },

  async open(folderPath: string) {
    _projectPath = folderPath;
    _projectName = folderPath.split(/[/\\]/).pop() || folderPath;
    _projectTree = await invoke<ProjectNode[]>("read_dir_children", {
      path: folderPath,
    });
  },

  // expand folders and Lazy-load children of a directory
  async expand(dirPath: string) {
    _loadingDirs.add(dirPath);
    try {
      const children = await invoke<ProjectNode[]>("read_dir_children", {
        path: dirPath,
      });
      project.applyNode(dirPath, children);
    } finally {
      _loadingDirs.delete(dirPath);
    }
  },

  // Merge children from a watcher event into the tree.
  applyNode(parentPath: string, newChildren: ProjectNode[]) {
    if (!_projectPath) return;
    _projectTree = mergeChildren(
      _projectTree,
      parentPath === _projectPath ? null : parentPath,
      newChildren,
    );
  },

  // Check if a directory has been lazy-loaded.
  isLoaded(dirPath: string): boolean {
    function findNode(nodes: ProjectNode[], path: string): ProjectNode | null {
      for (const n of nodes) {
        if (n.path === path) return n;
        if (n.children) {
          const found = findNode(n.children, path);
          if (found) return found;
        }
      }
      return null;
    }
    const node = findNode(_projectTree, dirPath);
    return node ? node.children !== undefined : false;
  },

  close() {
    _projectPath = null;
    _projectName = null;
    _projectTree = [];
    _loadingDirs.clear();
  },
};

// http-file domain - parse, serialize, update .http file content
export const httpFile = {
  async parse(content: string): Promise<ParsedHttpFileResult> {
    return invoke<ParsedHttpFileResult>("http_parse", { content });
  },

  async updateVars(
    content: string,
    variables: ParsedFileVariable[],
  ): Promise<string> {
    return invoke<string>("http_update_vars", { content, variables });
  },

  async updateRequest(
    content: string,
    requestIdx: number,
    updated: ParsedRequest,
  ): Promise<string> {
    return invoke<string>("http_update_req", { content, requestIdx, updated });
  },

  async appendRequest(
    content: string,
    newRequest: ParsedRequest,
  ): Promise<string> {
    return invoke<string>("http_append_req", { content, newRequest });
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

  // Platform-aware modifier label: "⌘" on macOS, "Ctrl" otherwise.
  // TODO real test on Mac... I don't have one.
  get modKey(): string {
    return _constants?.platform === "macos" ? "⌘" : "Ctrl";
  },
};
