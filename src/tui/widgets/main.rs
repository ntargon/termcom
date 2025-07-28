use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};
use std::time::SystemTime;

use crate::{
    core::session::SessionId,
    tui::{state::AppState, ui::ViewMode},
};

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub content: String,
    pub timestamp: SystemTime,
    pub is_sent: bool,
    pub session_id: SessionId,
}

pub fn render_main_panel(f: &mut Frame, area: Rect, state: &AppState) {
    match state.view_mode {
        ViewMode::SessionList => render_session_list(f, area, state),
        ViewMode::Chat => render_chat_view(f, area, state),
        ViewMode::Command => render_command_view(f, area, state),
    }
}

fn render_session_list(f: &mut Frame, area: Rect, state: &AppState) {
    let sessions = state.get_session_list();
    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(_i, (session_id, session_state))| {
            let is_selected = state.selected_session.as_ref() == Some(session_id);
            let status_icon = if session_state.connected { "●" } else { "○" };
            let status_color = if session_state.connected { Color::Green } else { Color::Red };
            
            let content = vec![Line::from(vec![
                Span::styled(status_icon, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(
                    &session_state.name,
                    if is_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    }
                ),
                Span::raw(" - "),
                Span::styled(
                    &session_state.config_info,
                    Style::default().fg(Color::DarkGray)
                ),
            ])];
            
            ListItem::new(content)
        })
        .collect();

    let title = format!("Sessions ({})", sessions.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Blue))
        );

    f.render_widget(list, area);

    // Show empty state if no sessions
    if sessions.is_empty() {
        let help_text = vec![
            Line::from("No active sessions"),
            Line::from(""),
            Line::from("Press 'c' to create a new connection"),
            Line::from("Commands:"),
            Line::from("  :serial /dev/ttyUSB0 9600  - Connect to serial"),
            Line::from("  :tcp localhost 8080       - Connect to TCP"),
        ];
        
        let help_area = centered_rect(60, 50, area);
        f.render_widget(Clear, help_area);
        
        let help_widget = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Getting Started")
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .style(Style::default().fg(Color::Gray));
        
        f.render_widget(help_widget, help_area);
    }
}

fn render_chat_view(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(session_state) = state.get_selected_session() {
        let messages: Vec<_> = session_state.messages.iter().rev().take(100).collect();
        
        let items: Vec<ListItem> = messages
            .iter()
            .rev()
            .map(|message| {
                let timestamp = message.timestamp
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                let time_str = format_timestamp(timestamp);
                let direction = if message.is_sent { "→" } else { "←" };
                let color = if message.is_sent { Color::Cyan } else { Color::Green };
                
                let content = vec![Line::from(vec![
                    Span::styled(
                        format!("[{}] {}", time_str, direction),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(" "),
                    Span::styled(&message.content, Style::default().fg(color)),
                ])];
                
                ListItem::new(content)
            })
            .collect();

        let title = format!("Chat - {}", session_state.name);
        let chat_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Color::Blue))
            );

        f.render_widget(chat_list, area);
    } else {
        let no_session = Paragraph::new("No session selected\n\nPress Tab to switch to session list")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Chat")
                    .border_style(Style::default().fg(Color::Red))
            )
            .style(Style::default().fg(Color::DarkGray));
        
        f.render_widget(no_session, area);
    }
}

fn render_command_view(f: &mut Frame, area: Rect, _state: &AppState) {
    let help_text = vec![
        Line::from("Command Mode"),
        Line::from(""),
        Line::from("Available commands:"),
        Line::from("  :serial <port> <baud>     - Connect to serial port"),
        Line::from("  :tcp <host> <port>        - Connect to TCP server"),
        Line::from("  :close                    - Close current session"),
        Line::from("  :quit                     - Quit application"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  :serial /dev/ttyUSB0 9600"),
        Line::from("  :tcp localhost 8080"),
        Line::from("  :tcp 192.168.1.100 23"),
    ];
    
    let command_help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Command Mode")
                .border_style(Style::default().fg(Color::Blue))
        )
        .style(Style::default().fg(Color::Gray));
    
    f.render_widget(command_help, area);
}

fn format_timestamp(timestamp: u64) -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let diff = now.saturating_sub(timestamp);
    
    if diff < 60 {
        format!("{}s", diff)
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else if diff < 86400 {
        format!("{}h", diff / 3600)
    } else {
        format!("{}d", diff / 86400)
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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