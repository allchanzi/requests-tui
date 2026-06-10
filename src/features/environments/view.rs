use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

use super::State;
use crate::shared::bruno::Collection;
use crate::shared::ui::UiTheme;

/// Render the left-upper pane: the environments of the active collection, marking the
/// active one and showing its variables underneath.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
    collection: Option<&Collection>,
    active_environment: Option<usize>,
    state: &mut State,
    focused: bool,
) {
    let Some(collection) = collection else {
        let list = List::new(vec![ListItem::new(Span::styled(
            "Enter a collection to see environments",
            theme.muted(),
        ))])
        .block(theme.panel_block("Environments", focused));
        frame.render_stateful_widget(list, area, &mut state.list);
        return;
    };

    let selected = state.list.selected();
    let mut items: Vec<ListItem> = Vec::new();

    for (index, environment) in collection.environments.iter().enumerate() {
        let active_marker = if Some(index) == active_environment {
            Span::styled("● ", theme.accent())
        } else {
            Span::raw("  ")
        };
        items.push(ListItem::new(Line::from(vec![
            active_marker,
            Span::raw(environment.name.clone()),
        ])));
    }

    if collection.environments.is_empty() {
        items.push(ListItem::new(Span::styled("No environments", theme.muted())));
    }

    // Show variables of the highlighted environment as non-selectable detail rows.
    if let Some(environment) = selected.and_then(|index| collection.environments.get(index)) {
        items.push(ListItem::new(Span::styled("  ── vars ──", theme.muted())));
        for entry in &environment.vars {
            let value = if environment.is_secret(&entry.name) {
                "<secret>".to_string()
            } else if entry.value.is_empty() {
                "<empty>".to_string()
            } else {
                entry.value.clone()
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("  {}: ", entry.name), theme.muted()),
                Span::raw(value),
            ])));
        }
    }

    let list = List::new(items)
        .block(theme.panel_block("Environments (enter: activate)", focused))
        .highlight_style(theme.selected())
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut state.list);
}
