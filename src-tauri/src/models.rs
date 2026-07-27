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

/// DeepSeek 响应中的缓存用量（位于 usage 对象内）
/// 参考：https://api-docs.deepseek.com/guides/kv_cache/
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheInfo {
    /// 命中 cache 的 prompt token 数
    pub prompt_cache_hit_tokens: Option<u64>,
    /// 未命中 cache 的 prompt token 数
    pub prompt_cache_miss_tokens: Option<u64>,
    /// 缓存命中率 = hit / (hit + miss)
    pub cache_hit_rate: Option<f64>,
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
    /// DeepSeek 缓存命中/未命中 token 数
    pub cache_info: CacheInfo,
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
    /// 缓存命中率（0.0 ~ 1.0），无数据时为 None
    pub cache_hit_rate: Option<f64>,
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

/// 从响应体 JSON 中解析 usage.prompt_cache_hit/miss_tokens
/// 返回 CacheInfo（含计算好的命中率）
pub fn extract_cache_info(body: &str) -> CacheInfo {
    let val = serde_json::from_str::<serde_json::Value>(body);
    match val {
        Ok(ref v) => {
            if let Some(usage) = v.get("usage") {
                let hit = usage.get("prompt_cache_hit_tokens").and_then(|v| v.as_u64());
                let miss = usage.get("prompt_cache_miss_tokens").and_then(|v| v.as_u64());
                let rate = match (hit, miss) {
                    (Some(h), Some(m)) if h + m > 0 => Some(h as f64 / (h + m) as f64),
                    _ => None,
                };
                CacheInfo {
                    prompt_cache_hit_tokens: hit,
                    prompt_cache_miss_tokens: miss,
                    cache_hit_rate: rate,
                }
            } else {
                CacheInfo::default()
            }
        }
        Err(_) => CacheInfo::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key_long() {
        assert_eq!(mask_api_key("sk-abcdefghijkl"), "sk-a****ijkl");
    }

    #[test]
    fn test_mask_api_key_short() {
        assert_eq!(mask_api_key("abc"), "****");
    }

    #[test]
    fn test_mask_api_key_empty() {
        assert_eq!(mask_api_key(""), "<no-key>");
    }

    #[test]
    fn test_extract_model_normal() {
        let body = r#"{"model":"deepseek-chat","messages":[{"role":"user","content":"hi"}]}"#;
        assert_eq!(extract_model(body), Some("deepseek-chat".into()));
    }

    #[test]
    fn test_extract_model_no_model() {
        let body = r#"{"messages":[{"role":"user","content":"hi"}]}"#;
        assert_eq!(extract_model(body), None);
    }

    #[test]
    fn test_extract_model_invalid_json() {
        assert_eq!(extract_model("not json"), None);
    }

    #[test]
    fn test_extract_messages_normal() {
        let body = r#"{"messages":[{"role":"user","content":"hi"}]}"#;
        let result = extract_messages(body);
        assert!(result.contains("user"));
        assert!(result.contains("hi"));
    }

    #[test]
    fn test_extract_messages_invalid() {
        assert_eq!(extract_messages("not json"), "not json");
    }

    #[test]
    fn test_extract_cache_info_full_hit() {
        let body = r#"{"usage":{"prompt_cache_hit_tokens":900,"prompt_cache_miss_tokens":100}}"#;
        let info = extract_cache_info(body);
        assert_eq!(info.prompt_cache_hit_tokens, Some(900));
        assert_eq!(info.prompt_cache_miss_tokens, Some(100));
        let rate = info.cache_hit_rate.unwrap();
        assert!((rate - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_extract_cache_info_no_usage() {
        let body = r#"{"id":"123"}"#;
        let info = extract_cache_info(body);
        assert!(info.prompt_cache_hit_tokens.is_none());
        assert!(info.cache_hit_rate.is_none());
    }

    #[test]
    fn test_extract_cache_info_zero_total() {
        let body = r#"{"usage":{"prompt_cache_hit_tokens":0,"prompt_cache_miss_tokens":0}}"#;
        let info = extract_cache_info(body);
        assert!(info.cache_hit_rate.is_none());
    }

    #[test]
    fn test_extract_cache_info_invalid_body() {
        let info = extract_cache_info("not json");
        assert!(info.prompt_cache_hit_tokens.is_none());
    }
}
