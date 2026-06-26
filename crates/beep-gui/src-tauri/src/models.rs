use std::sync::Mutex;

use beep_core::{HttpClient, RequestHistory};
use notify::RecommendedWatcher;
use serde::Serialize;

pub struct AppState {
    pub client: HttpClient,
    pub history: Mutex<RequestHistory>,
    pub watcher: Mutex<Option<RecommendedWatcher>>,
}

#[allow(dead_code)]
#[derive(serde::Serialize)]
pub struct AppConstants {
    pub version: String,
    pub platform: String,
    pub default_headers: Vec<(String, String)>,
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
