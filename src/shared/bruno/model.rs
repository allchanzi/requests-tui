use std::collections::HashMap;
use std::path::PathBuf;

/// HTTP method declared by a request's method block (`get`, `post`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

impl Method {
    pub fn from_block_name(name: &str) -> Option<Self> {
        match name {
            "get" => Some(Self::Get),
            "post" => Some(Self::Post),
            "put" => Some(Self::Put),
            "patch" => Some(Self::Patch),
            "delete" => Some(Self::Delete),
            "head" => Some(Self::Head),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
        }
    }
}

/// How the request body is encoded, taken from the method block's `body:` field
/// and the matching `body:<mode>` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyMode {
    None,
    Json,
    Text,
    Xml,
    Graphql,
    Other(String),
}

impl BodyMode {
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "" | "none" => Self::None,
            "json" => Self::Json,
            "text" => Self::Text,
            "xml" => Self::Xml,
            "graphql" => Self::Graphql,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Request kind from `meta { type: ... }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    Http,
    Graphql,
}

impl RequestType {
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "graphql" => Self::Graphql,
            _ => Self::Http,
        }
    }
}

/// Authentication resolved for a request or declared on a collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auth {
    None,
    Inherit,
    Bearer(String),
}

/// A single header/param line; Bruno marks disabled entries with a leading `~`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

impl Entry {
    pub fn new(name: impl Into<String>, value: impl Into<String>, enabled: bool) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            enabled,
        }
    }
}

/// A single executable request parsed from a `.bru` file.
#[derive(Debug, Clone)]
pub struct Request {
    pub name: String,
    pub seq: i64,
    pub req_type: RequestType,
    pub method: Method,
    pub url: String,
    pub body_mode: BodyMode,
    pub auth: Auth,
    pub headers: Vec<Entry>,
    pub path_params: Vec<Entry>,
    pub query_params: Vec<Entry>,
    pub body: String,
    pub path: PathBuf,
}

/// A node in the collection tree: either a folder grouping more nodes, or a request.
#[derive(Debug, Clone)]
pub enum Node {
    Folder { name: String, children: Vec<Node> },
    Request(Request),
}

/// A single environment with its variables and the names marked secret.
#[derive(Debug, Clone)]
pub struct Environment {
    pub name: String,
    pub vars: Vec<Entry>,
    pub secret_vars: Vec<String>,
}

impl Environment {
    pub fn is_secret(&self, name: &str) -> bool {
        self.secret_vars.iter().any(|secret| secret == name)
    }
}

/// A discovered Bruno collection: its tree of requests, collection-level defaults
/// (from `collection.bru`), and environments.
#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub path: PathBuf,
    pub auth: Auth,
    pub headers: Vec<Entry>,
    pub vars: Vec<Entry>,
    pub nodes: Vec<Node>,
    pub environments: Vec<Environment>,
    /// Values loaded from the collection's `.env` file, referenced in requests as
    /// `{{process.env.<key>}}` (Bruno-compatible). Never written back to disk.
    pub process_env: HashMap<String, String>,
}
