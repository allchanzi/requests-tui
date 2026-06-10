use std::path::Path;

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

use super::{Pane, State};
use crate::shared::bruno::Collection;
use crate::shared::ui::UiTheme;

/// Render the left-lower pane: either the list of discovered collections, or the
/// flattened request tree of the entered collection.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
    root: &Path,
    collections: &[Collection],
    state: &mut State,
    focused: bool,
) {
    match state.pane {
        Pane::Collections => {
            render_collections(frame, area, theme, root, collections, state, focused)
        }
        Pane::Requests => render_requests(frame, area, theme, collections, state, focused),
    }
}

fn render_collections(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
    root: &Path,
    collections: &[Collection],
    state: &mut State,
    focused: bool,
) {
    let items: Vec<ListItem> = if collections.is_empty() {
        vec![ListItem::new(Span::styled(
            "No collections found under root",
            theme.muted(),
        ))]
    } else {
        collections
            .iter()
            .map(|collection| {
                let count = count_requests(collection);
                let location = location_label(&collection.path, root);
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(collection.name.clone(), theme.title()),
                        Span::styled(format!("  ({count})"), theme.muted()),
                    ]),
                    Line::from(Span::styled(format!("  {location}"), theme.muted())),
                ])
            })
            .collect()
    };

    let list = List::new(items)
        .block(theme.panel_block("Collections", focused))
        .highlight_style(theme.selected())
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut state.collections);
}

fn render_requests(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &UiTheme,
    collections: &[Collection],
    state: &mut State,
    focused: bool,
) {
    let title = state
        .collections
        .selected()
        .and_then(|index| collections.get(index))
        .map(|collection| format!("{} (esc: back)", collection.name))
        .unwrap_or_else(|| "Requests".to_string());

    let items: Vec<ListItem> = state
        .rows
        .iter()
        .map(|row| {
            let indent = "  ".repeat(row.depth);
            match row.method {
                Some(method) => Line::from(vec![
                    Span::raw(indent),
                    Span::styled(
                        format!("{:<6}", method.as_str()),
                        theme.method_style(method.as_str()),
                    ),
                    Span::raw(" "),
                    Span::raw(row.label.clone()),
                ]),
                None => Line::from(vec![
                    Span::raw(indent),
                    Span::styled(format!("▸ {}", row.label), theme.folder()),
                ]),
            }
        })
        .map(ListItem::new)
        .collect();

    let list = List::new(items)
        .block(theme.panel_block(title, focused))
        .highlight_style(theme.selected())
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut state.requests);
}

/// The collection's directory shown relative to the scan root, so collections with the
/// same name living in different git worktrees are distinguishable. Falls back to the
/// directory's own name when it can't be made relative.
fn location_label(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let text = relative.to_string_lossy();
    if text.is_empty() {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        text.to_string()
    }
}

fn count_requests(collection: &Collection) -> usize {
    fn walk(nodes: &[crate::shared::bruno::Node]) -> usize {
        nodes
            .iter()
            .map(|node| match node {
                crate::shared::bruno::Node::Folder { children, .. } => walk(children),
                crate::shared::bruno::Node::Request(_) => 1,
            })
            .sum()
    }
    walk(&collection.nodes)
}

#[cfg(test)]
mod tests {
    use super::location_label;
    use std::path::Path;

    #[test]
    fn worktree_copies_get_distinct_locations() {
        let root = Path::new("/repo");
        // Same collection name in two different worktrees: the relative paths differ even
        // though the immediate parent folder ("apis") is identical.
        let main = location_label(Path::new("/repo/apis/Example Dashboard"), root);
        let feature = location_label(
            Path::new("/repo/.worktrees/feature-x/apis/Example Dashboard"),
            root,
        );
        assert_eq!(main, "apis/Example Dashboard");
        assert_eq!(feature, ".worktrees/feature-x/apis/Example Dashboard");
        assert_ne!(main, feature);
    }

    #[test]
    fn falls_back_to_dir_name_when_not_under_root() {
        let label = location_label(Path::new("/elsewhere/My Collection"), Path::new("/repo"));
        assert_eq!(label, "/elsewhere/My Collection");
    }
}
