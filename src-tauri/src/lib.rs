mod models;
mod proxy;
mod store;

use std::sync::atomic::Ordering;

use tauri::Emitter;
use tokio::sync::oneshot;

use models::{DiffResult, ProxyConfig, RequestRecord, RequestSummary};
use store::ProxyState;

/// 读取当前代理配置
#[tauri::command]
fn get_config(state: tauri::State<'_, ProxyState>) -> ProxyConfig {
    state.config.read().unwrap().clone()
}

/// 更新代理配置并通知前端
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

/// 启动代理服务器
#[tauri::command]
async fn start_proxy(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    if state.proxy_running.load(Ordering::SeqCst) {
        return Err("代理已在运行中".into());
    }

    let (tx, rx) = oneshot::channel::<()>();
    {
        let mut shutdown = state.shutdown_tx.lock().unwrap();
        *shutdown = Some(tx);
    }
    state.proxy_running.store(true, Ordering::SeqCst);

    let state_clone = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        proxy::start_server(state_clone, rx).await;
    });

    Ok(())
}

/// 停止代理服务器
#[tauri::command]
fn stop_proxy(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    if !state.proxy_running.load(Ordering::SeqCst) {
        return Err("代理未在运行".into());
    }

    let mut shutdown = state.shutdown_tx.lock().unwrap();
    if let Some(tx) = shutdown.take() {
        let _ = tx.send(());
    }
    state.proxy_running.store(false, Ordering::SeqCst);
    Ok(())
}

/// 获取代理运行状态
#[tauri::command]
fn proxy_status(state: tauri::State<'_, ProxyState>) -> bool {
    state.proxy_running.load(Ordering::SeqCst)
}

/// 获取请求列表摘要（按 API Key 可选过滤）
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

/// 按 ID 获取完整请求记录
#[tauri::command]
fn get_request_detail(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Option<RequestRecord> {
    state.get_record(&id)
}

/// 计算相邻两条请求的 prompt diff（按下游 key 隔离）
/// 只比较同一个 api_key_label 下的相邻请求
#[tauri::command]
fn get_diff(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Option<DiffResult> {
    let records = state.records.read().unwrap();
    let idx = records.iter().position(|r| r.id == id)?;
    let current = &records[idx];

    // 找同一 api_key_label 下前一条（时间上更早的）记录
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

/// 获取所有已出现的 API Key 标签列表（去重排序）
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

/// Tauri 应用入口点
/// 初始化状态，不自动启动代理（由前端控制）
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let proxy_state = ProxyState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(proxy_state.clone())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
