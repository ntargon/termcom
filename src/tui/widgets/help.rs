use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::{state::AppState, ui::centered_rect};

pub fn render_help_popup(f: &mut Frame, area: Rect, _state: &AppState) {
    let popup_area = centered_rect(70, 80, area);
    
    // Clear the background
    f.render_widget(Clear, popup_area);

    let help_content = vec![
        Line::from("TermCom - Simple TUI Help"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  q / Esc  - Quit"),
        Line::from("  h        - Toggle help"),
        Line::from("  Tab      - Switch modes (Chat ↔ Command)"),
        Line::from(""),
        Line::from("Chat Mode:"),
        Line::from("  i        - Type message (when connected)"),
        Line::from("  Enter    - Send message (in input mode)"),
        Line::from("  :        - Enter command mode"),
        Line::from(""),
        Line::from("Command Mode:"),
        Line::from("  :serial <port> <baud>  - Connect serial"),
        Line::from("  :tcp <host> <port>     - Connect TCP"),
        Line::from("  :close                 - Close connection"),
        Line::from("  :quit                  - Quit app"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  :serial /dev/ttyUSB0 9600"),
        Line::from("  :tcp localhost 8080"),
        Line::from(""),
        Line::from("Note: Only one connection at a time is supported."),
        Line::from("New connections will close the previous one."),
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