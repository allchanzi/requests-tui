pub mod view;

use ratatui::widgets::ListState;

use crate::shared::ui::selection;

/// View state for the environments slice.
pub struct State {
    pub list: ListState,
}

impl Default for State {
    fn default() -> Self {
        let mut list = ListState::default();
        list.select(Some(0));
        Self { list }
    }
}

impl State {
    pub fn selected(&self) -> Option<usize> {
        self.list.selected()
    }

    pub fn move_down(&mut self, len: usize) {
        selection::next(&mut self.list, len);
    }

    pub fn move_up(&mut self, len: usize) {
        selection::previous(&mut self.list, len);
    }

    pub fn clamp(&mut self, len: usize) {
        selection::clamp_selection(&mut self.list, len);
    }
}
