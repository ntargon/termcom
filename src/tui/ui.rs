use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Tabs},
    Frame,
};

use super::{
    state::AppState,
    widgets::{
        chat::render_chat_panel,
        connect::render_connect_panel,
        help::render_help_popup,
        sessions::render_sessions_panel,
        status::render_status_bar,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Sessions,
    Chat,
    Connect,
}

impl std::fmt::Display for ActivePanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivePanel::Sessions => write!(f, "Sessions"),
            ActivePanel::Chat => write!(f, "Chat"),
            ActivePanel::Connect => write!(f, "Connect"),
        }
    }
}

pub fn draw_ui(f: &mut Frame, state: &mut AppState) {
    let size = f.size();
    state.terminal_size = (size.width, size.height);

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    // Tab bar
    let tabs = Tabs::new(vec!["Sessions", "Chat", "Connect"])
        .block(Block::default().borders(Borders::ALL).title("TermCom"))
        .select(match state.active_panel {
            ActivePanel::Sessions => 0,
            ActivePanel::Chat => 1,
            ActivePanel::Connect => 2,
        })
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue)
                .fg(Color::White),
        );
    f.render_widget(tabs, chunks[0]);

    // Main content area
    match state.active_panel {
        ActivePanel::Sessions => render_sessions_panel(f, chunks[1], state),
        ActivePanel::Chat => render_chat_panel(f, chunks[1], state),
        ActivePanel::Connect => render_connect_panel(f, chunks[1], state),
    }

    // Status bar
    render_status_bar(f, chunks[2], state);

    // Help popup (if active)
    if state.show_help {
        render_help_popup(f, size, state);
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}