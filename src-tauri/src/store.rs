use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

use crate::models::{ProxyConfig, RequestRecord, RequestSummary};

#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<RwLock<ProxyConfig>>,
    pub records: Arc<RwLock<VecDeque<RequestRecord>>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(ProxyConfig::default())),
            records: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
        }
    }

    pub fn push_record(&self, record: RequestRecord) {
        let mut records = self.records.write().unwrap();
        if records.len() >= 1000 {
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
    ) {
        let mut records = self.records.write().unwrap();
        if let Some(record) = records.iter_mut().find(|r| r.id == id) {
            record.response_status = Some(status);
            record.response_body = Some(body);
            record.duration_ms = duration_ms;
        }
    }

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

    pub fn get_record(&self, id: &str) -> Option<RequestRecord> {
        let records = self.records.read().unwrap();
        records.iter().find(|r| r.id == id).cloned()
    }
}
