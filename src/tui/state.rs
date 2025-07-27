use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use crate::{
    core::session::{SessionId, SessionType},
    domain::error::TermComError,
};

use super::{ui::ActivePanel, widgets::chat::ChatMessage};

#[derive(Debug)]
pub struct AppState {
    pub active_panel: ActivePanel,
    pub input_mode: bool,
    pub input_buffer: String,
    pub terminal_size: (u16, u16),
    pub selected_session: Option<SessionId>,
    pub sessions: HashMap<SessionId, SessionState>,
    pub status_message: Option<String>,
    pub show_help: bool,
}

#[derive(Debug)]
pub struct SessionState {
    pub id: SessionId,
    pub name: String,
    pub session_type: SessionType,
    pub messages: Vec<ChatMessage>,
    pub connected: bool,
    pub last_activity: SystemTime,
    pub config_info: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            active_panel: ActivePanel::Sessions,
            input_mode: false,
            input_buffer: String::new(),
            terminal_size: (80, 24),
            selected_session: None,
            sessions: HashMap::new(),
            status_message: Some("Welcome to TermCom TUI! Press 'h' for help.".to_string()),
            show_help: false,
        }
    }

    pub async fn create_serial_session(&mut self, port: String, baud_rate: u32) -> Result<SessionId, TermComError> {
        let session_id = Uuid::new_v4().to_string();
        let name = format!("Serial {}", port);
        let config_info = format!("Port: {}, Baud: {}", port, baud_rate);
        
        // TODO: Implement actual serial connection
        // For now, just create a mock session
        
        let session_state = SessionState {
            id: session_id.clone(),
            name,
            session_type: SessionType::Interactive,
            messages: Vec::new(),
            connected: true,
            last_activity: SystemTime::now(),
            config_info,
        };

        self.sessions.insert(session_id.clone(), session_state);
        self.selected_session = Some(session_id.clone());
        self.status_message = Some(format!("Connected to serial port: {}", port));

        Ok(session_id)
    }

    pub async fn create_tcp_session(&mut self, host: String, port: u16) -> Result<SessionId, TermComError> {
        let session_id = Uuid::new_v4().to_string();
        let name = format!("TCP {}:{}", host, port);
        let config_info = format!("Host: {}, Port: {}", host, port);
        
        // TODO: Implement actual TCP connection
        // For now, just create a mock session
        
        let session_state = SessionState {
            id: session_id.clone(),
            name,
            session_type: SessionType::Interactive,
            messages: Vec::new(),
            connected: true,
            last_activity: SystemTime::now(),
            config_info,
        };

        self.sessions.insert(session_id.clone(), session_state);
        self.selected_session = Some(session_id.clone());
        self.status_message = Some(format!("Connected to TCP: {}:{}", host, port));

        Ok(session_id)
    }

    pub async fn add_message(&mut self, session_id: SessionId, content: String, is_sent: bool) -> Result<(), TermComError> {
        if let Some(session_state) = self.sessions.get_mut(&session_id) {
            let message = ChatMessage {
                content,
                timestamp: SystemTime::now(),
                is_sent,
                session_id: session_id.clone(),
            };

            session_state.messages.push(message);
            session_state.last_activity = SystemTime::now();

            // TODO: Implement actual message sending
            // For now, just simulate echo if it's sent
            if is_sent {
                // Simulate echo after a short delay
                let echo_message = ChatMessage {
                    content: format!("Echo: {}", session_state.messages.last().unwrap().content),
                    timestamp: SystemTime::now(),
                    is_sent: false,
                    session_id: session_id.clone(),
                };
                session_state.messages.push(echo_message);
            }
        }

        Ok(())
    }

    pub async fn close_session(&mut self, session_id: SessionId) -> Result<(), TermComError> {
        if let Some(session_state) = self.sessions.remove(&session_id) {
            // TODO: Implement actual session closing
            
            if self.selected_session.as_ref() == Some(&session_id) {
                self.selected_session = self.sessions.keys().next().cloned();
            }
            
            self.status_message = Some(format!("Closed session: {}", session_state.name));
        }

        Ok(())
    }

    pub async fn update_sessions(&mut self) -> Result<(), TermComError> {
        // TODO: Implement actual session updates
        // For now, this is a no-op since we're using mock sessions
        Ok(())
    }

    pub fn get_session_list(&self) -> Vec<(&SessionId, &SessionState)> {
        let mut sessions: Vec<_> = self.sessions.iter().collect();
        sessions.sort_by_key(|(_, state)| std::cmp::Reverse(state.last_activity));
        sessions
    }

    pub fn get_selected_session(&self) -> Option<&SessionState> {
        self.selected_session.as_ref().and_then(|id| self.sessions.get(id))
    }

    pub fn get_selected_session_mut(&mut self) -> Option<&mut SessionState> {
        self.selected_session.as_ref().and_then(|id| self.sessions.get_mut(id))
    }

    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}