use std::collections::HashMap;
use std::time::SystemTime;

use crate::{
    core::session::{SessionId, SessionType, SessionManager, SessionConfig},
    domain::{error::TermComError, config::{DeviceConfig, ConnectionConfig}},
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

    pub async fn create_serial_session(&mut self, session_manager: &SessionManager, port: String, baud_rate: u32) -> Result<SessionId, TermComError> {
        // Create device config for serial connection
        let device_config = DeviceConfig {
            name: format!("Serial {}", port),
            description: format!("Serial connection on {} at {} baud", port, baud_rate),
            connection: ConnectionConfig::Serial {
                port: port.clone(),
                baud_rate,
                data_bits: 8,
                stop_bits: 1,
                parity: crate::domain::config::ParityConfig::None,
                flow_control: crate::domain::config::FlowControlConfig::None,
            },
            commands: Vec::new(),
        };

        // Create session config
        let session_config = SessionConfig {
            name: format!("Serial {}", port),
            session_type: SessionType::Interactive,
            device_config,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
            timeout_ms: 5000,
            max_history_size: 1000,
            log_activities: true,
            tags: vec!["serial".to_string()],
            properties: std::collections::HashMap::new(),
        };

        // Create session using the real session manager
        let session_id = session_manager.create_session(session_config).await?;
        
        // Create TUI session state for tracking
        let session_state = SessionState {
            id: session_id.clone(),
            name: format!("Serial {}", port),
            session_type: SessionType::Interactive,
            messages: Vec::new(),
            connected: true,
            last_activity: SystemTime::now(),
            config_info: format!("Port: {}, Baud: {}", port, baud_rate),
        };

        self.sessions.insert(session_id.clone(), session_state);
        self.selected_session = Some(session_id.clone());
        self.status_message = Some(format!("Connected to serial port: {}", port));

        Ok(session_id)
    }

    pub async fn create_tcp_session(&mut self, session_manager: &SessionManager, host: String, port: u16) -> Result<SessionId, TermComError> {
        // Create device config for TCP connection
        let device_config = DeviceConfig {
            name: format!("TCP {}:{}", host, port),
            description: format!("TCP connection to {}:{}", host, port),
            connection: ConnectionConfig::Tcp {
                host: host.clone(),
                port,
                timeout_ms: 3000,
                keep_alive: true,
            },
            commands: Vec::new(),
        };

        // Create session config
        let session_config = SessionConfig {
            name: format!("TCP {}:{}", host, port),
            session_type: SessionType::Interactive,
            device_config,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
            timeout_ms: 5000,
            max_history_size: 1000,
            log_activities: true,
            tags: vec!["tcp".to_string()],
            properties: std::collections::HashMap::new(),
        };

        // Create session using the real session manager
        let session_id = session_manager.create_session(session_config).await?;
        
        // Create TUI session state for tracking
        let session_state = SessionState {
            id: session_id.clone(),
            name: format!("TCP {}:{}", host, port),
            session_type: SessionType::Interactive,
            messages: Vec::new(),
            connected: true,
            last_activity: SystemTime::now(),
            config_info: format!("Host: {}, Port: {}", host, port),
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