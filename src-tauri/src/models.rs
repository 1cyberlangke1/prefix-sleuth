use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub upstream_base: String,
    pub local_port: u16,
    pub allowed_keys: Vec<String>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            upstream_base: "https://api.deepseek.com".into(),
            local_port: 9527,
            allowed_keys: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub id: String,
    pub timestamp: String,
    pub api_key_label: String,
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub request_body: String,
    pub response_body: Option<String>,
    pub response_status: Option<u16>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSummary {
    pub id: String,
    pub timestamp: String,
    pub api_key_label: String,
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub response_status: Option<u16>,
    pub request_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub left_id: String,
    pub right_id: String,
    pub left_messages: String,
    pub right_messages: String,
    pub diff_text: String,
}

pub fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        let prefix = &key[..key.len().saturating_sub(4)];
        let suffix = &key[key.len().saturating_sub(4)..];
        format!("{}****{}", &prefix[..4.min(prefix.len())], suffix)
    } else if !key.is_empty() {
        "****".into()
    } else {
        "<no-key>".into()
    }
}

pub fn extract_model(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("model").and_then(|m| m.as_str()).map(String::from)
}

pub fn extract_messages(body: &str) -> String {
    let val = serde_json::from_str::<serde_json::Value>(body);
    match val {
        Ok(ref v) => {
            if let Some(msgs) = v.get("messages") {
                serde_json::to_string_pretty(msgs).unwrap_or_else(|_| body.to_string())
            } else {
                serde_json::to_string_pretty(v).unwrap_or_else(|_| body.to_string())
            }
        }
        Err(_) => body.to_string(),
    }
}
