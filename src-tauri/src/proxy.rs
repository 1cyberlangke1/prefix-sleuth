use bytes::Bytes;
use chrono::Utc;
use futures::StreamExt;
use http_body::Frame;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::sync::oneshot;

use crate::models::{extract_cache_info, extract_model, mask_api_key, RequestRecord};
use crate::store::ProxyState;

type DynBody = http_body_util::combinators::UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>;

const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
    "te", "trailers", "transfer-encoding", "upgrade",
];

fn strip_headers(headers: &hyper::HeaderMap) -> hyper::HeaderMap {
    let mut out = hyper::HeaderMap::new();
    for (name, value) in headers.iter() {
        let name_str = name.as_str().to_lowercase();
        if !HOP_BY_HOP.contains(&name_str.as_str())
            && name_str != "host"
            && name_str != "content-length"
        {
            out.insert(name.clone(), value.clone());
        }
    }
    out
}

fn inject_stream_options(body: &[u8]) -> Vec<u8> {
    if let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(obj) = json.as_object_mut() {
            if let Some(stream) = obj.get("stream").and_then(|v| v.as_bool()) {
                if stream {
                    let stream_options = serde_json::json!({ "include_usage": true });
                    obj.insert("stream_options".to_string(), stream_options);
                    if let Ok(new_body) = serde_json::to_vec(&json) {
                        return new_body;
                    }
                }
            }
        }
    }
    body.to_vec()
}

fn extract_bearer_token(headers: &hyper::HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|val| {
            val.strip_prefix("Bearer ")
                .or(val.strip_prefix("bearer "))
                .map(|s| s.trim().to_string())
        })
}

/// 验证下游 key 并返回（标签, 替换后的 headers）
/// 无下游配置时透传原始 key，否则校验并替换为上游 key
fn resolve_auth(
    headers: &hyper::HeaderMap,
    config: &crate::models::ProxyConfig,
) -> Result<(String, hyper::HeaderMap), Box<hyper::Response<DynBody>>> {
    if config.downstream_keys.is_empty() {
        let label = extract_bearer_token(headers)
            .as_deref()
            .map(mask_api_key)
            .unwrap_or_else(|| "<no-key>".into());
        return Ok((label, headers.clone()));
    }

    let token = extract_bearer_token(headers)
        .ok_or_else(|| Box::new(err_response(401, "Missing Authorization header".into())))?;

    let matched = config
        .downstream_keys
        .iter()
        .find(|dk| dk.key == token)
        .ok_or_else(|| Box::new(err_response(403, "Unknown downstream API key".into())))?;

    let label = matched.label.clone();
    let mut new_headers = headers.clone();
    if let Ok(auth_val) = hyper::header::HeaderValue::from_str(&format!("Bearer {}", config.upstream_api_key)) {
        new_headers.insert("authorization", auth_val);
    }

    Ok((label, new_headers))
}

fn err_response(status: u16, msg: String) -> hyper::Response<DynBody> {
    hyper::Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(boxed_full(msg))
        .unwrap()
}

fn boxed_full(data: impl Into<Bytes>) -> DynBody {
    Full::new(data.into())
        .map_err(|e: std::convert::Infallible| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed_unsync()
}

async fn collect_body(body: Incoming) -> Vec<u8> {
    body.collect().await
        .map(|c| c.to_bytes().to_vec())
        .unwrap_or_default()
}

fn build_response(
    status: hyper::StatusCode,
    headers: hyper::HeaderMap,
    body: DynBody,
) -> hyper::Response<DynBody> {
    let mut resp = hyper::Response::new(body);
    *resp.status_mut() = status;
    resp.headers_mut().extend(headers.iter().map(|(k, v)| (k.clone(), v.clone())));
    resp
}

async fn proxy_to_upstream(
    upstream_url: &str,
    method: hyper::Method,
    headers: hyper::HeaderMap,
    body: Vec<u8>,
) -> Result<(hyper::StatusCode, hyper::HeaderMap, reqwest::Response), reqwest::Error> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap();

    let forwarded_headers = strip_headers(&headers);
    let upstream_method = reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap();

    let resp = client
        .request(upstream_method, upstream_url)
        .headers(forwarded_headers)
        .body(body)
        .send()
        .await?;

    let status = resp.status();
    let rsp_headers = resp.headers().clone();
    Ok((status, rsp_headers, resp))
}

async fn handle_request(
    state: &ProxyState,
    req: hyper::Request<Incoming>,
) -> hyper::Response<DynBody> {
    let method = req.method().clone();
    let path = req.uri().path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    let req_headers = req.headers().clone();
    let is_chat = path.contains("/chat/completions") && method == hyper::Method::POST;

    // 校验下游 key 并获取标签 + 替换后的 headers
    // 用块作用域确保 RwLockReadGuard 在 .await 前释放
    let (api_key_label, final_headers, upstream_base) = {
        let config = state.config.read().unwrap();
        let (label, headers) = match resolve_auth(&req_headers, &config) {
            Ok(v) => v,
            Err(err_resp) => return *err_resp,
        };
        (label, headers, config.upstream_base.clone())
    };

    let upstream_url = format!("{}{}", upstream_base.trim_end_matches('/'), path);
    let mut body_bytes = collect_body(req.into_body()).await;

    if is_chat {
        body_bytes = inject_stream_options(&body_bytes);
    }

    let record_id = if is_chat {
        let model = extract_model(&String::from_utf8_lossy(&body_bytes));
        let record = RequestRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now().format("%H:%M:%S%.3f").to_string(),
            api_key_label: api_key_label.clone(),
            method: method.to_string(),
            path: path.clone(),
            model,
            request_body: String::from_utf8_lossy(&body_bytes).to_string(),
            request_headers: {
                let mut h = std::collections::HashMap::new();
                for (name, value) in req_headers.iter() {
                    if let Ok(v) = value.to_str() {
                        h.insert(name.as_str().to_string(), v.to_string());
                    }
                }
                h
            },
            response_body: None,
            response_status: None,
            duration_ms: 0,
            cache_info: Default::default(),
        };
        let rid = record.id.clone();
        state.push_record(record);
        Some(rid)
    } else {
        None
    };

    let upstream_resp = match proxy_to_upstream(&upstream_url, method, final_headers, body_bytes).await {
        Ok(r) => r,
        Err(e) => return err_response(502, format!("Upstream error: {}", e)),
    };

    let (status, resp_headers, reqwest_resp) = upstream_resp;

    let is_stream = resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream") || ct.contains("application/x-ndjson"))
        .unwrap_or(false);

    match record_id {
        Some(rid) => {
            let start = std::time::Instant::now();

            if is_stream {
                let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(128);
                let (record_tx, mut record_rx) = tokio::sync::mpsc::channel::<Bytes>(256);
                let rid2 = rid.clone();
                let state2 = state.clone();

                // 转发任务：纯透传，零阻塞
                tokio::spawn(async move {
                    let mut stream = reqwest_resp.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                let _ = record_tx.try_send(bytes.clone());
                                if tx.send(bytes).await.is_err() { break; }
                            }
                            Err(_) => break,
                        }
                    }
                });

                // 记录任务：异步积累，不影响转发
                tokio::spawn(async move {
                    let mut recorded = Vec::new();
                    let mut chunk_count = 0u64;
                    while let Some(bytes) = record_rx.recv().await {
                        recorded.extend_from_slice(&bytes);
                        chunk_count += 1;
                        if chunk_count % 5 == 0 {
                            let partial = String::from_utf8_lossy(&recorded).to_string();
                            let elapsed = start.elapsed().as_millis() as u64;
                            let cache_info = extract_cache_info(&partial);
                            state2.update_response(&rid2, status.as_u16(), partial, elapsed, cache_info);
                        }
                    }
                    let elapsed = start.elapsed().as_millis() as u64;
                    let full = String::from_utf8_lossy(&recorded).to_string();
                    let cache_info = extract_cache_info(&full);
                    state2.update_response(&rid2, status.as_u16(), full, elapsed, cache_info);
                });

                let frame_stream = tokio_stream::wrappers::ReceiverStream::new(rx)
                    .map(Frame::data)
                    .map(Ok::<_, Box<dyn std::error::Error + Send + Sync>>);
                let body = StreamBody::new(frame_stream).boxed_unsync();
                build_response(status, resp_headers, body)
            } else {
                let resp_body = match reqwest_resp.bytes().await {
                    Ok(b) => b,
                    Err(_) => Bytes::new(),
                };
                let elapsed = start.elapsed().as_millis() as u64;
                let body_str = String::from_utf8_lossy(&resp_body).to_string();
                let cache_info = extract_cache_info(&body_str);
                state.update_response(&rid, status.as_u16(), body_str, elapsed, cache_info);

                build_response(status, resp_headers, boxed_full(resp_body))
            }
        }
        None => {
            if is_stream {
                let frame_stream = reqwest_resp.bytes_stream()
                    .map(|r| r.map(Frame::data)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>));
                let body = StreamBody::new(frame_stream).boxed_unsync();
                build_response(status, resp_headers, body)
            } else {
                let resp_body = match reqwest_resp.bytes().await {
                    Ok(b) => b,
                    Err(_) => Bytes::new(),
                };
                build_response(status, resp_headers, boxed_full(resp_body))
            }
        }
    }
}

pub async fn start_server(state: ProxyState, listener: tokio::net::TcpListener, shutdown_rx: oneshot::Receiver<()>) {
    tokio::spawn(async move {
        let mut rx = shutdown_rx;
        loop {
            tokio::select! {
                _ = &mut rx => break,
                accept = listener.accept() => {
                    if let Ok((stream, _)) = accept {
                        let state = state.clone();
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            let svc = service_fn(move |req: hyper::Request<Incoming>| {
                                let state = state.clone();
                                async move {
                                    Ok::<_, std::convert::Infallible>(handle_request(&state, req).await)
                                }
                            });
                            let _ = http1::Builder::new()
                                .serve_connection(io, svc)
                                .await;
                        });
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_stream_options() {
        let original = br#"{"model":"deepseek-chat","messages":[],"stream":true}"#;
        let injected = inject_stream_options(original);
        let injected_str = String::from_utf8(injected).unwrap();
        assert!(injected_str.contains(r#""stream_options":{"include_usage":true}"#));

        let original_false = br#"{"model":"deepseek-chat","messages":[],"stream":false}"#;
        let injected_false = inject_stream_options(original_false);
        assert_eq!(injected_false, original_false);
    }
}
