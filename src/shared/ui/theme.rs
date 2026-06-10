use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};

/// Color palette + style helpers used across slices. Adapted from the pde TUI theme,
/// but with a self-contained default palette (no external theme files).
#[derive(Debug, Clone)]
pub struct UiTheme {
    pub name: String,
    pub palette: Palette,
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub text: Color,
    pub subtle: Color,
    pub surface0: Color,
    pub blue: Color,
    pub pink: Color,
    pub green: Color,
    pub yellow: Color,
    pub cyan: Color,
    pub red: Color,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            name: "catppuccin-mocha".to_string(),
            palette: Palette::default_dark(),
        }
    }
}

impl UiTheme {
    pub fn panel_block(&self, title: impl Into<String>, focused: bool) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_style(if focused {
                self.focused_border()
            } else {
                self.inactive_border()
            })
            .title(title.into())
    }

    pub fn focused_border(&self) -> Style {
        Style::new()
            .fg(self.palette.cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn inactive_border(&self) -> Style {
        Style::new().fg(self.palette.subtle)
    }

    pub fn muted(&self) -> Style {
        Style::new().fg(self.palette.subtle)
    }

    pub fn selected(&self) -> Style {
        Style::new()
            .fg(self.palette.blue)
            .bg(self.palette.surface0)
            .add_modifier(Modifier::BOLD)
    }

    pub fn title(&self) -> Style {
        Style::new()
            .fg(self.palette.text)
            .add_modifier(Modifier::BOLD)
    }

    pub fn help_title(&self) -> Style {
        Style::new()
            .fg(self.palette.cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn method_style(&self, method: &str) -> Style {
        let color = match method {
            "GET" => self.palette.green,
            "POST" => self.palette.yellow,
            "PUT" | "PATCH" => self.palette.blue,
            "DELETE" => self.palette.red,
            _ => self.palette.pink,
        };
        Style::new().fg(color).add_modifier(Modifier::BOLD)
    }

    pub fn status_style(&self, status: u16) -> Style {
        let color = match status {
            200..=299 => self.palette.green,
            300..=399 => self.palette.cyan,
            400..=499 => self.palette.yellow,
            _ => self.palette.red,
        };
        Style::new().fg(color).add_modifier(Modifier::BOLD)
    }

    pub fn accent(&self) -> Style {
        Style::new().fg(self.palette.pink)
    }

    pub fn folder(&self) -> Style {
        Style::new()
            .fg(self.palette.yellow)
            .add_modifier(Modifier::BOLD)
    }
}

impl Palette {
    fn default_dark() -> Self {
        Self {
            text: Color::Rgb(205, 214, 244),
            subtle: Color::Rgb(108, 112, 134),
            surface0: Color::Rgb(49, 50, 68),
            blue: Color::Rgb(137, 180, 250),
            pink: Color::Rgb(245, 194, 231),
            green: Color::Rgb(166, 227, 161),
            yellow: Color::Rgb(249, 226, 175),
            cyan: Color::Rgb(148, 226, 213),
            red: Color::Rgb(243, 139, 168),
        }
    }
}
