use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::{Field, State};
use crate::shared::ui::UiTheme;

/// Render the top-right request pane: method/meta line plus editable URL, headers and
/// body fields.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
    state: &mut State,
    focused: bool,
    sending: bool,
) {
    let outer = theme.panel_block(request_title(state, sending), focused);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let Some(source) = state.source.clone() else {
        frame.render_widget(
            Paragraph::new("Select a request and press Enter to load it.")
                .style(theme.muted())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    };

    let [meta_area, url_area, headers_area, body_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Percentage(35),
            Constraint::Min(3),
        ])
        .areas(inner);

    // Meta line.
    let meta = Line::from(vec![
        Span::styled(
            format!(" {} ", source.method.as_str()),
            theme.method_style(source.method.as_str()),
        ),
        Span::raw(" "),
        Span::styled(format!("type:{:?}", source.req_type).to_lowercase(), theme.muted()),
        Span::raw("  "),
        Span::styled(format!("body:{}", state.body_mode_label()), theme.muted()),
        Span::raw("  "),
        Span::styled(
            source
                .path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            theme.muted(),
        ),
    ]);
    frame.render_widget(Paragraph::new(meta), meta_area);

    // URL field.
    let url_text = if state.editing && state.field == Field::Url {
        format!("{}\u{2588}", state.url)
    } else {
        state.url.clone()
    };
    frame.render_widget(
        Paragraph::new(url_text)
            .block(field_block(theme, "URL", focused, state.field == Field::Url))
            .wrap(Wrap { trim: false }),
        url_area,
    );

    // Headers field.
    let headers_text = if state.editing && state.field == Field::Headers {
        format!("{}\u{2588}", state.headers)
    } else if state.headers.is_empty() {
        "(no headers)".to_string()
    } else {
        state.headers.clone()
    };
    frame.render_widget(
        Paragraph::new(headers_text)
            .block(field_block(
                theme,
                "Headers (Name: Value, ~ = disabled)",
                focused,
                state.field == Field::Headers,
            ))
            .wrap(Wrap { trim: false }),
        headers_area,
    );

    // Body field (textarea).
    let body_focused = focused && state.field == Field::Body;
    state.body.set_block(field_block(
        theme,
        &format!("Body [{}]", state.body_mode_label()),
        focused,
        body_focused,
    ));
    frame.render_widget(&state.body, body_area);
}

fn request_title(state: &State, sending: bool) -> String {
    let base = match &state.source {
        Some(request) => format!("Request — {}", request.name),
        None => "Request".to_string(),
    };
    if sending {
        format!("{base}  ⏳ sending…")
    } else if state.editing {
        format!("{base}  [editing {} · esc to stop]", state.field.label())
    } else {
        format!("{base}  [i: edit · s: send]")
    }
}

fn field_block(theme: &UiTheme, title: &str, pane_focused: bool, field_active: bool) -> Block<'static> {
    let style = if pane_focused && field_active {
        theme.focused_border()
    } else {
        theme.inactive_border()
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(title.to_string())
}
