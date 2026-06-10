use ratatui::widgets::ListState;

pub fn select_first_when_present(state: &mut ListState, len: usize) {
    state.select((len > 0).then_some(0));
}

pub fn clamp_selection(state: &mut ListState, len: usize) {
    state.select(match (len, state.selected()) {
        (0, _) => None,
        (size, Some(index)) => Some(index.min(size - 1)),
        (_, None) => Some(0),
    });
}

pub fn next(state: &mut ListState, len: usize) {
    state.select(cycled_index(state.selected(), len, 1));
}

pub fn previous(state: &mut ListState, len: usize) {
    state.select(cycled_index(state.selected(), len, len.saturating_sub(1)));
}

fn cycled_index(selected: Option<usize>, len: usize, offset: usize) -> Option<usize> {
    (len > 0).then(|| (selected.unwrap_or(0) + offset) % len)
}
