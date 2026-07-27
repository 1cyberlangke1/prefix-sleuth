use std::sync::Arc;

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

use crate::models::{extract_model, mask_api_key, RequestRecord};
use crate::store::ProxyState;

/// 统一响应 body 类型：UnsyncBoxBody<Bytes, Box<dyn Error>>
/// 支持从 Full（静态数据）、StreamBody（流式）等构造
type DynBody = http_body_util::combinators::UnsyncBoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>;

/// 转发时需移除的 HTTP hop-by-hop headers
/// 这些 header 是点对点的，不能透传
const HOP_BY_HOP: &[&str] = &[
    "connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
    "te", "trailers", "transfer-encoding", "upgrade",
];

/// 移除 hop-by-hop headers + host + content-length，防止干扰上游
/// 输入：原始请求 headers → 输出：可转发给上游的 headers
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

/// 从 Authorization header 中提取 API Key 并脱敏显示
/// 如 "Bearer sk-abcdefgh" → "sk-a****bcdefgh"
fn extract_api_key_label(headers: &hyper::HeaderMap) -> String {
    if let Some(val) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(key) = val.strip_prefix("Bearer ").or(val.strip_prefix("bearer ")) {
            return mask_api_key(key.trim());
        }
        mask_api_key(val)
    } else {
        "<no-auth>".into()
    }
}

/// 收集 hyper Incoming body 为 Vec<u8>（输入超时无保护，简单实现）
async fn collect_body(body: Incoming) -> Vec<u8> {
    body.collect().await
        .map(|c| c.to_bytes().to_vec())
        .unwrap_or_default()
}

/// 将静态数据装箱为 DynBody，方便统一返回类型
fn boxed_full(data: impl Into<Bytes>) -> DynBody {
    Full::new(data.into())
        .map_err(|e: std::convert::Infallible| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed_unsync()
}

/// 快速构造一个纯文本错误响应（用于 502 等场景）
fn err_response(status: u16, msg: String) -> hyper::Response<DynBody> {
    hyper::Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(boxed_full(msg))
        .unwrap()
}

/// 向上游发送真正的 HTTP 请求，透传 method/headers/body
/// 输入：上游 URL、原始 method、过滤后的 headers、body、本地端口
/// 输出：上游 HTTP 响应（状态码 + headers + reqwest Response 流）
async fn proxy_request(
    upstream_url: &str,
    method: hyper::Method,
    headers: hyper::HeaderMap,
    body: Vec<u8>,
    local_port: u16,
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
        .header("host", format!("localhost:{}", local_port))
        .body(body)
        .send()
        .await?;

    let status = resp.status();
    let headers = resp.headers().clone();
    Ok((status, headers, resp))
}

/// 用状态码 + headers + body 构造标准 hyper Response
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

/// 核心请求处理器：记录 → 转发 → 双写流式/非流式 → 返回响应
/// 输入：共享状态 + 客户端 HTTP 请求
/// 输出：转发后的 HTTP 响应（所有错误已吞掉作为 502）
async fn handle_request(
    state: &ProxyState,
    req: hyper::Request<Incoming>,
) -> hyper::Response<DynBody> {
    let method = req.method().clone();
    let path = req.uri().path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    let req_headers = req.headers().clone();
    let api_key_label = extract_api_key_label(&req_headers);
    let is_chat = path.contains("/chat/completions") && method == hyper::Method::POST;

    let (upstream_url, port) = {
        let config = state.config.read().unwrap();
        (
            format!("{}{}", config.upstream_base.trim_end_matches('/'), path),
            config.local_port,
        )
    };

    let body_bytes = collect_body(req.into_body()).await;

    // 只有 POST /v1/chat/completions 才记录请求+响应，其他路径透传但不记录
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
            response_body: None,
            response_status: None,
            duration_ms: 0,
        };
        let rid = record.id.clone();
        state.push_record(record);
        Some(rid)
    } else {
        None
    };

    let upstream_resp = match proxy_request(&upstream_url, method, req_headers, body_bytes, port).await {
        Ok(r) => r,
        Err(e) => return err_response(502, format!("Upstream error: {}", e)),
    };

    let (status, resp_headers, reqwest_resp) = upstream_resp;

    let is_stream = resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream") || ct.contains("application/x-ndjson"))
        .unwrap_or(false);

    let resp = match record_id {
        // 需要记录响应：流式双写 or 非流式直接存
        Some(rid) => {
            let recorded_body = Arc::new(std::sync::Mutex::new(Vec::new()));
            let start = std::time::Instant::now();

            if is_stream {
                // 流式双写：mpsc channel 一边推给客户端，一边 append Vec<u8> 重建完整 body
                let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(128);
                let recorded = recorded_body.clone();
                let rid2 = rid.clone();
                let state2 = state.clone();

                tokio::spawn(async move {
                    let mut stream = reqwest_resp.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                recorded.lock().unwrap().extend_from_slice(&bytes);
                                if tx.send(bytes).await.is_err() { break; }
                            }
                            Err(_) => break,
                        }
                    }
                    drop(tx);
                    let elapsed = start.elapsed().as_millis() as u64;
                    let full = String::from_utf8_lossy(&recorded.lock().unwrap()).to_string();
                    state2.update_response(&rid2, status.as_u16(), full, elapsed);
                });

                let frame_stream = tokio_stream::wrappers::ReceiverStream::new(rx)
                    .map(Frame::data)
                    .map(Ok::<_, Box<dyn std::error::Error + Send + Sync>>);
                let body = StreamBody::new(frame_stream).boxed_unsync();
                build_response(status, resp_headers, body)
            } else {
                // 非流式：等完整 body 再存
                let resp_body = match reqwest_resp.bytes().await {
                    Ok(b) => b,
                    Err(_) => Bytes::new(),
                };
                let elapsed = start.elapsed().as_millis() as u64;
                let body_str = String::from_utf8_lossy(&resp_body).to_string();
                state.update_response(&rid, status.as_u16(), body_str, elapsed);

                build_response(status, resp_headers, boxed_full(resp_body))
            }
        }
        // 非 chat 路径：透传但不记录
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
    };

    resp
}

/// 启动代理服务器，监听 localhost:port，接受 HTTP 1.1 连接
/// 输入：共享 ProxyState（内含 config + records）
/// 输出：oneshot::Sender<()>，调用 send() 即可关闭服务器
pub async fn start_server(state: ProxyState) -> oneshot::Sender<()> {
    let port = {
        let config = state.config.read().unwrap();
        config.local_port
    };

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .expect("Failed to bind proxy port");

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let state = state.clone();
                // 每连接一个独立 task，互不阻塞
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req: hyper::Request<Incoming>| {
                        let state = state.clone();
                        async move {
                            // 使用 Infallible 避免 Box<dyn Error> 生命周期问题
                            Ok::<_, std::convert::Infallible>(handle_request(&state, req).await)
                        }
                    });
                    let _ = http1::Builder::new()
                        .serve_connection(io, svc)
                        .await;
                });
            }
        }
    });

    let (tx, _) = oneshot::channel::<()>();
    tx
}
