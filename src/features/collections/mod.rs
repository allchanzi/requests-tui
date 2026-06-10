pub mod view;

use ratatui::widgets::ListState;

use crate::shared::bruno::{Collection, Method, Node, Request};
use crate::shared::ui::selection;

/// Which level of the left-lower pane is showing: the list of collections, or the
/// request tree of the entered collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Collections,
    Requests,
}

/// A flattened tree row for display: folders carry no request, requests carry a clone.
#[derive(Debug, Clone)]
pub struct Row {
    pub depth: usize,
    pub label: String,
    pub method: Option<Method>,
    pub request: Option<Request>,
}

/// View state for the collections slice (selection only; the data lives in `App`).
pub struct State {
    pub pane: Pane,
    pub collections: ListState,
    pub rows: Vec<Row>,
    pub requests: ListState,
}

impl Default for State {
    fn default() -> Self {
        let mut collections = ListState::default();
        collections.select(Some(0));
        Self {
            pane: Pane::Collections,
            collections,
            rows: Vec::new(),
            requests: ListState::default(),
        }
    }
}

impl State {
    pub fn selected_collection(&self) -> Option<usize> {
        self.collections.selected()
    }

    /// Enter a collection: flatten its tree into display rows and switch to the
    /// requests pane.
    pub fn enter(&mut self, collection: &Collection) {
        self.rows.clear();
        flatten(&collection.nodes, 0, &mut self.rows);
        self.pane = Pane::Requests;
        selection::select_first_when_present(&mut self.requests, self.rows.len());
    }

    pub fn back(&mut self) {
        self.pane = Pane::Collections;
    }

    pub fn selected_request(&self) -> Option<&Request> {
        self.requests
            .selected()
            .and_then(|index| self.rows.get(index))
            .and_then(|row| row.request.as_ref())
    }

    pub fn move_down(&mut self, collections_len: usize) {
        match self.pane {
            Pane::Collections => selection::next(&mut self.collections, collections_len),
            Pane::Requests => selection::next(&mut self.requests, self.rows.len()),
        }
    }

    pub fn move_up(&mut self, collections_len: usize) {
        match self.pane {
            Pane::Collections => selection::previous(&mut self.collections, collections_len),
            Pane::Requests => selection::previous(&mut self.requests, self.rows.len()),
        }
    }
}

fn flatten(nodes: &[Node], depth: usize, out: &mut Vec<Row>) {
    for node in nodes {
        match node {
            Node::Folder { name, children } => {
                out.push(Row {
                    depth,
                    label: name.clone(),
                    method: None,
                    request: None,
                });
                flatten(children, depth + 1, out);
            }
            Node::Request(request) => out.push(Row {
                depth,
                label: request.name.clone(),
                method: Some(request.method),
                request: Some(request.clone()),
            }),
        }
    }
}
