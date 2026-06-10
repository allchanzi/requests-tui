use std::path::{Path, PathBuf};

use super::model::{Auth, BodyMode, Entry, Environment, Method, Request, RequestType};

/// One top-level block in a `.bru` file, e.g. `meta { ... }` or `vars:secret [ ... ]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub name: String,
    pub kind: BlockKind,
    /// Raw text between the delimiters (dedented for brace blocks).
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Braces,
    Brackets,
}

impl Block {
    /// Parse a brace block as ordered `key: value` entries (skipping blank lines and
    /// the empty placeholder line Bruno writes as `: `). A leading `~` marks an entry
    /// as disabled.
    pub fn entries(&self) -> Vec<Entry> {
        self.body
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let (enabled, rest) = match trimmed.strip_prefix('~') {
                    Some(rest) => (false, rest.trim_start()),
                    None => (true, trimmed),
                };
                let (key, value) = rest.split_once(':')?;
                let key = key.trim();
                if key.is_empty() {
                    return None;
                }
                Some(Entry::new(key, value.trim(), enabled))
            })
            .collect()
    }

    /// Look up a single `key: value` within a brace block.
    pub fn value(&self, key: &str) -> Option<String> {
        self.entries()
            .into_iter()
            .find(|entry| entry.name == key)
            .map(|entry| entry.value)
    }

    /// Parse a bracket block as a comma/newline separated list of names.
    pub fn names(&self) -> Vec<String> {
        self.body
            .split([',', '\n'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect()
    }
}

/// Tokenize a `.bru` document into its top-level blocks. Brace blocks are matched by
/// depth counting while ignoring delimiters inside double-quoted strings, so JSON and
/// GraphQL bodies survive intact.
pub fn parse_blocks(input: &str) -> Vec<Block> {
    let chars: Vec<char> = input.chars().collect();
    let mut blocks = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index].is_whitespace() {
            index += 1;
            continue;
        }

        let name_start = index;
        while index < chars.len() && is_name_char(chars[index]) {
            index += 1;
        }
        if index == name_start {
            // Not a block header (stray char); skip it.
            index += 1;
            continue;
        }
        let name: String = chars[name_start..index].iter().collect();

        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }

        let (kind, open, close) = match chars[index] {
            '{' => (BlockKind::Braces, '{', '}'),
            '[' => (BlockKind::Brackets, '[', ']'),
            _ => continue,
        };

        let (body, next) = capture_block(&chars, index, open, close);
        index = next;

        let body = if kind == BlockKind::Braces {
            dedent(&body)
        } else {
            body
        };
        blocks.push(Block { name, kind, body });
    }

    blocks
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-' | '.')
}

/// Returns the inner content (excluding the delimiters) and the index just past the
/// matching close delimiter.
fn capture_block(chars: &[char], open_index: usize, open: char, close: char) -> (String, usize) {
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut content = String::new();
    let mut index = open_index;

    while index < chars.len() {
        let ch = chars[index];
        index += 1;

        if in_string {
            content.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == open {
            depth += 1;
            if depth == 1 {
                // Skip recording the outermost open delimiter.
                continue;
            }
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                break;
            }
        } else if ch == '"' {
            in_string = true;
        }

        content.push(ch);
    }

    (content, index)
}

/// Remove the common leading-whitespace indentation Bruno adds to block bodies, and
/// trim surrounding blank lines.
fn dedent(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    let dedented: Vec<String> = lines
        .iter()
        .map(|line| {
            if line.len() >= indent {
                line[indent..].to_string()
            } else {
                line.trim_start().to_string()
            }
        })
        .collect();

    dedented.join("\n").trim_matches('\n').to_string()
}

fn find<'a>(blocks: &'a [Block], name: &str) -> Option<&'a Block> {
    blocks.iter().find(|block| block.name == name)
}

/// Resolve auth from a set of blocks (used for both requests and `collection.bru`).
/// `auth { mode: ... }` selects the scheme; `auth:bearer { token: ... }` carries the token.
pub fn parse_auth(blocks: &[Block]) -> Auth {
    let mode = find(blocks, "auth")
        .and_then(|block| block.value("mode"))
        .unwrap_or_default();
    match mode.trim() {
        "inherit" => Auth::Inherit,
        "bearer" => Auth::Bearer(bearer_token(blocks)),
        _ => Auth::None,
    }
}

fn bearer_token(blocks: &[Block]) -> String {
    find(blocks, "auth:bearer")
        .and_then(|block| block.value("token"))
        .unwrap_or_default()
}

/// Parse a request `.bru` file into a [`Request`].
pub fn parse_request(input: &str, path: &Path) -> Option<Request> {
    let blocks = parse_blocks(input);

    let meta = find(&blocks, "meta");
    let name = meta
        .and_then(|block| block.value("name"))
        .or_else(|| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "Untitled".to_string());
    let seq = meta
        .and_then(|block| block.value("seq"))
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(i64::MAX);
    let req_type = meta
        .and_then(|block| block.value("type"))
        .map(|raw| RequestType::parse(&raw))
        .unwrap_or(RequestType::Http);

    let method_block = blocks
        .iter()
        .find(|block| Method::from_block_name(&block.name).is_some())?;
    let method = Method::from_block_name(&method_block.name)?;
    let url = method_block.value("url").unwrap_or_default();
    let body_mode = method_block
        .value("body")
        .map(|raw| BodyMode::parse(&raw))
        .unwrap_or(BodyMode::None);

    // The method block's `auth:` field is authoritative for a request; fall back to the
    // `auth { mode }` block when it is absent.
    let auth = match method_block.value("auth").as_deref() {
        Some("inherit") => Auth::Inherit,
        Some("none") => Auth::None,
        Some("bearer") => Auth::Bearer(bearer_token(&blocks)),
        _ => parse_auth(&blocks),
    };

    let headers = find(&blocks, "headers")
        .map(Block::entries)
        .unwrap_or_default();
    let path_params = find(&blocks, "params:path")
        .map(Block::entries)
        .unwrap_or_default();
    let query_params = find(&blocks, "params:query")
        .map(Block::entries)
        .unwrap_or_default();

    let body = body_block(&blocks, &body_mode);

    Some(Request {
        name,
        seq,
        req_type,
        method,
        url,
        body_mode,
        auth,
        headers,
        path_params,
        query_params,
        body,
        path: PathBuf::from(path),
    })
}

fn body_block(blocks: &[Block], mode: &BodyMode) -> String {
    let block_name = match mode {
        BodyMode::Json => "body:json",
        BodyMode::Text => "body:text",
        BodyMode::Xml => "body:xml",
        BodyMode::Graphql => "body:graphql",
        BodyMode::Other(name) => return find(blocks, &format!("body:{name}"))
            .map(|block| block.body.clone())
            .unwrap_or_default(),
        BodyMode::None => return String::new(),
    };
    find(blocks, block_name)
        .map(|block| block.body.clone())
        .unwrap_or_default()
}

/// Parse an environment `.bru` file (the `vars` and `vars:secret` blocks).
pub fn parse_environment(input: &str, name: &str) -> Environment {
    let blocks = parse_blocks(input);
    let vars = find(&blocks, "vars").map(Block::entries).unwrap_or_default();
    let secret_vars = find(&blocks, "vars:secret")
        .map(Block::names)
        .unwrap_or_default();
    Environment {
        name: name.to_string(),
        vars,
        secret_vars,
    }
}

/// Parse `collection.bru`, returning collection-level auth, headers and vars.
pub fn parse_collection_config(input: &str) -> (Auth, Vec<Entry>, Vec<Entry>) {
    let blocks = parse_blocks(input);
    let auth = parse_auth(&blocks);
    let headers = find(&blocks, "headers")
        .map(Block::entries)
        .unwrap_or_default();
    let vars = find(&blocks, "vars").map(Block::entries).unwrap_or_default();
    (auth, headers, vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::Path;

    use crate::shared::bruno::interpolate::VarContext;
    use crate::shared::bruno::model::Collection;

    const GET_ITEM: &str = "meta {\n  name: Get Item\n  type: http\n  seq: 4\n}\n\nget {\n  url: {{url}}/api/v1/items/:item_id\n  body: none\n  auth: inherit\n}\n\nparams:path {\n  item_id: 42\n}\n\nheaders {\n  Content-Type: application/json\n  X-Custom: 1\n}\n";

    const CREATE_ITEM: &str = "meta {\n  name: Create Item\n  type: http\n  seq: 14\n}\n\npost {\n  url: {{url}}/api/v1/items\n  body: json\n  auth: inherit\n}\n\nheaders {\n  Content-Type: application/json\n}\n\nbody:json {\n  {\n    \"name\": \"Example\",\n    \"nested\": { \"a\": 1 }\n  }\n}\n";

    const GET_NODE_GQL: &str = "meta {\n  name: Get Node\n  type: graphql\n  seq: 5\n}\n\npost {\n  url: {{url}}graphql/\n  body: graphql\n  auth: inherit\n}\n\nbody:graphql {\n  query {\n    node(id: \"1\") {\n      id\n      state\n    }\n  }\n}\n";

    const ENV_STAGE: &str = "vars {\n  url: https://api.example.com/\n}\nvars:secret [\n  access-token\n]\n";

    #[test]
    fn parses_get_with_path_params_and_headers() {
        let request = parse_request(GET_ITEM, Path::new("Get Item.bru")).unwrap();
        assert_eq!(request.name, "Get Item");
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.seq, 4);
        assert_eq!(request.auth, Auth::Inherit);
        assert_eq!(request.url, "{{url}}/api/v1/items/:item_id");
        assert_eq!(request.path_params.len(), 1);
        assert_eq!(request.path_params[0].name, "item_id");
        assert_eq!(request.path_params[0].value, "42");
        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers[0].name, "Content-Type");
    }

    #[test]
    fn parses_json_body_with_nested_braces() {
        let request = parse_request(CREATE_ITEM, Path::new("Create Item.bru")).unwrap();
        assert_eq!(request.method, Method::Post);
        assert_eq!(request.body_mode, BodyMode::Json);
        // The nested object braces must survive intact and round-trip as JSON.
        let value: serde_json::Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(value["name"], "Example");
        assert_eq!(value["nested"]["a"], 1);
    }

    #[test]
    fn parses_graphql_body() {
        let request = parse_request(GET_NODE_GQL, Path::new("Get Node.bru")).unwrap();
        assert_eq!(request.req_type, RequestType::Graphql);
        assert_eq!(request.body_mode, BodyMode::Graphql);
        assert!(request.body.contains("node(id: \"1\")"));
    }

    #[test]
    fn parses_environment_with_secret() {
        let environment = parse_environment(ENV_STAGE, "Example Stage");
        assert_eq!(environment.name, "Example Stage");
        let url = environment.vars.iter().find(|entry| entry.name == "url");
        assert_eq!(
            url.map(|entry| entry.value.as_str()),
            Some("https://api.example.com/")
        );
        assert!(environment.is_secret("access-token"));
    }

    const OWN_BEARER: &str = "meta {\n  name: Own Bearer\n  type: http\n  seq: 1\n}\n\nget {\n  url: https://example.com/x\n  body: none\n  auth: bearer\n}\n\nauth:bearer {\n  token: abc123\n}\n";

    const JSON_NO_CT: &str = "meta {\n  name: Json No CT\n  type: http\n  seq: 1\n}\n\npost {\n  url: https://example.com/x\n  body: json\n  auth: none\n}\n\nbody:json {\n  {\"a\": 1}\n}\n";

    #[test]
    fn parses_own_bearer_without_mode_block() {
        // A request that declares `auth: bearer` but has no `auth { mode }` block must
        // still pick up the token from `auth:bearer`.
        let request = parse_request(OWN_BEARER, Path::new("Own Bearer.bru")).unwrap();
        assert_eq!(request.auth, Auth::Bearer("abc123".to_string()));
    }

    #[test]
    fn prepare_adds_default_content_type_and_authorization() {
        let collection = Collection {
            name: "c".into(),
            path: Path::new(".").to_path_buf(),
            auth: Auth::None,
            headers: vec![],
            vars: vec![],
            nodes: vec![],
            environments: vec![],
            process_env: HashMap::new(),
        };

        // JSON body with no explicit Content-Type gets one.
        let request = parse_request(JSON_NO_CT, Path::new("Json No CT.bru")).unwrap();
        let prepared = crate::shared::bruno::prepare(&request, &collection, None, &HashMap::new());
        assert!(prepared
            .request
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("content-type") && v == "application/json"));

        // Own bearer becomes an Authorization header.
        let bearer = parse_request(OWN_BEARER, Path::new("Own Bearer.bru")).unwrap();
        let prepared = crate::shared::bruno::prepare(&bearer, &collection, None, &HashMap::new());
        assert!(prepared
            .request
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("authorization") && v == "Bearer abc123"));
    }

    #[test]
    fn detects_missing_secret_var() {
        // url is provided, access-token is an empty secret -> missing when referenced.
        let environment = parse_environment(ENV_STAGE, "Example Stage");
        let collection = Collection {
            name: "test".into(),
            path: Path::new(".").to_path_buf(),
            auth: Auth::None,
            headers: vec![],
            vars: vec![],
            nodes: vec![],
            environments: vec![],
            process_env: HashMap::new(),
        };
        let context = VarContext::build(&collection, Some(&environment), &HashMap::new());
        let mut missing = Vec::new();
        let expanded = context.expand("{{url}}path {{access-token}}", &mut missing);
        assert!(expanded.starts_with("https://api.example.com"));
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].name, "access-token");
        assert!(missing[0].secret);
    }

    #[test]
    fn resolves_process_env_reference() {
        let mut process_env = HashMap::new();
        process_env.insert("API_KEY".to_string(), "s3cr3t".to_string());
        let collection = Collection {
            name: "test".into(),
            path: Path::new(".").to_path_buf(),
            auth: Auth::None,
            headers: vec![],
            vars: vec![],
            nodes: vec![],
            environments: vec![],
            process_env,
        };
        let context = VarContext::build(&collection, None, &HashMap::new());
        let mut missing = Vec::new();
        let expanded = context.expand("Bearer {{process.env.API_KEY}}", &mut missing);
        assert_eq!(expanded, "Bearer s3cr3t");
        assert!(missing.is_empty());

        // An undefined process.env reference is reported missing and flagged secret.
        let mut missing = Vec::new();
        context.expand("{{process.env.MISSING}}", &mut missing);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].name, "process.env.MISSING");
        assert!(missing[0].secret);
    }
}
