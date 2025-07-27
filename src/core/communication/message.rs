use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Unified message representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: SystemTime,
    pub session_id: String,
    pub device_name: String,
    pub message_type: MessageType,
    pub data: Vec<u8>,
    pub metadata: MessageMetadata,
}

/// Message type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Data sent to device
    Sent,
    /// Data received from device
    Received,
    /// System event (connection, disconnection, etc.)
    System,
    /// Error message
    Error,
    /// Command execution
    Command,
    /// Response to command
    Response,
}

/// Additional message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Transport type used
    pub transport: String,
    /// Size of the message in bytes
    pub size: usize,
    /// Duration for command/response pairs
    pub duration_ms: Option<u64>,
    /// Sequence number for ordering
    pub sequence: u64,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: std::collections::HashMap<String, String>,
}

impl Message {
    /// Create a new message with basic information
    pub fn new(
        session_id: String,
        device_name: String,
        message_type: MessageType,
        data: Vec<u8>,
        transport: String,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let size = data.len();
        
        Self {
            id,
            timestamp: SystemTime::now(),
            session_id,
            device_name,
            message_type,
            data,
            metadata: MessageMetadata {
                transport,
                size,
                duration_ms: None,
                sequence: 0,
                tags: Vec::new(),
                properties: std::collections::HashMap::new(),
            },
        }
    }
    
    /// Create a sent message
    pub fn sent(
        session_id: String,
        device_name: String,
        data: Vec<u8>,
        transport: String,
    ) -> Self {
        Self::new(session_id, device_name, MessageType::Sent, data, transport)
    }
    
    /// Create a received message
    pub fn received(
        session_id: String,
        device_name: String,
        data: Vec<u8>,
        transport: String,
    ) -> Self {
        Self::new(session_id, device_name, MessageType::Received, data, transport)
    }
    
    /// Create a system message
    pub fn system(
        session_id: String,
        device_name: String,
        message: String,
        transport: String,
    ) -> Self {
        Self::new(
            session_id,
            device_name,
            MessageType::System,
            message.into_bytes(),
            transport,
        )
    }
    
    /// Create an error message
    pub fn error(
        session_id: String,
        device_name: String,
        error: String,
        transport: String,
    ) -> Self {
        Self::new(
            session_id,
            device_name,
            MessageType::Error,
            error.into_bytes(),
            transport,
        )
    }
    
    /// Create a command message
    pub fn command(
        session_id: String,
        device_name: String,
        command: String,
        transport: String,
    ) -> Self {
        Self::new(
            session_id,
            device_name,
            MessageType::Command,
            command.into_bytes(),
            transport,
        )
    }
    
    /// Create a response message
    pub fn response(
        session_id: String,
        device_name: String,
        response: Vec<u8>,
        transport: String,
        duration_ms: Option<u64>,
    ) -> Self {
        let mut msg = Self::new(
            session_id,
            device_name,
            MessageType::Response,
            response,
            transport,
        );
        msg.metadata.duration_ms = duration_ms;
        msg
    }
    
    /// Get message data as string (if valid UTF-8)
    pub fn data_as_string(&self) -> Option<String> {
        String::from_utf8(self.data.clone()).ok()
    }
    
    /// Get message data as hex string
    pub fn data_as_hex(&self) -> String {
        self.data
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Add a tag to the message
    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
        }
    }
    
    /// Add a property to the message
    pub fn add_property(&mut self, key: String, value: String) {
        self.metadata.properties.insert(key, value);
    }
    
    /// Set the sequence number
    pub fn set_sequence(&mut self, sequence: u64) {
        self.metadata.sequence = sequence;
    }
    
    /// Check if message matches a pattern
    pub fn matches_pattern(&self, pattern: &MessagePattern) -> bool {
        // Check session ID
        if let Some(ref session_id) = pattern.session_id {
            if &self.session_id != session_id {
                return false;
            }
        }
        
        // Check device name
        if let Some(ref device_name) = pattern.device_name {
            if &self.device_name != device_name {
                return false;
            }
        }
        
        // Check message type
        if let Some(ref message_type) = pattern.message_type {
            if !std::mem::discriminant(&self.message_type).eq(&std::mem::discriminant(message_type)) {
                return false;
            }
        }
        
        // Check transport
        if let Some(ref transport) = pattern.transport {
            if &self.metadata.transport != transport {
                return false;
            }
        }
        
        // Check tags
        if !pattern.tags.is_empty() {
            if !pattern.tags.iter().all(|tag| self.metadata.tags.contains(tag)) {
                return false;
            }
        }
        
        true
    }
}

/// Pattern for filtering messages
#[derive(Debug, Clone, Default)]
pub struct MessagePattern {
    pub session_id: Option<String>,
    pub device_name: Option<String>,
    pub message_type: Option<MessageType>,
    pub transport: Option<String>,
    pub tags: Vec<String>,
}

impl MessagePattern {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    
    pub fn with_device_name(mut self, device_name: String) -> Self {
        self.device_name = Some(device_name);
        self
    }
    
    pub fn with_message_type(mut self, message_type: MessageType) -> Self {
        self.message_type = Some(message_type);
        self
    }
    
    pub fn with_transport(mut self, transport: String) -> Self {
        self.transport = Some(transport);
        self
    }
    
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_creation() {
        let msg = Message::sent(
            "session1".to_string(),
            "device1".to_string(),
            b"hello".to_vec(),
            "serial".to_string(),
        );
        
        assert_eq!(msg.session_id, "session1");
        assert_eq!(msg.device_name, "device1");
        assert!(matches!(msg.message_type, MessageType::Sent));
        assert_eq!(msg.data, b"hello");
        assert_eq!(msg.metadata.transport, "serial");
        assert_eq!(msg.metadata.size, 5);
    }
    
    #[test]
    fn test_message_data_conversion() {
        let msg = Message::sent(
            "session1".to_string(),
            "device1".to_string(),
            b"hello".to_vec(),
            "serial".to_string(),
        );
        
        assert_eq!(msg.data_as_string(), Some("hello".to_string()));
        assert_eq!(msg.data_as_hex(), "68 65 6c 6c 6f");
    }
    
    #[test]
    fn test_message_pattern_matching() {
        let mut msg = Message::sent(
            "session1".to_string(),
            "device1".to_string(),
            b"hello".to_vec(),
            "serial".to_string(),
        );
        msg.add_tag("test".to_string());
        
        let pattern = MessagePattern::new()
            .with_session_id("session1".to_string())
            .with_message_type(MessageType::Sent)
            .with_tag("test".to_string());
        
        assert!(msg.matches_pattern(&pattern));
        
        let wrong_pattern = MessagePattern::new()
            .with_session_id("session2".to_string());
        
        assert!(!msg.matches_pattern(&wrong_pattern));
    }
    
    #[test]
    fn test_message_metadata() {
        let mut msg = Message::command(
            "session1".to_string(),
            "device1".to_string(),
            "GET_STATUS".to_string(),
            "tcp".to_string(),
        );
        
        msg.add_tag("important".to_string());
        msg.add_property("priority".to_string(), "high".to_string());
        msg.set_sequence(42);
        
        assert!(msg.metadata.tags.contains(&"important".to_string()));
        assert_eq!(msg.metadata.properties.get("priority"), Some(&"high".to_string()));
        assert_eq!(msg.metadata.sequence, 42);
    }
}