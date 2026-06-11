use std::sync::Mutex;

use beep_core::client::default_headers;
use beep_core::{HttpClient, HttpRequest, HttpResponse, RequestHistory};

pub struct AppState {
    pub client: HttpClient,
    pub history: Mutex<RequestHistory>,
}

#[tauri::command]
async fn execute_request(
    state: tauri::State<'_, AppState>,
    payload: HttpRequest,
) -> Result<HttpResponse, String> {
    let response = state.client.execute(&payload)?;
    state
        .history
        .lock()
        .unwrap()
        .add(payload, Some(response.clone()), None);
    Ok(response)
}

#[tauri::command]
fn get_history(state: tauri::State<'_, AppState>) -> Vec<beep_core::history::HistoryEntry> {
    let history = state.history.lock().unwrap();
    history.get_all().into_iter().cloned().collect()
}

#[tauri::command]
fn clear_history(state: tauri::State<'_, AppState>) {
    state.history.lock().unwrap().clear();
}

#[tauri::command]
fn delete_history_entry(state: tauri::State<'_, AppState>, id: u64) -> bool {
    state.history.lock().unwrap().remove_by_id(id)
}

/// Return core default headers so the GUI can display them as auto-generated headers.
#[tauri::command]
fn get_default_headers() -> Vec<(String, String)> {
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

    headers
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            client: HttpClient::new(),
            history: Mutex::new(RequestHistory::new()),
        })
        .invoke_handler(tauri::generate_handler![
            execute_request,
            get_history,
            clear_history,
            delete_history_entry,
            get_default_headers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
