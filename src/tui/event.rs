use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{domain::error::TermComError, core::session::SessionId};

use super::{
    state::AppState,
    ui::ActivePanel,
};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Quit,
    SwitchPanel(ActivePanel),
    ToggleInputMode,
    SendMessage(String),
    ConnectSerial { port: String, baud_rate: u32 },
    ConnectTcp { host: String, port: u16 },
    SelectSession(SessionId),
    CloseSession(SessionId),
}

pub struct EventHandler;

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_key_event(&self, key: KeyEvent, state: &mut AppState) -> Result<Option<AppEvent>, TermComError> {
        // Handle input mode
        if state.input_mode {
            return self.handle_input_mode(key, state);
        }

        // Handle help mode
        if state.show_help {
            match key.code {
                KeyCode::Char('h') | KeyCode::Esc => {
                    state.toggle_help();
                    return Ok(None);
                }
                _ => return Ok(None),
            }
        }

        // Global key bindings
        match key.code {
            KeyCode::Char('h') => {
                state.toggle_help();
                Ok(None)
            }
            KeyCode::Char('q') => Ok(Some(AppEvent::Quit)),
            KeyCode::Tab => {
                let next_panel = match state.active_panel {
                    ActivePanel::Sessions => ActivePanel::Chat,
                    ActivePanel::Chat => ActivePanel::Connect,
                    ActivePanel::Connect => ActivePanel::Sessions,
                };
                Ok(Some(AppEvent::SwitchPanel(next_panel)))
            }
            _ => self.handle_panel_specific(key, state),
        }
    }

    fn handle_input_mode(&self, key: KeyEvent, state: &mut AppState) -> Result<Option<AppEvent>, TermComError> {
        match key.code {
            KeyCode::Enter => {
                let input = state.input_buffer.trim().to_string();
                state.input_buffer.clear();
                state.input_mode = false;

                if !input.is_empty() {
                    match state.active_panel {
                        ActivePanel::Chat => {
                            return Ok(Some(AppEvent::SendMessage(input)));
                        }
                        ActivePanel::Connect => {
                            return self.handle_connect_input(input);
                        }
                        _ => {}
                    }
                }
                Ok(None)
            }
            KeyCode::Backspace => {
                state.input_buffer.pop();
                Ok(None)
            }
            KeyCode::Char(c) => {
                state.input_buffer.push(c);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn handle_connect_input(&self, input: String) -> Result<Option<AppEvent>, TermComError> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        if parts.len() < 2 {
            return Ok(None);
        }

        match parts[0].to_lowercase().as_str() {
            "serial" => {
                if parts.len() >= 3 {
                    let port = parts[1].to_string();
                    if let Ok(baud_rate) = parts[2].parse::<u32>() {
                        return Ok(Some(AppEvent::ConnectSerial { port, baud_rate }));
                    }
                }
            }
            "tcp" => {
                if parts.len() >= 3 {
                    let host = parts[1].to_string();
                    if let Ok(port) = parts[2].parse::<u16>() {
                        return Ok(Some(AppEvent::ConnectTcp { host, port }));
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    fn handle_panel_specific(&self, key: KeyEvent, state: &mut AppState) -> Result<Option<AppEvent>, TermComError> {
        match state.active_panel {
            ActivePanel::Sessions => self.handle_sessions_panel(key, state),
            ActivePanel::Chat => self.handle_chat_panel(key, state),
            ActivePanel::Connect => self.handle_connect_panel(key, state),
        }
    }

    fn handle_sessions_panel(&self, key: KeyEvent, state: &mut AppState) -> Result<Option<AppEvent>, TermComError> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                // Select previous session
                self.select_previous_session(state);
                Ok(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Select next session
                self.select_next_session(state);
                Ok(None)
            }
            KeyCode::Enter => {
                // Switch to chat panel
                Ok(Some(AppEvent::SwitchPanel(ActivePanel::Chat)))
            }
            KeyCode::Char('d') => {
                // Close selected session
                if let Some(session_id) = state.selected_session.clone() {
                    Ok(Some(AppEvent::CloseSession(session_id)))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Char('n') => {
                // Switch to connect panel
                Ok(Some(AppEvent::SwitchPanel(ActivePanel::Connect)))
            }
            _ => Ok(None),
        }
    }

    fn handle_chat_panel(&self, key: KeyEvent, state: &mut AppState) -> Result<Option<AppEvent>, TermComError> {
        match key.code {
            KeyCode::Char('i') => {
                Ok(Some(AppEvent::ToggleInputMode))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Scroll up in chat history
                Ok(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Scroll down in chat history
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn handle_connect_panel(&self, key: KeyEvent, state: &mut AppState) -> Result<Option<AppEvent>, TermComError> {
        match key.code {
            KeyCode::Char('i') => {
                Ok(Some(AppEvent::ToggleInputMode))
            }
            _ => Ok(None),
        }
    }

    fn select_previous_session(&self, state: &mut AppState) {
        let sessions = state.get_session_list();
        if sessions.is_empty() {
            return;
        }

        let current_index = if let Some(selected) = &state.selected_session {
            sessions.iter().position(|(id, _)| *id == selected).unwrap_or(0)
        } else {
            0
        };

        let new_index = if current_index == 0 {
            sessions.len() - 1
        } else {
            current_index - 1
        };

        state.selected_session = Some(sessions[new_index].0.clone());
    }

    fn select_next_session(&self, state: &mut AppState) {
        let sessions = state.get_session_list();
        if sessions.is_empty() {
            return;
        }

        let current_index = if let Some(selected) = &state.selected_session {
            sessions.iter().position(|(id, _)| *id == selected).unwrap_or(0)
        } else {
            0
        };

        let new_index = (current_index + 1) % sessions.len();
        state.selected_session = Some(sessions[new_index].0.clone());
    }
}