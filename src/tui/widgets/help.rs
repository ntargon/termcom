use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::{state::AppState, ui::centered_rect};

pub fn render_help_popup(f: &mut Frame, area: Rect, _state: &AppState) {
    let popup_area = centered_rect(70, 80, area);
    
    // Clear the background
    f.render_widget(Clear, popup_area);

    let help_content = vec![
        Line::from(Span::styled(
            "TermCom Help",
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(Span::styled("Global Controls:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  h - Toggle this help screen"),
        Line::from("  q - Quit application"),
        Line::from("  Ctrl+C - Force quit"),
        Line::from("  Tab - Switch between panels"),
        Line::from("  Esc - Cancel input/close dialogs"),
        Line::from(""),
        Line::from(Span::styled("Sessions Panel:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  ↑/k - Select previous session"),
        Line::from("  ↓/j - Select next session"),
        Line::from("  Enter - Switch to chat panel"),
        Line::from("  n - Switch to connect panel"),
        Line::from("  d - Close selected session"),
        Line::from(""),
        Line::from(Span::styled("Chat Panel:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  i - Start typing message"),
        Line::from("  Enter - Send message (in input mode)"),
        Line::from("  ↑/k - Scroll up in chat history"),
        Line::from("  ↓/j - Scroll down in chat history"),
        Line::from(""),
        Line::from(Span::styled("Connect Panel:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  i - Start typing connection command"),
        Line::from("  Enter - Execute connection command"),
        Line::from(""),
        Line::from(Span::styled("Connection Commands:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  serial <port> <baud> - Connect to serial device"),
        Line::from("  tcp <host> <port> - Connect to TCP server"),
        Line::from(""),
        Line::from(Span::styled("Examples:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  serial /dev/ttyUSB0 9600"),
        Line::from("  tcp localhost 8080"),
        Line::from(""),
        Line::from("Press 'h' or 'Esc' to close this help"),
    ];

    let help = Paragraph::new(help_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(help, popup_area);
}