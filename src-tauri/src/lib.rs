mod models;
mod proxy;
mod store;

use tauri::Emitter;
use tauri::Manager;

use models::{DiffResult, ProxyConfig, RequestRecord, RequestSummary};
use store::ProxyState;

/// 读取当前代理配置（上游地址、端口、允许的 API Key 列表）
#[tauri::command]
fn get_config(state: tauri::State<'_, ProxyState>) -> ProxyConfig {
    state.config.read().unwrap().clone()
}

/// 更新代理配置并通知前端
/// 输入：新的 ProxyConfig；输出：Ok(()) 或错误信息
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

/// 获取请求列表摘要（按 API Key 可选过滤）
/// 输入：api_key_filter（可选，如 "sk-****abcd"）
/// 输出：请求摘要数组（不含完整 body，未响应时 response_status = null）
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

/// 按 ID 获取完整请求记录（含 request_body + response_body）
/// 输入：请求 ID；输出：RequestRecord 或 null
#[tauri::command]
fn get_request_detail(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Option<RequestRecord> {
    state.get_record(&id)
}

/// 计算相邻两条请求的 prompt diff
/// 取请求 ID 对应记录与它前一条记录（时间上更早的），
/// 提取 messages 数组做 text diff
/// 输入：请求 ID（较新的那条）；输出：DiffResult 或 null（不足两条时）
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

/// 用 similar crate 做行级 unified diff
/// 前缀：空格=相同，减号=删除，加号=新增
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

/// 获取所有已出现的 API Key 标签列表（去重排序），供前端下拉筛选
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
/// 初始化 ProxyState → 注册 commands → 启动代理服务器 → 显示窗口
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
