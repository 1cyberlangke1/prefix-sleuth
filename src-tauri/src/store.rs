use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tokio::sync::oneshot;

use crate::models::{CacheInfo, ProxyConfig, RequestRecord, RequestSummary};

/// 应用全局状态，同时被 Tauri 命令和代理服务器持有
/// 内部用 Arc<RwLock<>> 实现线程安全，Clone 只复制 Arc 指针
#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<std::sync::RwLock<ProxyConfig>>,
    /// 请求记录队列，最新在前，最多保留 1000 条
    pub records: Arc<std::sync::RwLock<VecDeque<RequestRecord>>>,
    /// 代理服务器是否正在运行
    pub proxy_running: Arc<AtomicBool>,
    /// 关闭服务器的信号发送端
    pub shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(std::sync::RwLock::new(ProxyConfig::default())),
            records: Arc::new(std::sync::RwLock::new(VecDeque::with_capacity(1000))),
            proxy_running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// 插入新请求记录到头（最新优先），超出 1000 条则淘汰末尾
    pub fn push_record(&self, record: RequestRecord) {
        let mut records = self.records.write().unwrap();
        if records.len() >= 1000 {
            records.pop_back();
        }
        records.push_front(record);
    }

    /// 流式请求完成后，用重建的完整响应 body 更新已有记录
    pub fn update_response(
        &self,
        id: &str,
        status: u16,
        body: String,
        duration_ms: u64,
        cache_info: CacheInfo,
    ) {
        let mut records = self.records.write().unwrap();
        if let Some(record) = records.iter_mut().find(|r| r.id == id) {
            record.response_status = Some(status);
            record.response_body = Some(body);
            record.duration_ms = duration_ms;
            record.cache_info = cache_info;
        }
    }

    /// 返回所有记录的摘要列表
    pub fn get_summaries(&self) -> Vec<RequestSummary> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .map(|r| {
                let preview = r.request_body.chars().take(120).collect::<String>();
                RequestSummary {
                    id: r.id.clone(),
                    timestamp: r.timestamp.clone(),
                    api_key_label: r.api_key_label.clone(),
                    method: r.method.clone(),
                    path: r.path.clone(),
                    model: r.model.clone(),
                    response_status: r.response_status,
                    request_preview: preview,
                    cache_hit_rate: r.cache_info.cache_hit_rate,
                }
            })
            .collect()
    }

    /// 按 ID 查找完整请求记录
    pub fn get_record(&self, id: &str) -> Option<RequestRecord> {
        let records = self.records.read().unwrap();
        records.iter().find(|r| r.id == id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CacheInfo;

    fn make_record(id: &str, label: &str) -> RequestRecord {
        RequestRecord {
            id: id.into(),
            timestamp: "12:00:00.000".into(),
            api_key_label: label.into(),
            method: "POST".into(),
            path: "/v1/chat/completions".into(),
            model: Some("deepseek-chat".into()),
            request_body: r#"{"messages":[{"role":"user","content":"hi"}]}"#.into(),
            response_body: None,
            response_status: None,
            duration_ms: 0,
            cache_info: CacheInfo::default(),
        }
    }

    #[test]
    fn test_push_and_get_summaries() {
        let state = ProxyState::new();
        state.push_record(make_record("1", "app-a"));
        state.push_record(make_record("2", "app-b"));
        let summaries = state.get_summaries();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, "2");
        assert_eq!(summaries[1].id, "1");
    }

    #[test]
    fn test_update_response() {
        let state = ProxyState::new();
        state.push_record(make_record("1", "test"));

        let cache_info = CacheInfo {
            prompt_cache_hit_tokens: Some(100),
            prompt_cache_miss_tokens: Some(50),
            cache_hit_rate: Some(2.0 / 3.0),
        };
        state.update_response("1", 200, r#"{"usage":{}}"#.into(), 123, cache_info.clone());

        let record = state.get_record("1").unwrap();
        assert_eq!(record.response_status, Some(200));
        assert_eq!(record.duration_ms, 123);
        assert_eq!(record.cache_info.prompt_cache_hit_tokens, Some(100));
    }

    #[test]
    fn test_max_records() {
        let state = ProxyState::new();
        for i in 0..1001 {
            state.push_record(make_record(&i.to_string(), "x"));
        }
        assert_eq!(state.get_summaries().len(), 1000);
        assert!(state.get_record("0").is_none());
    }

    #[test]
    fn test_empty_state() {
        let state = ProxyState::new();
        assert!(state.get_summaries().is_empty());
        assert!(state.get_record("nonexistent").is_none());
    }
}
