use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use super::{
    state::AppState,
    widgets::{
        main::render_main_panel,
        help::render_help_popup,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Chat,
    Command,
}

impl std::fmt::Display for ViewMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewMode::Chat => write!(f, "Chat"),
            ViewMode::Command => write!(f, "Command"),
        }
    }
}

pub fn draw_ui(f: &mut Frame, state: &mut AppState) {
    let size = f.size();
    state.terminal_size = (size.width, size.height);

    // Simple 3-segment layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Input + Status bar
        ])
        .split(size);

    // Header bar - connection status and session info
    render_header_bar(f, chunks[0], state);

    // Main content area
    render_main_panel(f, chunks[1], state);

    // Input and status bar
    render_input_status_bar(f, chunks[2], state);

    // Help popup (if active)
    if state.show_help {
        render_help_popup(f, size, state);
    }
}

fn render_header_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let status_text = if let Some(connection) = state.get_connection() {
        format!("[{}] {} | Mode: {}", 
            if connection.connected { "●" } else { "○" },
            connection.name,
            state.view_mode
        )
    } else {
        format!("No connection | Mode: {} | Press ':' to connect", state.view_mode)
    };

    let header = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Gray));
    
    f.render_widget(header, area);
}

fn render_input_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Input line
            Constraint::Length(1), // Status line
            Constraint::Length(1), // Help hint
        ])
        .split(area);

    // Input line
    let input_text = if state.input_mode {
        format!("» {}", state.input_buffer)
    } else {
        match state.view_mode {
            ViewMode::Chat => {
                if state.get_connection().is_some() {
                    "Press 'i' to send message".to_string()
                } else {
                    "Press ':' to connect".to_string()
                }
            },
            ViewMode::Command => "Type command and press Enter".to_string(),
        }
    };

    let input_style = if state.input_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input_widget = Paragraph::new(input_text).style(input_style);
    f.render_widget(input_widget, chunks[0]);

    // Status line
    if let Some(msg) = &state.status_message {
        let status = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(status, chunks[1]);
    }

    // Help hint
    let help_text = "q: quit | h: help | Tab: Chat ↔ Command | Esc: cancel";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);

    // Set cursor position when in input mode
    if state.input_mode {
        f.set_cursor(
            chunks[0].x + state.input_buffer.len() as u16 + 2, // "» " = 2 chars
            chunks[0].y,
        );
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