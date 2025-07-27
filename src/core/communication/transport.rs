use crate::domain::{config::DeviceConfig, error::TermComResult};
use crate::core::communication::message::Message;
use async_trait::async_trait;
use std::collections::HashMap;

/// Transport type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransportType {
    Serial,
    Tcp,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Serial => write!(f, "serial"),
            TransportType::Tcp => write!(f, "tcp"),
        }
    }
}

/// Unified transport trait for communication abstractions
#[async_trait]
pub trait Transport: Send + Sync {
    /// Get the transport type
    fn transport_type(&self) -> TransportType;
    
    /// Create a new session for the given device
    async fn create_session(&self, device_config: &DeviceConfig) -> TermComResult<String>;
    
    /// Close an existing session
    async fn close_session(&self, session_id: &str) -> TermComResult<()>;
    
    /// Send data to a session
    async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()>;
    
    /// Send a command to a session
    async fn send_command(&self, session_id: &str, command: &str) -> TermComResult<()>;
    
    /// Receive the next message from any session
    async fn receive_message(&mut self) -> Option<Message>;
    
    /// Check if a session is connected
    async fn is_session_connected(&self, session_id: &str) -> bool;
    
    /// Get session information
    async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo>;
    
    /// List all active sessions
    async fn list_sessions(&self) -> Vec<SessionInfo>;
    
    /// Close all sessions
    async fn close_all_sessions(&self) -> TermComResult<()>;
    
    /// Get session count
    async fn get_session_count(&self) -> usize;
    
    /// Get maximum allowed sessions
    fn get_max_sessions(&self) -> usize;
}

/// Session information structure
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub device_name: String,
    pub transport_type: TransportType,
    pub status: SessionStatus,
    pub created_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

/// Session status enumeration
#[derive(Debug, Clone)]
pub enum SessionStatus {
    Connected,
    Disconnected,
    Error(String),
    Connecting,
    Closing,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Connected => write!(f, "Connected"),
            SessionStatus::Disconnected => write!(f, "Disconnected"),
            SessionStatus::Error(err) => write!(f, "Error: {}", err),
            SessionStatus::Connecting => write!(f, "Connecting"),
            SessionStatus::Closing => write!(f, "Closing"),
        }
    }
}

/// Transport registry for managing multiple transport types
pub struct TransportRegistry {
    transports: HashMap<TransportType, Box<dyn Transport>>,
}

impl TransportRegistry {
    /// Create a new transport registry
    pub fn new() -> Self {
        Self {
            transports: HashMap::new(),
        }
    }
    
    /// Register a transport
    pub fn register_transport(&mut self, transport: Box<dyn Transport>) {
        let transport_type = transport.transport_type();
        self.transports.insert(transport_type, transport);
    }
    
    /// Get a transport by type
    pub fn get_transport(&mut self, transport_type: &TransportType) -> Option<&mut Box<dyn Transport>> {
        self.transports.get_mut(transport_type)
    }
    
    /// Get all available transport types
    pub fn available_transports(&self) -> Vec<TransportType> {
        self.transports.keys().cloned().collect()
    }
    
    /// Check if a transport type is available
    pub fn has_transport(&self, transport_type: &TransportType) -> bool {
        self.transports.contains_key(transport_type)
    }
    
    /// Create a session using the appropriate transport
    pub async fn create_session(&mut self, device_config: &DeviceConfig) -> TermComResult<(String, TransportType)> {
        let transport_type = match &device_config.connection {
            crate::domain::config::ConnectionConfig::Serial { .. } => TransportType::Serial,
            crate::domain::config::ConnectionConfig::Tcp { .. } => TransportType::Tcp,
        };
        
        if let Some(transport) = self.get_transport(&transport_type) {
            let session_id = transport.create_session(device_config).await?;
            Ok((session_id, transport_type))
        } else {
            Err(crate::domain::error::TermComError::Communication {
                message: format!("Transport type {:?} not available", transport_type),
            })
        }
    }
    
    /// Close a session by finding the appropriate transport
    pub async fn close_session(&mut self, session_id: &str) -> TermComResult<()> {
        // Try to find the session in all transports
        for transport in self.transports.values_mut() {
            if transport.is_session_connected(session_id).await {
                return transport.close_session(session_id).await;
            }
        }
        
        Err(crate::domain::error::TermComError::Communication {
            message: format!("Session '{}' not found in any transport", session_id),
        })
    }
    
    /// Send data to a session
    pub async fn send_data(&mut self, session_id: &str, data: Vec<u8>) -> TermComResult<()> {
        // Try to find the session in all transports
        for transport in self.transports.values_mut() {
            if transport.is_session_connected(session_id).await {
                return transport.send_data(session_id, data).await;
            }
        }
        
        Err(crate::domain::error::TermComError::Communication {
            message: format!("Session '{}' not found in any transport", session_id),
        })
    }
    
    /// Get session information from all transports
    pub async fn get_session_info(&mut self, session_id: &str) -> Option<SessionInfo> {
        for transport in self.transports.values_mut() {
            if let Some(info) = transport.get_session_info(session_id).await {
                return Some(info);
            }
        }
        None
    }
    
    /// List all sessions from all transports
    pub async fn list_all_sessions(&mut self) -> Vec<SessionInfo> {
        let mut all_sessions = Vec::new();
        
        for transport in self.transports.values_mut() {
            all_sessions.extend(transport.list_sessions().await);
        }
        
        all_sessions
    }
    
    /// Close all sessions in all transports
    pub async fn close_all_sessions(&mut self) -> TermComResult<()> {
        let mut errors = Vec::new();
        
        for transport in self.transports.values_mut() {
            if let Err(e) = transport.close_all_sessions().await {
                errors.push(e.to_string());
            }
        }
        
        if !errors.is_empty() {
            return Err(crate::domain::error::TermComError::Communication {
                message: format!("Errors closing sessions: {}", errors.join(", ")),
            });
        }
        
        Ok(())
    }
}

impl Default for TransportRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock transport for testing
    struct MockTransport {
        transport_type: TransportType,
        sessions: HashMap<String, String>, // session_id -> device_name
    }
    
    impl MockTransport {
        fn new(transport_type: TransportType) -> Self {
            Self {
                transport_type,
                sessions: HashMap::new(),
            }
        }
    }
    
    #[async_trait]
    impl Transport for MockTransport {
        fn transport_type(&self) -> TransportType {
            self.transport_type.clone()
        }
        
        async fn create_session(&self, _device_config: &DeviceConfig) -> TermComResult<String> {
            let session_id = format!("{}_{}", self.transport_type, uuid::Uuid::new_v4().simple());
            Ok(session_id)
        }
        
        async fn close_session(&self, _session_id: &str) -> TermComResult<()> {
            Ok(())
        }
        
        async fn send_data(&self, _session_id: &str, _data: Vec<u8>) -> TermComResult<()> {
            Ok(())
        }
        
        async fn send_command(&self, _session_id: &str, _command: &str) -> TermComResult<()> {
            Ok(())
        }
        
        async fn receive_message(&mut self) -> Option<Message> {
            None
        }
        
        async fn is_session_connected(&self, _session_id: &str) -> bool {
            true
        }
        
        async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
            Some(SessionInfo {
                id: session_id.to_string(),
                device_name: "test_device".to_string(),
                transport_type: self.transport_type.clone(),
                status: SessionStatus::Connected,
                created_at: std::time::SystemTime::now(),
                last_activity: std::time::SystemTime::now(),
                bytes_sent: 0,
                bytes_received: 0,
                messages_sent: 0,
                messages_received: 0,
            })
        }
        
        async fn list_sessions(&self) -> Vec<SessionInfo> {
            Vec::new()
        }
        
        async fn close_all_sessions(&self) -> TermComResult<()> {
            Ok(())
        }
        
        async fn get_session_count(&self) -> usize {
            self.sessions.len()
        }
        
        fn get_max_sessions(&self) -> usize {
            10
        }
    }
    
    #[test]
    fn test_transport_type_display() {
        assert_eq!(TransportType::Serial.to_string(), "serial");
        assert_eq!(TransportType::Tcp.to_string(), "tcp");
    }
    
    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Connected.to_string(), "Connected");
        assert_eq!(SessionStatus::Disconnected.to_string(), "Disconnected");
        assert_eq!(SessionStatus::Error("test".to_string()).to_string(), "Error: test");
        assert_eq!(SessionStatus::Connecting.to_string(), "Connecting");
        assert_eq!(SessionStatus::Closing.to_string(), "Closing");
    }
    
    #[tokio::test]
    async fn test_transport_registry() {
        let mut registry = TransportRegistry::new();
        
        // Register mock transports
        registry.register_transport(Box::new(MockTransport::new(TransportType::Serial)));
        registry.register_transport(Box::new(MockTransport::new(TransportType::Tcp)));
        
        assert!(registry.has_transport(&TransportType::Serial));
        assert!(registry.has_transport(&TransportType::Tcp));
        assert_eq!(registry.available_transports().len(), 2);
    }
}