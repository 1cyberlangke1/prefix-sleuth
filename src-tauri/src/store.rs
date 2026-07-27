use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

use crate::models::{ProxyConfig, RequestRecord, RequestSummary};

/// 应用全局状态，同时被 Tauri 命令和代理服务器持有
/// 内部用 Arc<RwLock<>> 实现线程安全，Clone 只复制 Arc 指针
#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<RwLock<ProxyConfig>>,
    /// 请求记录队列，最新在前，最多保留 1000 条
    pub records: Arc<RwLock<VecDeque<RequestRecord>>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(ProxyConfig::default())),
            records: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
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
    /// 输入 record.id、状态码、完整 body、耗时
    pub fn update_response(
        &self,
        id: &str,
        status: u16,
        body: String,
        duration_ms: u64,
    ) {
        let mut records = self.records.write().unwrap();
        if let Some(record) = records.iter_mut().find(|r| r.id == id) {
            record.response_status = Some(status);
            record.response_body = Some(body);
            record.duration_ms = duration_ms;
        }
    }

    /// 返回所有记录的摘要列表（不含完整 body，前端列表用）
    pub fn get_summaries(&self) -> Vec<RequestSummary> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .map(|r| {
                let preview = r
                    .request_body
                    .chars()
                    .take(120)
                    .collect::<String>();
                RequestSummary {
                    id: r.id.clone(),
                    timestamp: r.timestamp.clone(),
                    api_key_label: r.api_key_label.clone(),
                    method: r.method.clone(),
                    path: r.path.clone(),
                    model: r.model.clone(),
                    response_status: r.response_status,
                    request_preview: preview,
                }
            })
            .collect()
    }

    /// 按 ID 查找完整请求记录（前端详情页用）
    pub fn get_record(&self, id: &str) -> Option<RequestRecord> {
        let records = self.records.read().unwrap();
        records.iter().find(|r| r.id == id).cloned()
    }
}
