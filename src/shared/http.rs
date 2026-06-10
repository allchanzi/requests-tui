use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::blocking::Client;

use crate::shared::bruno::{Method, PreparedRequest};

/// A completed HTTP response, with the body decoded to text and JSON pretty-printed.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub time_ms: u128,
    pub size: usize,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub content_type: Option<String>,
}

/// Either a finished response or a human-readable error message.
pub type SendResult = Result<Response, String>;

/// A handle to an in-flight request running on a worker thread; poll [`Self::try_take`]
/// each event-loop tick to collect the result without blocking the UI.
pub struct SendHandle {
    rx: Receiver<SendResult>,
}

impl SendHandle {
    pub fn spawn(request: PreparedRequest) -> Self {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let result = execute(request).map_err(|error| format!("{error:#}"));
            let _ = tx.send(result);
        });
        Self { rx }
    }

    /// Non-blocking poll for the result. Returns `Some` once the request completes.
    pub fn try_take(&self) -> Option<SendResult> {
        self.rx.try_recv().ok()
    }
}

fn execute(request: PreparedRequest) -> Result<Response> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("building HTTP client")?;

    let mut builder = client.request(method(request.method), &request.url);
    for (name, value) in &request.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    if !request.query.is_empty() {
        builder = builder.query(&request.query);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }

    let started = Instant::now();
    let response = builder.send().context("sending request")?;
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                value.to_str().unwrap_or("<binary>").to_string(),
            )
        })
        .collect::<Vec<_>>();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let raw = response.text().context("reading response body")?;
    let time_ms = started.elapsed().as_millis();
    let size = raw.len();
    let body = pretty_print(&raw, content_type.as_deref());

    Ok(Response {
        status: status.as_u16(),
        status_text: status
            .canonical_reason()
            .unwrap_or("")
            .to_string(),
        time_ms,
        size,
        headers,
        body,
        content_type,
    })
}

fn method(method: Method) -> reqwest::Method {
    match method {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Patch => reqwest::Method::PATCH,
        Method::Delete => reqwest::Method::DELETE,
        Method::Head => reqwest::Method::HEAD,
    }
}

fn pretty_print(raw: &str, content_type: Option<&str>) -> String {
    let looks_json = content_type
        .map(|ct| ct.contains("json"))
        .unwrap_or(false);
    if looks_json {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
            if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                return pretty;
            }
        }
    }
    raw.to_string()
}
