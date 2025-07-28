use std::time::SystemTime;

use crate::{
    core::session::{SessionManager, SessionConfig},
    domain::{error::TermComError, config::{DeviceConfig, ConnectionConfig}},
};

use super::{ui::ViewMode, widgets::main::ChatMessage};

#[derive(Debug)]
pub struct AppState {
    pub view_mode: ViewMode,
    pub input_mode: bool,
    pub input_buffer: String,
    pub terminal_size: (u16, u16),
    pub connection: Option<Connection>,
    pub status_message: Option<String>,
    pub show_help: bool,
}

#[derive(Debug)]
pub struct Connection {
    pub name: String,
    pub config_info: String,
    pub connected: bool,
    pub messages: Vec<ChatMessage>,
    pub last_activity: SystemTime,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            view_mode: ViewMode::Chat,
            input_mode: false,
            input_buffer: String::new(),
            terminal_size: (80, 24),
            connection: None,
            status_message: Some("Welcome to TermCom TUI! Press ':' to connect or 'h' for help.".to_string()),
            show_help: false,
        }
    }

    pub async fn create_serial_connection(&mut self, _session_manager: &SessionManager, port: String, baud_rate: u32) -> Result<(), TermComError> {
        // Close existing connection if any
        if self.connection.is_some() {
            self.close_connection().await?;
        }

        // TODO: Implement actual serial connection
        // For now, create a mock connection
        let connection = Connection {
            name: format!("Serial {}", port),
            config_info: format!("Port: {}, Baud: {}", port, baud_rate),
            connected: true,
            messages: Vec::new(),
            last_activity: SystemTime::now(),
        };

        self.connection = Some(connection);
        self.status_message = Some(format!("Connected to serial port: {}", port));

        Ok(())
    }

    pub async fn create_tcp_connection(&mut self, _session_manager: &SessionManager, host: String, port: u16) -> Result<(), TermComError> {
        // Close existing connection if any
        if self.connection.is_some() {
            self.close_connection().await?;
        }

        // TODO: Implement actual TCP connection
        // For now, create a mock connection
        let connection = Connection {
            name: format!("TCP {}:{}", host, port),
            config_info: format!("Host: {}, Port: {}", host, port),
            connected: true,
            messages: Vec::new(),
            last_activity: SystemTime::now(),
        };

        self.connection = Some(connection);
        self.status_message = Some(format!("Connected to TCP: {}:{}", host, port));

        Ok(())
    }

    pub async fn add_message(&mut self, content: String, is_sent: bool) -> Result<(), TermComError> {
        if let Some(connection) = &mut self.connection {
            let message = ChatMessage {
                content: content.clone(),
                timestamp: SystemTime::now(),
                is_sent,
            };

            connection.messages.push(message);
            connection.last_activity = SystemTime::now();

            // TODO: Implement actual message sending
            // For now, just simulate echo if it's sent
            if is_sent {
                // Simulate echo after a short delay
                let echo_message = ChatMessage {
                    content: format!("Echo: {}", content),
                    timestamp: SystemTime::now(),
                    is_sent: false,
                };
                connection.messages.push(echo_message);
            }
        }

        Ok(())
    }

    pub async fn close_connection(&mut self) -> Result<(), TermComError> {
        if let Some(connection) = self.connection.take() {
            // TODO: Implement actual connection closing
            self.status_message = Some(format!("Closed connection: {}", connection.name));
        } else {
            self.status_message = Some("No connection to close".to_string());
        }

        Ok(())
    }

    pub async fn update_connection(&mut self) -> Result<(), TermComError> {
        // TODO: Implement actual connection updates
        // For now, this is a no-op since we're using mock connections
        Ok(())
    }

    pub fn get_connection(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

    pub fn get_connection_mut(&mut self) -> Option<&mut Connection> {
        self.connection.as_mut()
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