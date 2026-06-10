pub mod view;

use crate::shared::bruno::MissingVar;

/// Modal state for collecting values of variables that are missing at send time. The
/// queue is consumed one variable at a time.
pub struct State {
    pub queue: Vec<MissingVar>,
    pub index: usize,
    pub input: String,
}

impl State {
    pub fn new(queue: Vec<MissingVar>) -> Self {
        Self {
            queue,
            index: 0,
            input: String::new(),
        }
    }

    pub fn current(&self) -> Option<&MissingVar> {
        self.queue.get(self.index)
    }

    pub fn push(&mut self, ch: char) {
        self.input.push(ch);
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    /// Advance to the next variable, returning the just-entered (name, value). Returns
    /// `None` when the queue is exhausted.
    pub fn take_current(&mut self) -> Option<(String, String)> {
        let name = self.current()?.name.clone();
        let value = std::mem::take(&mut self.input);
        self.index += 1;
        Some((name, value))
    }

    pub fn is_done(&self) -> bool {
        self.index >= self.queue.len()
    }
}
