pub mod view;

use tui_textarea::TextArea;

use crate::shared::bruno::{BodyMode, Entry, Request};

/// Which editable field of the request currently has the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Url,
    Headers,
    Body,
}

impl Field {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Url => "URL",
            Self::Headers => "Headers",
            Self::Body => "Body",
        }
    }
}

/// Editable view of the loaded request. Edits live only in these buffers and are never
/// written back to the `.bru` file.
pub struct State {
    pub source: Option<Request>,
    pub field: Field,
    pub editing: bool,
    pub url: String,
    pub headers: String,
    pub body: TextArea<'static>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            source: None,
            field: Field::Url,
            editing: false,
            url: String::new(),
            headers: String::new(),
            body: TextArea::default(),
        }
    }
}

impl State {
    /// Load a request into the editable buffers.
    pub fn load(&mut self, request: Request) {
        self.url = request.url.clone();
        self.headers = headers_to_text(&request.headers);
        self.body = TextArea::from(request.body.lines().map(str::to_string).collect::<Vec<_>>());
        self.field = Field::Url;
        self.editing = false;
        self.source = Some(request);
    }

    pub fn has_request(&self) -> bool {
        self.source.is_some()
    }

    pub fn next_field(&mut self) {
        self.field = match self.field {
            Field::Url => Field::Headers,
            Field::Headers => Field::Body,
            Field::Body => Field::Url,
        };
    }

    pub fn previous_field(&mut self) {
        self.field = match self.field {
            Field::Url => Field::Body,
            Field::Headers => Field::Url,
            Field::Body => Field::Headers,
        };
    }

    /// Build the effective request to send, applying the edited URL / headers / body
    /// over the parsed source.
    pub fn effective_request(&self) -> Option<Request> {
        let mut request = self.source.clone()?;
        request.url = self.url.clone();
        request.headers = text_to_headers(&self.headers);
        request.body = self.body.lines().join("\n");
        Some(request)
    }

    pub fn body_mode_label(&self) -> &'static str {
        match self.source.as_ref().map(|request| &request.body_mode) {
            Some(BodyMode::Json) => "json",
            Some(BodyMode::Graphql) => "graphql",
            Some(BodyMode::Text) => "text",
            Some(BodyMode::Xml) => "xml",
            Some(BodyMode::Other(_)) => "other",
            Some(BodyMode::None) | None => "none",
        }
    }
}

fn headers_to_text(headers: &[Entry]) -> String {
    headers
        .iter()
        .map(|entry| {
            let prefix = if entry.enabled { "" } else { "~" };
            format!("{prefix}{}: {}", entry.name, entry.value)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn text_to_headers(text: &str) -> Vec<Entry> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (enabled, rest) = match trimmed.strip_prefix('~') {
                Some(rest) => (false, rest.trim_start()),
                None => (true, trimmed),
            };
            let (name, value) = rest.split_once(':')?;
            Some(Entry::new(name.trim(), value.trim(), enabled))
        })
        .collect()
}
