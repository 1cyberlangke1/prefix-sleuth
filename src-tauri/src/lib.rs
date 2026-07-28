#![cfg_attr(not(feature = "desktop"), allow(dead_code))]

mod models;
#[cfg(feature = "desktop")]
mod proxy;
mod store;

#[cfg(feature = "desktop")]
use models::{DiffResult, ProxyConfig, RequestRecord, RequestSummary};
#[cfg(feature = "desktop")]
use store::ProxyState;

#[cfg(feature = "desktop")]
use std::sync::atomic::Ordering;
#[cfg(feature = "desktop")]
use tauri::{Emitter, Manager};
#[cfg(feature = "desktop")]
use tokio::sync::oneshot;

#[cfg(feature = "desktop")]
#[tauri::command]
fn get_config(state: tauri::State<'_, ProxyState>) -> ProxyConfig {
    state.config.read().unwrap().clone()
}

#[cfg(feature = "desktop")]
async fn do_start_proxy(state: &ProxyState, app_handle: &tauri::AppHandle) -> Result<(), String> {
    if state.proxy_running.load(Ordering::SeqCst) {
        return Err("代理已在运行中".into());
    }

    let port = state.config.read().map_err(|e| e.to_string())?.local_port;
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("绑定端口 {} 失败: {}", port, e))?;

    let (tx, rx) = oneshot::channel::<()>();
    {
        let mut shutdown = state.shutdown_tx.lock().map_err(|e| e.to_string())?;
        *shutdown = Some(tx);
    }
    state.proxy_running.store(true, Ordering::SeqCst);
    let _ = app_handle.emit("proxy-status-changed", true);

    let state_clone = state.clone();
    tauri::async_runtime::spawn(async move {
        proxy::start_server(state_clone, listener, rx).await;
    });

    Ok(())
}

#[cfg(feature = "desktop")]
fn do_stop_proxy(state: &ProxyState, app_handle: &tauri::AppHandle) -> Result<(), String> {
    if !state.proxy_running.load(Ordering::SeqCst) {
        return Ok(());
    }

    let mut shutdown = state.shutdown_tx.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = shutdown.take() {
        let _ = tx.send(());
    }
    state.proxy_running.store(false, Ordering::SeqCst);
    let _ = app_handle.emit("proxy-status-changed", false);
    Ok(())
}

#[cfg(feature = "desktop")]
#[tauri::command]
async fn update_config(
    state: tauri::State<'_, ProxyState>,
    app_handle: tauri::AppHandle,
    config: ProxyConfig,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write().map_err(|e| e.to_string())?;
        *cfg = config;
    }
    state.save_config();
    
    let was_running = state.proxy_running.load(Ordering::SeqCst);
    if was_running {
        do_stop_proxy(&state, &app_handle)?;
        do_start_proxy(&state, &app_handle).await?;
    }

    app_handle.emit("config-changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "desktop")]
#[tauri::command]
async fn start_proxy(
    state: tauri::State<'_, ProxyState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    do_start_proxy(&state, &app_handle).await
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn stop_proxy(
    state: tauri::State<'_, ProxyState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    do_stop_proxy(&state, &app_handle)
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn proxy_status(state: tauri::State<'_, ProxyState>) -> bool {
    state.proxy_running.load(Ordering::SeqCst)
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn get_requests(
    state: tauri::State<'_, ProxyState>,
    api_key_filter: Option<String>,
) -> Vec<RequestSummary> {
    let mut summaries = state.get_summaries();
    if let Some(ref filter) = api_key_filter {
        if !filter.is_empty() {
            summaries.retain(|s| s.api_key_label == *filter);
        }
    }
    summaries
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn get_request_detail(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Option<RequestRecord> {
    state.get_record(&id)
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn get_diff(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Option<DiffResult> {
    let records = state.records.read().unwrap();
    let idx = records.iter().position(|r| r.id == id)?;
    let current = &records[idx];

    let prev = records.iter().skip(idx + 1).find(|r| r.api_key_label == current.api_key_label)?;

    let left_messages = models::extract_messages(&prev.request_body);
    let right_messages = models::extract_messages(&current.request_body);
    let diff_text = diff_text(&left_messages, &right_messages);

    Some(DiffResult {
        left_id: prev.id.clone(),
        right_id: current.id.clone(),
        left_messages,
        right_messages,
        diff_text,
    })
}

#[cfg(feature = "desktop")]
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

#[cfg(feature = "desktop")]
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

#[cfg(feature = "desktop")]
#[tauri::command]
fn clear_requests_by_key(
    state: tauri::State<'_, ProxyState>,
    label: String,
) -> Result<(), String> {
    state.delete_by_label(&label);
    Ok(())
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn clear_all_requests(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    state.delete_all();
    Ok(())
}

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_zoom(1.3);
            }
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let data_dir = Some(exe_dir.join("data"));
            if let Some(ref dir) = data_dir {
                let _ = std::fs::create_dir_all(dir);
            }
            let proxy_state = ProxyState::new(data_dir);
            proxy_state.load_config();
            proxy_state.set_app_handle(app.handle().clone());
            app.manage(proxy_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            start_proxy,
            stop_proxy,
            proxy_status,
            get_requests,
            get_request_detail,
            get_diff,
            get_api_keys,
            clear_requests_by_key,
            clear_all_requests,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
