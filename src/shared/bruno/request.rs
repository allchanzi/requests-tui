use std::collections::HashMap;

use super::interpolate::{MissingVar, VarContext};
use super::model::{Auth, BodyMode, Collection, Environment, Method, Request};

/// A request with all `{{var}}` placeholders expanded, ready to hand to the HTTP sender.
#[derive(Debug, Clone)]
pub struct PreparedRequest {
    pub method: Method,
    pub url: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// The outcome of preparing a request: the wire-ready request plus any variables that
/// still need values before it can be sent.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub request: PreparedRequest,
    pub missing: Vec<MissingVar>,
}

/// Expand a request against its collection, the selected environment, and any
/// session-provided overrides, resolving auth, path/query params and the body.
pub fn prepare(
    request: &Request,
    collection: &Collection,
    environment: Option<&Environment>,
    overrides: &HashMap<String, String>,
) -> Prepared {
    let context = VarContext::build(collection, environment, overrides);
    let mut missing = Vec::new();

    // URL: expand vars, then substitute `:path` params.
    let mut url = context.expand(&request.url, &mut missing);
    for param in request.path_params.iter().filter(|entry| entry.enabled) {
        let value = context.expand(&param.value, &mut missing);
        url = url.replace(&format!(":{}", param.name), &value);
    }

    let query = request
        .query_params
        .iter()
        .filter(|entry| entry.enabled)
        .map(|entry| {
            (
                entry.name.clone(),
                context.expand(&entry.value, &mut missing),
            )
        })
        .collect();

    // Collection-level headers are defaults; request headers of the same name win.
    let mut headers: Vec<(String, String)> = Vec::new();
    for entry in collection.headers.iter().filter(|entry| entry.enabled) {
        set_header(
            &mut headers,
            entry.name.clone(),
            context.expand(&entry.value, &mut missing),
        );
    }
    for entry in request.headers.iter().filter(|entry| entry.enabled) {
        set_header(
            &mut headers,
            entry.name.clone(),
            context.expand(&entry.value, &mut missing),
        );
    }

    apply_auth(request, collection, &context, &mut headers, &mut missing);

    let body = build_body(request, &context, &mut headers, &mut missing);

    Prepared {
        request: PreparedRequest {
            method: request.method,
            url,
            query,
            headers,
            body,
        },
        missing,
    }
}

fn apply_auth(
    request: &Request,
    collection: &Collection,
    context: &VarContext,
    headers: &mut Vec<(String, String)>,
    missing: &mut Vec<MissingVar>,
) {
    let resolved = match &request.auth {
        Auth::Inherit => &collection.auth,
        other => other,
    };

    if let Auth::Bearer(token) = resolved {
        if !has_header(headers, "authorization") {
            let token = context.expand(token, missing);
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
    }
}

fn build_body(
    request: &Request,
    context: &VarContext,
    headers: &mut Vec<(String, String)>,
    missing: &mut Vec<MissingVar>,
) -> Option<String> {
    match &request.body_mode {
        BodyMode::None => None,
        BodyMode::Graphql => {
            let query = context.expand(&request.body, missing);
            if !has_header(headers, "content-type") {
                headers.push((
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                ));
            }
            let payload = serde_json::json!({ "query": query });
            Some(payload.to_string())
        }
        mode => {
            let body = context.expand(&request.body, missing);
            if body.trim().is_empty() {
                return None;
            }
            if let Some(content_type) = default_content_type(mode) {
                if !has_header(headers, "content-type") {
                    headers.push(("Content-Type".to_string(), content_type.to_string()));
                }
            }
            Some(body)
        }
    }
}

/// The Content-Type Bruno would imply from a body mode, used only when the request
/// doesn't already set one explicitly.
fn default_content_type(mode: &BodyMode) -> Option<&'static str> {
    match mode {
        BodyMode::Json => Some("application/json"),
        BodyMode::Xml => Some("application/xml"),
        BodyMode::Text => Some("text/plain"),
        _ => None,
    }
}

fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers
        .iter()
        .any(|(key, _)| key.eq_ignore_ascii_case(name))
}

/// Insert or replace a header by case-insensitive name, preserving insertion order.
fn set_header(headers: &mut Vec<(String, String)>, name: String, value: String) {
    if let Some(existing) = headers
        .iter_mut()
        .find(|(key, _)| key.eq_ignore_ascii_case(&name))
    {
        existing.1 = value;
    } else {
        headers.push((name, value));
    }
}
