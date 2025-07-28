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
        Line::from("  Tab      - Switch modes (Sessions → Chat → Command)"),
        Line::from(""),
        Line::from("Sessions Mode:"),
        Line::from("  Enter    - Open selected session"),
        Line::from("  c        - Create new connection"),
        Line::from(""),
        Line::from("Chat Mode:"),
        Line::from("  i        - Type message"),
        Line::from("  Enter    - Send message (in input mode)"),
        Line::from(""),
        Line::from("Command Mode:"),
        Line::from("  :serial <port> <baud>  - Connect serial"),
        Line::from("  :tcp <host> <port>     - Connect TCP"),
        Line::from("  :close                 - Close session"),
        Line::from("  :quit                  - Quit app"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  :serial /dev/ttyUSB0 9600"),
        Line::from("  :tcp localhost 8080"),
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