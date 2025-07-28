use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};
use std::time::SystemTime;

use crate::{
    tui::{state::AppState, ui::ViewMode},
};

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub content: String,
    pub timestamp: SystemTime,
    pub is_sent: bool,
}

pub fn render_main_panel(f: &mut Frame, area: Rect, state: &AppState) {
    match state.view_mode {
        ViewMode::Chat => render_chat_view(f, area, state),
        ViewMode::Command => render_command_view(f, area, state),
    }
}


fn render_chat_view(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(connection) = state.get_connection() {
        let messages: Vec<_> = connection.messages.iter().rev().take(100).collect();
        
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

        let title = format!("Chat - {}", connection.name);
        let chat_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Color::Blue))
            );

        f.render_widget(chat_list, area);
    } else {
        let no_connection_text = vec![
            Line::from("No active connection"),
            Line::from(""),
            Line::from("Commands to connect:"),
            Line::from("  :serial /dev/ttyUSB0 9600  - Connect to serial"),
            Line::from("  :tcp localhost 8080       - Connect to TCP"),
            Line::from(""),
            Line::from("Press ':' to enter command mode"),
        ];
        
        let no_connection = Paragraph::new(no_connection_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Chat")
                    .border_style(Style::default().fg(Color::Blue))
            )
            .style(Style::default().fg(Color::DarkGray));
        
        f.render_widget(no_connection, area);
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

