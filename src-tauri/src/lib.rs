mod models;
mod proxy;
mod store;

use tauri::Emitter;
use tauri::Manager;

use models::{DiffResult, ProxyConfig, RequestRecord, RequestSummary};
use store::ProxyState;

#[tauri::command]
fn get_config(state: tauri::State<'_, ProxyState>) -> ProxyConfig {
    state.config.read().unwrap().clone()
}

#[tauri::command]
fn update_config(
    state: tauri::State<'_, ProxyState>,
    app_handle: tauri::AppHandle,
    config: ProxyConfig,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write().unwrap();
        *cfg = config;
    }
    app_handle.emit("config-changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_requests(
    state: tauri::State<'_, ProxyState>,
    api_key_filter: Option<String>,
) -> Vec<RequestSummary> {
    let mut summaries = state.get_summaries();
    if let Some(ref filter) = api_key_filter {
        if !filter.is_empty() {
            summaries.retain(|s| s.api_key_label.contains(filter));
        }
    }
    summaries
}

#[tauri::command]
fn get_request_detail(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Option<RequestRecord> {
    state.get_record(&id)
}

#[tauri::command]
fn get_diff(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Option<DiffResult> {
    let records = state.records.read().unwrap();
    let idx = records.iter().position(|r| r.id == id)?;

    if idx + 1 >= records.len() {
        return None;
    }

    let right = &records[idx];
    let left = &records[idx + 1];

    let left_messages = models::extract_messages(&left.request_body);
    let right_messages = models::extract_messages(&right.request_body);

    let diff_text = diff_text(&left_messages, &right_messages);

    Some(DiffResult {
        left_id: left.id.clone(),
        right_id: right.id.clone(),
        left_messages,
        right_messages,
        diff_text,
    })
}

fn diff_text(left: &str, right: &str) -> String {
    let mut result = String::new();
    for change in similar::TextDiff::from_lines(left, right).iter_all_changes() {
        let tag = match change.tag() {
            similar::ChangeTag::Equal => " ",
            similar::ChangeTag::Delete => "-",
            similar::ChangeTag::Insert => "+",
        };
        for line in change.value().lines() {
            result.push_str(&format!("{} {}\n", tag, line));
        }
    }
    result
}

#[tauri::command]
fn get_api_keys(state: tauri::State<'_, ProxyState>) -> Vec<String> {
    let records = state.records.read().unwrap();
    let mut keys: Vec<String> = records
        .iter()
        .map(|r| r.api_key_label.clone())
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let proxy_state = ProxyState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(proxy_state.clone())
        .setup(|app| {
            let state: tauri::State<'_, ProxyState> = app.state();
            let state_clone = state.inner().clone();
            tauri::async_runtime::spawn(async move {
                proxy::start_server(state_clone).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            get_requests,
            get_request_detail,
            get_diff,
            get_api_keys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
