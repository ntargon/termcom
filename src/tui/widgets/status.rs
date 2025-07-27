use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::state::AppState;

pub fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let status_text = if let Some(message) = &state.status_message {
        message.clone()
    } else {
        format!(
            "Panel: {} | Sessions: {} | Help: h | Quit: q",
            state.active_panel,
            state.sessions.len()
        )
    };

    let status_style = if state.status_message.is_some() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let status = Paragraph::new(Line::from(vec![
        Span::styled(status_text, status_style),
    ]));

    f.render_widget(status, area);
}