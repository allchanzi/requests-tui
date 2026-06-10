use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::State;
use crate::shared::ui::UiTheme;

/// Render the centered missing-variable prompt modal.
pub fn render(frame: &mut Frame<'_>, theme: &UiTheme, state: &State) {
    let Some(current) = state.current() else {
        return;
    };

    let area = centered_rect(60, 9, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.focused_border())
        .title(format!(
            " Missing variable {}/{} ",
            state.index + 1,
            state.queue.len()
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [prompt_area, input_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .areas(inner);

    let secret = if current.secret { " (secret)" } else { "" };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("Enter value for "),
            Span::styled(format!("{{{{{}}}}}", current.name), theme.accent()),
            Span::styled(secret.to_string(), theme.muted()),
        ])),
        prompt_area,
    );

    let shown = if current.secret {
        "*".repeat(state.input.chars().count())
    } else {
        state.input.clone()
    };
    frame.render_widget(
        Paragraph::new(format!("> {shown}\u{2588}")).style(theme.title()),
        input_area,
    );

    frame.render_widget(
        Paragraph::new("enter: confirm · esc: cancel").style(theme.muted()),
        hint_area,
    );
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area)[1];

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical)[1]
}
