use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::features::collections::{self, Pane};
use crate::features::request::Field;
use crate::features::{environments, prompt, request, response};
use crate::shared::bruno::{self, Collection, Environment};
use crate::shared::curl;
use crate::shared::http::{Response, SendHandle};
use crate::shared::ui::UiTheme;

/// Which pane currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Environments,
    Collections,
    Request,
    Response,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Self::Environments => Self::Collections,
            Self::Collections => Self::Request,
            Self::Request => Self::Response,
            Self::Response => Self::Environments,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Environments => Self::Response,
            Self::Collections => Self::Environments,
            Self::Request => Self::Collections,
            Self::Response => Self::Request,
        }
    }
}

/// A modal layered over the main UI.
pub enum Overlay {
    None,
    Help,
    Prompt(prompt::State),
    Curl(String),
}

/// Loop control returned from key handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

/// The application: owns the discovered collection data and each slice's view state.
pub struct App {
    pub theme: UiTheme,
    /// The directory the collections were discovered under; collection paths are shown
    /// relative to it so worktree duplicates are distinguishable.
    pub root: PathBuf,
    pub collections: Vec<Collection>,
    pub active_collection: Option<usize>,
    pub active_environment: Option<usize>,
    pub focus: Focus,
    pub overlay: Overlay,
    pub collections_state: collections::State,
    pub environments_state: environments::State,
    pub request_state: request::State,
    pub response_state: response::State,
    pub send: Option<SendHandle>,
    pub sending: bool,
    pub response: Option<Response>,
    pub error: Option<String>,
    pub overrides: HashMap<String, String>,
    pub message: String,
}

impl App {
    pub fn new(root: PathBuf) -> Result<Self> {
        let collections = bruno::discover(&root)?;
        let message = format!(
            "{} collection(s) found · tab: switch pane · ?: help",
            collections.len()
        );
        Ok(Self {
            theme: UiTheme::default(),
            root,
            collections,
            active_collection: None,
            active_environment: None,
            focus: Focus::Collections,
            overlay: Overlay::None,
            collections_state: collections::State::default(),
            environments_state: environments::State::default(),
            request_state: request::State::default(),
            response_state: response::State::default(),
            send: None,
            sending: false,
            response: None,
            error: None,
            overrides: HashMap::new(),
            message,
        })
    }

    pub fn active_collection(&self) -> Option<&Collection> {
        self.active_collection
            .and_then(|index| self.collections.get(index))
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        let collection = self.active_collection()?;
        self.active_environment
            .and_then(|index| collection.environments.get(index))
    }

    /// Collect a completed in-flight request, if any. Called each event-loop tick.
    pub fn poll_send(&mut self) {
        let Some(handle) = &self.send else {
            return;
        };
        let Some(result) = handle.try_take() else {
            return;
        };
        self.send = None;
        self.sending = false;
        match result {
            Ok(response) => {
                self.message = format!("{} · {} ms", response.status, response.time_ms);
                self.response = Some(response);
                self.response_state.reset();
                self.focus = Focus::Response;
            }
            Err(error) => {
                self.message = format!("Request failed: {error}");
                self.error = Some(error);
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Flow {
        match self.overlay {
            Overlay::Help => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
                ) {
                    self.overlay = Overlay::None;
                }
                return Flow::Continue;
            }
            Overlay::Prompt(_) => return self.handle_prompt_key(key),
            Overlay::Curl(_) => {
                self.handle_curl_key(key);
                return Flow::Continue;
            }
            Overlay::None => {}
        }

        if self.focus == Focus::Request && self.request_state.editing {
            self.handle_request_edit_key(key);
            return Flow::Continue;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Quit;
            }
            KeyCode::Char('q') => return Flow::Quit,
            KeyCode::Char('?') => {
                self.overlay = Overlay::Help;
                return Flow::Continue;
            }
            KeyCode::Tab => {
                self.focus = self.focus.next();
                return Flow::Continue;
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
                return Flow::Continue;
            }
            KeyCode::Char('s') => {
                self.trigger_send();
                return Flow::Continue;
            }
            KeyCode::Char('c') => {
                self.generate_curl();
                return Flow::Continue;
            }
            _ => {}
        }

        match self.focus {
            Focus::Environments => self.handle_environments_key(key),
            Focus::Collections => self.handle_collections_key(key),
            Focus::Request => self.handle_request_key(key),
            Focus::Response => self.handle_response_key(key),
        }
        Flow::Continue
    }

    fn handle_environments_key(&mut self, key: KeyEvent) {
        let len = self
            .active_collection()
            .map(|collection| collection.environments.len())
            .unwrap_or(0);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.environments_state.move_up(len),
            KeyCode::Down | KeyCode::Char('j') => self.environments_state.move_down(len),
            KeyCode::Enter => {
                self.active_environment = self.environments_state.selected();
                if let Some(environment) = self.active_environment() {
                    self.message = format!("Environment: {}", environment.name);
                }
            }
            _ => {}
        }
    }

    fn handle_collections_key(&mut self, key: KeyEvent) {
        let len = self.collections.len();
        match self.collections_state.pane {
            Pane::Collections => match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.collections_state.move_up(len),
                KeyCode::Down | KeyCode::Char('j') => self.collections_state.move_down(len),
                KeyCode::Enter => self.enter_selected_collection(),
                _ => {}
            },
            Pane::Requests => match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.collections_state.move_up(len),
                KeyCode::Down | KeyCode::Char('j') => self.collections_state.move_down(len),
                KeyCode::Enter => self.load_selected_request(),
                KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('h') => {
                    self.collections_state.back();
                }
                _ => {}
            },
        }
    }

    fn enter_selected_collection(&mut self) {
        let Some(index) = self.collections_state.selected_collection() else {
            return;
        };
        let Some(collection) = self.collections.get(index) else {
            return;
        };
        let env_len = collection.environments.len();
        let name = collection.name.clone();
        self.collections_state.enter(collection);
        self.active_collection = Some(index);
        self.environments_state.clamp(env_len);
        self.active_environment = (env_len > 0).then(|| self.environments_state.selected().unwrap_or(0));
        self.message = format!("Collection: {name}");
    }

    fn load_selected_request(&mut self) {
        let Some(request) = self.collections_state.selected_request().cloned() else {
            return;
        };
        let name = request.name.clone();
        self.request_state.load(request);
        self.focus = Focus::Request;
        self.message = format!("Loaded {name}");
    }

    fn handle_request_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.request_state.previous_field(),
            KeyCode::Down | KeyCode::Char('j') => self.request_state.next_field(),
            KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('e') => {
                if self.request_state.has_request() {
                    self.request_state.editing = true;
                    self.message = "Editing · esc to stop".into();
                }
            }
            _ => {}
        }
    }

    fn handle_request_edit_key(&mut self, key: KeyEvent) {
        match self.request_state.field {
            Field::Body => match key.code {
                KeyCode::Esc => self.request_state.editing = false,
                _ => {
                    self.request_state.body.input(key);
                }
            },
            Field::Url => match key.code {
                KeyCode::Esc | KeyCode::Enter => self.request_state.editing = false,
                KeyCode::Backspace => {
                    self.request_state.url.pop();
                }
                KeyCode::Char(ch) => self.request_state.url.push(ch),
                _ => {}
            },
            Field::Headers => match key.code {
                KeyCode::Esc => self.request_state.editing = false,
                KeyCode::Enter => self.request_state.headers.push('\n'),
                KeyCode::Backspace => {
                    self.request_state.headers.pop();
                }
                KeyCode::Char(ch) => self.request_state.headers.push(ch),
                _ => {}
            },
        }
    }

    fn handle_response_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.response_state.scroll_down(1),
            KeyCode::Up | KeyCode::Char('k') => self.response_state.scroll_up(1),
            KeyCode::PageDown => self.response_state.scroll_down(10),
            KeyCode::PageUp => self.response_state.scroll_up(10),
            KeyCode::Char('h') => self.response_state.toggle_headers(),
            _ => {}
        }
    }

    fn trigger_send(&mut self) {
        let Some(request) = self.request_state.effective_request() else {
            self.message = "No request loaded".into();
            return;
        };
        let Some(index) = self.active_collection else {
            self.message = "Enter a collection first".into();
            return;
        };

        let prepared = {
            let collection = &self.collections[index];
            let environment = self
                .active_environment
                .and_then(|env_index| collection.environments.get(env_index));
            bruno::prepare(&request, collection, environment, &self.overrides)
        };

        if !prepared.missing.is_empty() {
            self.message = "Missing variables — enter values".into();
            self.overlay = Overlay::Prompt(prompt::State::new(prepared.missing));
            return;
        }

        self.start_send(prepared.request);
    }

    fn generate_curl(&mut self) {
        let Some(request) = self.request_state.effective_request() else {
            self.message = "No request loaded".into();
            return;
        };
        let Some(index) = self.active_collection else {
            self.message = "Enter a collection first".into();
            return;
        };

        let prepared = {
            let collection = &self.collections[index];
            let environment = self
                .active_environment
                .and_then(|env_index| collection.environments.get(env_index));
            bruno::prepare(&request, collection, environment, &self.overrides)
        };

        let command = curl::to_curl(&prepared.request);
        self.message = if prepared.missing.is_empty() {
            "curl generated · y: copy · esc: close".into()
        } else {
            format!(
                "curl generated ({} unresolved var(s) left as placeholders)",
                prepared.missing.len()
            )
        };
        self.overlay = Overlay::Curl(command);
    }

    fn handle_curl_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') => {
                if let Overlay::Curl(command) = &self.overlay {
                    let copied = curl::copy_to_clipboard(command);
                    self.message = if copied {
                        "Copied curl to clipboard".into()
                    } else {
                        "No clipboard tool found (pbcopy/wl-copy/xclip/clip)".into()
                    };
                }
            }
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('q') => {
                self.overlay = Overlay::None;
            }
            _ => {}
        }
    }

    fn start_send(&mut self, request: bruno::PreparedRequest) {
        self.response = None;
        self.error = None;
        self.response_state.reset();
        self.sending = true;
        self.send = Some(SendHandle::spawn(request));
        self.message = "Sending…".into();
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) -> Flow {
        enum Outcome {
            Idle,
            Cancel,
            Entry(Option<(String, String)>, bool),
        }

        let outcome = match &mut self.overlay {
            Overlay::Prompt(state) => match key.code {
                KeyCode::Esc => Outcome::Cancel,
                KeyCode::Enter => {
                    let entry = state.take_current();
                    Outcome::Entry(entry, state.is_done())
                }
                KeyCode::Backspace => {
                    state.backspace();
                    Outcome::Idle
                }
                KeyCode::Char(ch) => {
                    state.push(ch);
                    Outcome::Idle
                }
                _ => Outcome::Idle,
            },
            _ => Outcome::Idle,
        };

        match outcome {
            Outcome::Idle => {}
            Outcome::Cancel => {
                self.overlay = Overlay::None;
                self.message = "Cancelled".into();
            }
            Outcome::Entry(entry, done) => {
                if let Some((name, value)) = entry {
                    self.overrides.insert(name, value);
                }
                if done {
                    self.overlay = Overlay::None;
                    self.trigger_send();
                }
            }
        }
        Flow::Continue
    }
}
