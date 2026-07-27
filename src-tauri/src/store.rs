use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::oneshot;

use crate::models::{CacheInfo, ProxyConfig, RequestRecord, RequestSummary};

const MAX_RECORDS: usize = 1000;

/// 应用全局状态，同时被 Tauri 命令和代理服务器持有
#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<RwLock<ProxyConfig>>,
    pub records: Arc<RwLock<VecDeque<RequestRecord>>>,
    pub proxy_running: Arc<AtomicBool>,
    pub shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    data_dir: Arc<Option<PathBuf>>,
}

impl ProxyState {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let state = Self {
            config: Arc::new(RwLock::new(ProxyConfig::default())),
            records: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_RECORDS))),
            proxy_running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            data_dir: Arc::new(data_dir),
        };
        state.load_history();
        state
    }

    fn log_path(&self) -> Option<PathBuf> {
        self.data_dir.as_ref().as_ref().map(|d| d.join("requests.jsonl"))
    }

    fn config_path(&self) -> Option<PathBuf> {
        self.data_dir.as_ref().as_ref().map(|d| d.join("config.json"))
    }

    /// 从 JSONL 文件加载历史记录
    fn load_history(&self) {
        let path = match self.log_path() {
            Some(p) => p,
            None => return,
        };
        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut reader = std::io::BufReader::new(file);
        let mut all: Vec<RequestRecord> = Vec::new();
        while let Ok(record) = jsonl::read::<_, RequestRecord>(&mut reader) {
            all.push(record);
        }
        // 按 ID 去重（保留最后出现的版本，即最新更新）
        let mut seen = HashSet::new();
        let mut records = self.records.write().unwrap();
        for record in all.into_iter().rev() {
            if seen.contains(&record.id) {
                continue;
            }
            seen.insert(record.id.clone());
            if records.len() >= MAX_RECORDS {
                break;
            }
            records.push_back(record);
        }
    }

    /// 追加一条记录到 JSONL 文件
    fn append_to_log(&self, record: &RequestRecord) {
        let path = match self.log_path() {
            Some(p) => p,
            None => return,
        };
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = jsonl::write(&mut file, record);
        }
    }

    /// 保存配置到文件
    pub fn save_config(&self) {
        let path = match self.config_path() {
            Some(p) => p,
            None => return,
        };
        let config = self.config.read().unwrap();
        if let Ok(json) = serde_json::to_string_pretty(&*config) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// 从文件加载配置
    pub fn load_config(&self) {
        let path = match self.config_path() {
            Some(p) => p,
            None => return,
        };
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<ProxyConfig>(&content) {
                let mut cfg = self.config.write().unwrap();
                *cfg = config;
            }
        }
    }

    pub fn push_record(&self, record: RequestRecord) {
        self.append_to_log(&record);
        let mut records = self.records.write().unwrap();
        if records.len() >= MAX_RECORDS {
            records.pop_back();
        }
        records.push_front(record);
    }

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
            // 同时追加更新后的记录到日志
            self.append_to_log(record);
        }
    }

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
        let state = ProxyState::new(None);
        state.push_record(make_record("1", "app-a"));
        state.push_record(make_record("2", "app-b"));
        let summaries = state.get_summaries();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, "2");
        assert_eq!(summaries[1].id, "1");
    }

    #[test]
    fn test_update_response() {
        let state = ProxyState::new(None);
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
        let state = ProxyState::new(None);
        for i in 0..1001 {
            state.push_record(make_record(&i.to_string(), "x"));
        }
        assert_eq!(state.get_summaries().len(), 1000);
        assert!(state.get_record("0").is_none());
    }

    #[test]
    fn test_empty_state() {
        let state = ProxyState::new(None);
        assert!(state.get_summaries().is_empty());
        assert!(state.get_record("nonexistent").is_none());
    }
}
