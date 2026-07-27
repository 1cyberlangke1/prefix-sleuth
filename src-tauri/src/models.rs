use serde::{Deserialize, Serialize};

/// 代理配置，用户可在设置页修改
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 上游 API 地址，如 https://api.deepseek.com
    pub upstream_base: String,
    /// 本地代理监听端口，默认 9527
    pub local_port: u16,
    /// 允许的 API Key 白名单，空则全部放行
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

/// 一条完整的请求-响应记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub id: String,
    /// 时间戳（时分秒.毫秒）
    pub timestamp: String,
    /// 脱敏后的 API Key 标签，如 sk-****abcd
    pub api_key_label: String,
    pub method: String,
    pub path: String,
    /// 请求中的 model 字段（仅 chat/completions 存在）
    pub model: Option<String>,
    /// 原始请求体 JSON
    pub request_body: String,
    /// 原始响应体 JSON（流式完成后重建）
    pub response_body: Option<String>,
    /// HTTP 响应状态码
    pub response_status: Option<u16>,
    /// 耗时（毫秒）
    pub duration_ms: u64,
}

/// 请求列表的摘要信息（不含完整 body）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSummary {
    pub id: String,
    pub timestamp: String,
    pub api_key_label: String,
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub response_status: Option<u16>,
    /// 请求体前 120 字符的预览
    pub request_preview: String,
}

/// 相邻两条请求的 prompt diff 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    /// 前一条请求（较早的）
    pub left_id: String,
    /// 当前请求（较新的）
    pub right_id: String,
    /// left 的 messages 数组（格式化 JSON）
    pub left_messages: String,
    /// right 的 messages 数组（格式化 JSON）
    pub right_messages: String,
    /// similar crate 生成的 unified diff 文本
    pub diff_text: String,
}

/// 脱敏 API Key：保留前缀 + 后 4 位，中间变星号
/// 输入 sk-abcdefghijkl → 输出 sk-a****jkl
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

/// 从请求体 JSON 中提取 model 字段（如 "deepseek-chat"）
pub fn extract_model(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("model").and_then(|m| m.as_str()).map(String::from)
}

/// 从请求体 JSON 中提取 messages 数组并格式化为漂亮 JSON
/// 如果没有 messages 字段则格式化整个 body
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
