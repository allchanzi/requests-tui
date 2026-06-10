use std::io::Write;
use std::process::{Command, Stdio};

use crate::shared::bruno::{Method, PreparedRequest};

/// Render a prepared request as a copy-pasteable `curl` command (one option per line,
/// continued with `\`).
pub fn to_curl(request: &PreparedRequest) -> String {
    let url = with_query(&request.url, &request.query);

    let mut parts = vec![format!(
        "curl --request {} {}",
        method(request.method),
        shell_quote(&url)
    )];

    if request.method == Method::Head {
        parts[0] = format!("curl --head {}", shell_quote(&url));
    }

    for (name, value) in &request.headers {
        parts.push(format!("--header {}", shell_quote(&format!("{name}: {value}"))));
    }

    if let Some(body) = &request.body {
        parts.push(format!("--data {}", shell_quote(body)));
    }

    parts.join(" \\\n  ")
}

/// Copy `text` to the system clipboard, returning whether a clipboard tool succeeded.
/// Tries the platform-appropriate CLI tools without pulling in a dependency.
pub fn copy_to_clipboard(text: &str) -> bool {
    let candidates: &[(&str, &[&str])] = &[
        ("pbcopy", &[]),
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
        ("clip", &[]),
    ];

    for (program, args) in candidates {
        if try_copy(program, args, text) {
            return true;
        }
    }
    false
}

fn try_copy(program: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    if let Some(stdin) = child.stdin.as_mut() {
        if stdin.write_all(text.as_bytes()).is_err() {
            return false;
        }
    }
    matches!(child.wait(), Ok(status) if status.success())
}

fn method(method: Method) -> &'static str {
    method.as_str()
}

fn with_query(url: &str, query: &[(String, String)]) -> String {
    if query.is_empty() {
        return url.to_string();
    }
    let encoded = query
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let separator = if url.contains('?') { "&" } else { "?" };
    format!("{url}{separator}{encoded}")
}

/// Wrap a string in single quotes for POSIX shells, escaping embedded single quotes.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Minimal percent-encoding for query components (unreserved chars pass through).
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_curl_with_query_headers_and_body() {
        let request = PreparedRequest {
            method: Method::Post,
            url: "https://example.com/api".to_string(),
            query: vec![("q".to_string(), "a b".to_string())],
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some("{\"x\":1}".to_string()),
        };
        let curl = to_curl(&request);
        assert!(curl.contains("curl --request POST 'https://example.com/api?q=a%20b'"));
        assert!(curl.contains("--header 'Content-Type: application/json'"));
        assert!(curl.contains("--data '{\"x\":1}'"));
    }

    #[test]
    fn escapes_single_quotes_in_body() {
        let request = PreparedRequest {
            method: Method::Post,
            url: "https://example.com".to_string(),
            query: vec![],
            headers: vec![],
            body: Some("it's".to_string()),
        };
        let curl = to_curl(&request);
        assert!(curl.contains("--data 'it'\\''s'"));
    }
}
