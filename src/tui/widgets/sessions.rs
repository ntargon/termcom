use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::state::AppState;

pub fn render_sessions_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Session list
            Constraint::Percentage(60), // Session details
        ])
        .split(area);

    // Session list
    render_session_list(f, chunks[0], state);
    
    // Session details
    render_session_details(f, chunks[1], state);
}

fn render_session_list(f: &mut Frame, area: Rect, state: &AppState) {
    let sessions = state.get_session_list();
    
    let items: Vec<ListItem> = sessions
        .iter()
        .map(|(session_id, session_state)| {
            let status_icon = if session_state.connected { "●" } else { "○" };
            let status_color = if session_state.connected { Color::Green } else { Color::Red };
            
            let content = vec![Line::from(vec![
                Span::styled(status_icon, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::raw(&session_state.name),
            ])];
            
            let mut item = ListItem::new(content);
            
            // Highlight selected session
            if state.selected_session.as_ref() == Some(session_id) {
                item = item.style(Style::default().bg(Color::Blue).fg(Color::White));
            }
            
            item
        })
        .collect();

    let title = format!("Sessions ({})", sessions.len());
    let sessions_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(if matches!(state.active_panel, crate::tui::ui::ActivePanel::Sessions) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );

    f.render_widget(sessions_list, area);
}

fn render_session_details(f: &mut Frame, area: Rect, state: &AppState) {
    let content = if let Some(session_state) = state.get_selected_session() {
        vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&session_state.name),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", session_state.session_type)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if session_state.connected { "Connected" } else { "Disconnected" },
                    Style::default().fg(if session_state.connected { Color::Green } else { Color::Red }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Messages: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", session_state.messages.len())),
            ]),
            Line::from(""),
            Line::from(Span::styled("Configuration:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(session_state.config_info.clone()),
            Line::from(""),
            Line::from("Controls:"),
            Line::from("  ↑/k - Select previous session"),
            Line::from("  ↓/j - Select next session"),
            Line::from("  Enter - Switch to chat"),
            Line::from("  n - New connection"),
            Line::from("  d - Close session"),
        ]
    } else {
        vec![
            Line::from("No session selected"),
            Line::from(""),
            Line::from("Controls:"),
            Line::from("  n - New connection"),
            Line::from("  h - Show help"),
            Line::from("  q - Quit"),
        ]
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Session Details"),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(details, area);
}