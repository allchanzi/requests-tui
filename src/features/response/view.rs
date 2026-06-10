use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::State;
use crate::shared::http::Response;
use crate::shared::ui::UiTheme;

/// Render the bottom-right response pane: a status line plus the (optionally
/// header-prefixed) body, or a placeholder/sending/error state.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
    response: Option<&Response>,
    error: Option<&str>,
    sending: bool,
    state: &mut State,
    focused: bool,
) {
    let outer = theme.panel_block("Response  [h: headers · j/k: scroll]", focused);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if let Some(error) = error {
        frame.render_widget(
            Paragraph::new(error.to_string())
                .style(theme.status_style(599))
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let Some(response) = response else {
        let text = if sending {
            "Sending…"
        } else {
            "No response yet. Focus a request and press s to send."
        };
        frame.render_widget(Paragraph::new(text).style(theme.muted()), inner);
        return;
    };

    let [status_area, body_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .areas(inner);

    let status_line = Line::from(vec![
        Span::styled(
            format!("{} {}", response.status, response.status_text),
            theme.status_style(response.status),
        ),
        Span::raw("  "),
        Span::styled(format!("{} ms", response.time_ms), theme.muted()),
        Span::raw("  "),
        Span::styled(format!("{} B", response.size), theme.muted()),
        Span::raw("  "),
        Span::styled(
            response.content_type.clone().unwrap_or_default(),
            theme.muted(),
        ),
    ]);
    frame.render_widget(Paragraph::new(status_line), status_area);

    let mut lines: Vec<Line> = Vec::new();
    if state.show_headers {
        for (name, value) in &response.headers {
            lines.push(Line::from(vec![
                Span::styled(format!("{name}: "), theme.accent()),
                Span::raw(value.clone()),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "──────────",
            theme.muted(),
        )));
    }
    for line in response.body.lines() {
        lines.push(Line::from(line.to_string()));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .scroll((state.scroll, 0))
            .wrap(Wrap { trim: false }),
        body_area,
    );
}
