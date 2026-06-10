pub mod view;

/// View state for the response pane (scroll position and header visibility).
#[derive(Default)]
pub struct State {
    pub scroll: u16,
    pub show_headers: bool,
}

impl State {
    pub fn reset(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_down(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_add(lines);
    }

    pub fn scroll_up(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    pub fn toggle_headers(&mut self) {
        self.show_headers = !self.show_headers;
    }
}
