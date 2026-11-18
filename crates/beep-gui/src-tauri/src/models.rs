use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use beep_core::{RequestHistory, VarStore};
use notify::RecommendedWatcher;
use serde::Serialize;

pub struct AppState {
    pub history: Arc<Mutex<RequestHistory>>,
    pub watcher: Mutex<Option<RecommendedWatcher>>,
    /// Session variables (`client.vars`), shared by all requests for the app lifetime.
    pub client_vars: Mutex<VarStore>,
    /// `$processEnv` names the user allowed this session.
    pub allowed_env: Mutex<HashSet<String>>,
    /// Selected environment name (`None` = Default, `.beep/env.json`).
    pub selected_env: Mutex<Option<String>>,
}

#[allow(dead_code)]
#[derive(serde::Serialize)]
pub struct AppConstants {
    pub version: String,
    pub platform: String,
    pub default_headers: Vec<(String, String)>,
    pub dynamic_vars: Vec<DynamicVarInfo>,
}

/// Dynamic variable for editor autocomplete (`$uuid`, ...).
#[derive(serde::Serialize)]
pub struct DynamicVarInfo {
    pub name: String,
    pub usage: String,
    pub description: String,
}

/// One environment for the picker: `name` None = Default; `files` in load order (empty = not created).
#[derive(Debug, Clone, Serialize)]
pub struct EnvInfo {
    pub name: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ProjectNode>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FsChangePayload {
    pub parent_path: String,
    pub children: Vec<ProjectNode>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FsContentChangePayload {
    pub path: String,
}
