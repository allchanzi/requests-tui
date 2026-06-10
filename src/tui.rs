use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::app::{App, Flow, Focus, Overlay};
use crate::features::{collections, environments, prompt, request, response};

type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn run(mut app: App) -> Result<()> {
    let mut terminal = init_terminal()?;
    let result = event_loop(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

fn init_terminal() -> Result<AppTerminal> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
    Terminal::new(CrosstermBackend::new(stdout)).context("failed to create terminal")
}

fn restore_terminal(terminal: &mut AppTerminal) -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")
}

fn event_loop(terminal: &mut AppTerminal, app: &mut App) -> Result<()> {
    loop {
        app.poll_send();
        terminal.draw(|frame| render(frame, app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && app.handle_key(key) == Flow::Quit {
                return Ok(());
            }
        }
    }
}

fn render(frame: &mut Frame<'_>, app: &mut App) {
    let [main, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .areas(frame.area());

    let [left, right] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .areas(main);

    let [env_area, coll_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Min(0)])
        .areas(left);

    let has_response = app.response.is_some() || app.error.is_some();
    let right_split = if has_response {
        [Constraint::Percentage(50), Constraint::Percentage(50)]
    } else {
        [Constraint::Percentage(90), Constraint::Min(1)]
    };
    let [req_area, resp_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints(right_split)
        .areas(right);

    // Copy out the small Copy fields before borrowing the rest mutably.
    let focus = app.focus;
    let sending = app.sending;
    let active_collection = app.active_collection;
    let active_environment = app.active_environment;

    let App {
        theme,
        root,
        collections,
        collections_state,
        environments_state,
        request_state,
        response_state,
        response,
        error,
        overlay,
        message,
        ..
    } = app;

    let active = active_collection.and_then(|index| collections.get(index));
    environments::view::render(
        frame,
        env_area,
        theme,
        active,
        active_environment,
        environments_state,
        focus == Focus::Environments,
    );
    collections::view::render(
        frame,
        coll_area,
        theme,
        root,
        collections,
        collections_state,
        focus == Focus::Collections,
    );
    request::view::render(
        frame,
        req_area,
        theme,
        request_state,
        focus == Focus::Request,
        sending,
    );
    response::view::render(
        frame,
        resp_area,
        theme,
        response.as_ref(),
        error.as_deref(),
        sending,
        response_state,
        focus == Focus::Response,
    );

    render_footer(frame, footer, theme, message);

    match overlay {
        Overlay::Help => render_help(frame, theme),
        Overlay::Prompt(state) => prompt::view::render(frame, theme, state),
        Overlay::Curl(command) => render_curl(frame, theme, command),
        Overlay::None => {}
    }
}

fn render_curl(frame: &mut Frame<'_>, theme: &crate::shared::ui::UiTheme, command: &str) {
    let area = centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.focused_border())
        .title(" curl  (y: copy · esc: close) ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    frame.render_widget(
        Paragraph::new(command.to_string())
            .style(theme.title())
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &crate::shared::ui::UiTheme,
    message: &str,
) {
    let line = Line::from(vec![
        Span::raw(message.to_string()),
        Span::raw("  "),
        Span::styled(
            "tab: pane · enter: open/load · s: send · c: curl · ?: help · q: quit",
            theme.muted(),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, theme: &crate::shared::ui::UiTheme) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(Span::styled(
            format!("Requests TUI — help (theme: {})", theme.name),
            theme.help_title(),
        )),
        Line::from(""),
        Line::from("tab / shift-tab   move focus between panes"),
        Line::from("j / k, ↑ / ↓      navigate within a pane"),
        Line::from("enter             open collection / load request / activate env"),
        Line::from("esc / backspace   back to collection list"),
        Line::from("i or e            edit the focused request field"),
        Line::from("esc               stop editing"),
        Line::from("s                 send the request"),
        Line::from("c                 generate a curl command (y to copy)"),
        Line::from("h (response)      toggle response headers"),
        Line::from("?                 toggle this help"),
        Line::from("q / ctrl-c        quit"),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help (?) ")
        .border_style(theme.help_title());
    frame.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: true }), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
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
