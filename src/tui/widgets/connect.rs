use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::state::AppState;

pub fn render_connect_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Instructions
            Constraint::Length(3),  // Input area
            Constraint::Min(0),     // Examples
        ])
        .split(area);

    // Instructions
    render_instructions(f, chunks[0], state);
    
    // Input area
    render_connect_input(f, chunks[1], state);
    
    // Examples
    render_examples(f, chunks[2], state);
}

fn render_instructions(f: &mut Frame, area: Rect, _state: &AppState) {
    let content = vec![
        Line::from(Span::styled(
            "Create New Connection",
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from("Connection Types:"),
        Line::from("  Serial: serial <port> <baud_rate>"),
        Line::from("  TCP:    tcp <host> <port>"),
        Line::from(""),
        Line::from("Controls:"),
        Line::from("  i - Start typing connection command"),
        Line::from("  Enter - Execute connection command"),
    ];

    let instructions = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Instructions"),
        );

    f.render_widget(instructions, area);
}

fn render_connect_input(f: &mut Frame, area: Rect, state: &AppState) {
    let input_text = if state.input_mode {
        format!("Command: {}", state.input_buffer)
    } else {
        "Press 'i' to start typing a connection command".to_string()
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
                .title("Connection Command")
                .border_style(if state.input_mode && matches!(state.active_panel, crate::tui::ui::ActivePanel::Connect) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );

    f.render_widget(input, area);

    // Set cursor position when in input mode
    if state.input_mode && matches!(state.active_panel, crate::tui::ui::ActivePanel::Connect) {
        f.set_cursor(
            area.x + state.input_buffer.len() as u16 + 10, // "Command: " = 9 chars + 1
            area.y + 1,
        );
    }
}

fn render_examples(f: &mut Frame, area: Rect, _state: &AppState) {
    let content = vec![
        Line::from(Span::styled(
            "Examples:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Serial Connections:", Style::default().fg(Color::Green))),
        Line::from("  serial /dev/ttyUSB0 9600"),
        Line::from("  serial /dev/ttyACM0 115200"),
        Line::from("  serial COM3 38400"),
        Line::from(""),
        Line::from(Span::styled("TCP Connections:", Style::default().fg(Color::Blue))),
        Line::from("  tcp localhost 8080"),
        Line::from("  tcp 192.168.1.100 23"),
        Line::from("  tcp example.com 1234"),
        Line::from(""),
        Line::from(Span::styled("Common Serial Baud Rates:", Style::default().fg(Color::Yellow))),
        Line::from("  9600, 19200, 38400, 57600, 115200"),
    ];

    let examples = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Examples")
                .border_style(if matches!(_state.active_panel, crate::tui::ui::ActivePanel::Connect) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(examples, area);
}