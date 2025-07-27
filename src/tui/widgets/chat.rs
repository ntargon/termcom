use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::SystemTime;

use crate::{core::session::SessionId, tui::state::AppState};

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub content: String,
    pub timestamp: SystemTime,
    pub is_sent: bool,
    pub session_id: SessionId,
}

pub fn render_chat_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Chat messages
            Constraint::Length(3), // Input area
        ])
        .split(area);

    // Chat messages
    render_chat_messages(f, chunks[0], state);
    
    // Input area
    render_chat_input(f, chunks[1], state);
}

fn render_chat_messages(f: &mut Frame, area: Rect, state: &AppState) {
    let messages = if let Some(session_state) = state.get_selected_session() {
        session_state.messages.iter().rev().take(100).collect::<Vec<_>>()
    } else {
        Vec::new()
    };

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
                    Style::default().fg(Color::Gray),
                ),
                Span::raw(" "),
                Span::styled(&message.content, Style::default().fg(color)),
            ])];
            
            ListItem::new(content)
        })
        .collect();

    let title = if let Some(session_state) = state.get_selected_session() {
        format!("Chat - {}", session_state.name)
    } else {
        "Chat - No session selected".to_string()
    };

    let chat_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(if matches!(state.active_panel, crate::tui::ui::ActivePanel::Chat) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );

    f.render_widget(chat_list, area);
}

fn render_chat_input(f: &mut Frame, area: Rect, state: &AppState) {
    let input_text = if state.input_mode {
        format!("Message: {}", state.input_buffer)
    } else {
        "Press 'i' to start typing a message".to_string()
    };

    let style = if state.input_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let input = Paragraph::new(input_text)
        .style(style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .border_style(if state.input_mode {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );

    f.render_widget(input, area);

    // Set cursor position when in input mode
    if state.input_mode {
        f.set_cursor(
            area.x + state.input_buffer.len() as u16 + 10, // "Message: " = 9 chars + 1
            area.y + 1,
        );
    }
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