mod commands;
mod models;

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use notify::{Event, EventKind, RecursiveMode, Watcher};
use tauri::Emitter;

use beep_core::default_headers;
use beep_core::{
    ExecuteInput, ExecutionContext, HistoryEntrySummary, ParsedRequest, RequestHistory,
};

use models::{AppConstants, AppState, FsChangePayload, FsContentChangePayload, ProjectNode};

// directories excluded from tree and watcher
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "dist",
    "build",
    "__pycache__",
    "coverage",
    ".svelte-kit",
    "gen",
];

fn is_skip_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// read_dir_path_children Read immediate children of a single directory (non-recursive) by path
fn read_dir_path_children(dir: &Path) -> Vec<ProjectNode> {
    let mut nodes = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return nodes,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        // skip excluded dirs
        let is_dir = path.is_dir();
        if is_dir && is_skip_dir(&name) {
            continue;
        }

        if is_dir {
            // children: None signals "not yet loaded" to the frontend
            nodes.push(ProjectNode {
                name,
                path: path.to_string_lossy().to_string(),
                is_dir: true,
                children: None,
            });
        } else if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            if ext == "json" || ext == "http" || ext == "js" {
                nodes.push(ProjectNode {
                    name,
                    path: path.to_string_lossy().to_string(),
                    is_dir: false,
                    children: None,
                });
            }
        }
    }

    // sort: directories first, then files, alphabetically within each group
    nodes.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    nodes
}

#[tauri::command]
async fn execute_request(
    state: tauri::State<'_, AppState>,
    mut payload: ParsedRequest,
    file_vars: Vec<beep_core::ParsedFileVariable>,
    source_file_dir: Option<String>,
) -> Result<beep_core::HttpResult, String> {
    // Resolve external script file references � read file content
    if payload.pre_script_external {
        if let Some(ref path) = payload.pre_script {
            let content = read_external_script(path, source_file_dir.as_deref())?;
            payload.pre_script = Some(content);
            payload.pre_script_external = false;
        }
    }
    if payload.post_script_external {
        if let Some(ref path) = payload.post_script {
            let content = read_external_script(path, source_file_dir.as_deref())?;
            payload.post_script = Some(content);
            payload.post_script_external = false;
        }
    }

    let mut ctx = ExecutionContext::new(state.history.clone());
    ctx.file_vars = file_vars;
    ctx.source_file_dir = source_file_dir;
    beep_core::execute(ExecuteInput::Parsed(payload), &mut ctx).await
}

fn read_external_script(path: &str, base_dir: Option<&str>) -> Result<String, String> {
    let normalized = path.replace('\\', "/");
    let resolved = if normalized.starts_with("./") {
        match base_dir {
            Some(dir) => std::path::Path::new(dir).join(normalized),
            None => {
                return Err(format!(
                    "Cannot resolve relative script path '{path}' without a project or file context"
                ));
            }
        }
    } else {
        std::path::PathBuf::from(path)
    };
    std::fs::read_to_string(&resolved)
        .map_err(|e| format!("Cannot read script file '{}': {e}", resolved.display()))
}

#[tauri::command]
fn get_history(state: tauri::State<'_, AppState>) -> Vec<HistoryEntrySummary> {
    let history = state.history.lock().unwrap();
    history.get_all_summaries()
}

#[tauri::command]
fn get_history_entry(
    state: tauri::State<'_, AppState>,
    id: u64,
) -> Result<beep_core::HistoryEntry, String> {
    let history = state
        .history
        .lock()
        .map_err(|_| "Failed to access history".to_string())?;

    history
        .get_entry_by_id(id)
        .cloned()
        .ok_or("history entry not found".to_string())
}

#[tauri::command]
fn clear_history(state: tauri::State<'_, AppState>) {
    state.history.lock().unwrap().clear();
}

#[tauri::command]
fn delete_history_entry(state: tauri::State<'_, AppState>, id: u64) -> bool {
    state.history.lock().unwrap().remove_by_id(id)
}

/// read_dir_children return list children of a single directory
/// TODO for now this always Ok, we can use the Err check in the future.
#[tauri::command]
async fn read_dir_children(path: String) -> Result<Vec<ProjectNode>, String> {
    Ok(read_dir_path_children(Path::new(&path)))
}

#[tauri::command]
fn read_file_content(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))
}

#[tauri::command]
fn save_file_content(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("Failed to save file: {e}"))
}

#[tauri::command]
fn watch_project(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut guard = state.watcher.lock().map_err(|e| e.to_string())?;
    *guard = None;

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let is_modify = matches!(event.kind, EventKind::Modify(_));
            let is_structural = matches!(
                event.kind,
                EventKind::Create(_)
                    | EventKind::Remove(_)
                    | EventKind::Modify(notify::event::ModifyKind::Name(_))
            );

            if !is_modify && !is_structural {
                return;
            }

            if let Some(p) = event.paths.first() {
                if p.components()
                    .any(|c| is_skip_dir(&c.as_os_str().to_string_lossy()))
                {
                    return;
                }

                let file_path = p.to_string_lossy().to_string();

                // any Modify (Data or Name) can change file content
                if is_modify {
                    let _ = app.emit(
                        "fs-content-change",
                        FsContentChangePayload { path: file_path },
                    );
                }

                // structural changes need tree refresh
                if is_structural {
                    let parent = p.parent().map(|d| d.to_path_buf()).unwrap_or(p.clone());
                    let children = read_dir_path_children(&parent);
                    let _ = app.emit(
                        "fs-change",
                        FsChangePayload {
                            parent_path: parent.to_string_lossy().to_string(),
                            children,
                        },
                    );
                }
            }
        }
    })
    .map_err(|e| e.to_string())?;

    watcher
        .watch(Path::new(&path), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;
    *guard = Some(watcher);

    Ok(())
}

#[tauri::command]
fn unwatch_project(state: tauri::State<'_, AppState>) {
    *state.watcher.lock().unwrap() = None;
}

#[tauri::command]
fn get_app_constants() -> AppConstants {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let mut headers: Vec<(String, String)> = default_headers()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // additional header for GUI
    let additional = vec![
        (
            "Accept-Encoding".to_string(),
            "gzip, deflate, br".to_string(),
        ),
        ("Connection".to_string(), "keep-alive".to_string()),
    ];
    for (key, value) in additional {
        if let Some(existing) = headers.iter_mut().find(|(k, _)| *k == key) {
            existing.1 = value;
        } else {
            headers.push((key, value));
        }
    }

    AppConstants {
        platform: std::env::consts::OS.to_string(),
        default_headers: headers,
        version,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            history: Arc::new(Mutex::new(RequestHistory::new())),
            watcher: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_app_constants,
            //
            read_dir_children,
            read_file_content,
            save_file_content,
            watch_project,
            unwatch_project,
            //
            execute_request,
            //
            get_history,
            get_history_entry,
            clear_history,
            delete_history_entry,
            //
            commands::http::http_parse,
            commands::http::http_serialize_req,
            commands::http::http_serialize_vars,
            commands::http::http_update_vars,
            commands::http::http_update_req,
            commands::http::http_append_req,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
