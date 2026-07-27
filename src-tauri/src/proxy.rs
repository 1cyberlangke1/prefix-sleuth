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

async fn collect_body(body: Incoming) -> Vec<u8> {
    body.collect().await
        .map(|c| c.to_bytes().to_vec())
        .unwrap_or_default()
}

fn boxed_full(data: impl Into<Bytes>) -> DynBody {
    Full::new(data.into())
        .map_err(|e: std::convert::Infallible| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed_unsync()
}

fn err_response(status: u16, msg: String) -> hyper::Response<DynBody> {
    hyper::Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(boxed_full(msg))
        .unwrap()
}

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
        Some(rid) => {
            let recorded_body = Arc::new(std::sync::Mutex::new(Vec::new()));
            let start = std::time::Instant::now();

            if is_stream {
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
                    .map(|bytes| Ok::<_, Box<dyn std::error::Error + Send + Sync>>(Frame::data(bytes)));
                let body = StreamBody::new(frame_stream).boxed_unsync();
                build_response(status, resp_headers, body)
            } else {
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
    });

    let (tx, _) = oneshot::channel::<()>();
    tx
}
