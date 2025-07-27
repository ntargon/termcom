use crate::core::communication::{
    message::{Message, MessagePattern, MessageType},
    transport::{Transport, TransportRegistry, TransportType, SessionInfo},
};
use crate::domain::{config::DeviceConfig, error::{TermComError, TermComResult}};
use crate::infrastructure::{serial::SerialManager, tcp::TcpManager};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Central communication engine that manages all transport types
pub struct CommunicationEngine {
    registry: Arc<RwLock<TransportRegistry>>,
    message_history: Arc<RwLock<VecDeque<Message>>>,
    message_sender: mpsc::UnboundedSender<Message>,
    message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<Message>>>,
    sequence_counter: Arc<RwLock<u64>>,
    max_history_size: usize,
    running: Arc<RwLock<bool>>,
}

impl CommunicationEngine {
    /// Create a new communication engine
    pub fn new(max_history_size: usize, max_sessions_per_transport: usize) -> Self {
        let mut registry = TransportRegistry::new();
        
        // Register built-in transports
        registry.register_transport(Box::new(SerialTransportAdapter::new(max_sessions_per_transport)));
        registry.register_transport(Box::new(TcpTransportAdapter::new(max_sessions_per_transport)));
        
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        Self {
            registry: Arc::new(RwLock::new(registry)),
            message_history: Arc::new(RwLock::new(VecDeque::new())),
            message_sender,
            message_receiver: Arc::new(RwLock::new(message_receiver)),
            sequence_counter: Arc::new(RwLock::new(0)),
            max_history_size,
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Start the communication engine
    pub async fn start(&self) -> TermComResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(TermComError::Communication {
                message: "Communication engine is already running".to_string(),
            });
        }
        *running = true;
        drop(running);
        
        info!("Communication engine started");
        
        // Start message processing task
        self.start_message_processor().await;
        
        Ok(())
    }
    
    /// Stop the communication engine
    pub async fn stop(&self) -> TermComResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        *running = false;
        drop(running);
        
        // Close all sessions
        let mut registry = self.registry.write().await;
        registry.close_all_sessions().await?;
        
        info!("Communication engine stopped");
        Ok(())
    }
    
    /// Create a new communication session
    pub async fn create_session(&self, device_config: &DeviceConfig) -> TermComResult<String> {
        let mut registry = self.registry.write().await;
        let (session_id, transport_type) = registry.create_session(device_config).await?;
        
        // Send system message
        let message = Message::system(
            session_id.clone(),
            device_config.name.clone(),
            format!("Session created using {} transport", transport_type),
            transport_type.to_string(),
        );
        
        self.add_message_to_history(message).await;
        
        info!("Created session '{}' for device '{}'", session_id, device_config.name);
        Ok(session_id)
    }
    
    /// Close a communication session
    pub async fn close_session(&self, session_id: &str) -> TermComResult<()> {
        let mut registry = self.registry.write().await;
        
        // Get session info before closing
        let session_info = registry.get_session_info(session_id).await;
        
        registry.close_session(session_id).await?;
        
        // Send system message
        if let Some(info) = session_info {
            let message = Message::system(
                session_id.to_string(),
                info.device_name,
                "Session closed".to_string(),
                info.transport_type.to_string(),
            );
            
            self.add_message_to_history(message).await;
        }
        
        info!("Closed session '{}'", session_id);
        Ok(())
    }
    
    /// Send data to a session
    pub async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()> {
        let mut registry = self.registry.write().await;
        
        // Get session info
        let session_info = registry.get_session_info(session_id).await
            .ok_or_else(|| TermComError::Communication {
                message: format!("Session '{}' not found", session_id),
            })?;
        
        // Send data
        registry.send_data(session_id, data.clone()).await?;
        
        // Create and store message
        let mut message = Message::sent(
            session_id.to_string(),
            session_info.device_name,
            data.clone(),
            session_info.transport_type.to_string(),
        );
        
        // Set sequence number
        let sequence = self.next_sequence().await;
        message.set_sequence(sequence);
        
        self.add_message_to_history(message).await;
        
        debug!("Sent {} bytes to session '{}'", data.len(), session_id);
        Ok(())
    }
    
    /// Send a command to a session
    pub async fn send_command(&self, session_id: &str, command: &str) -> TermComResult<()> {
        let mut registry = self.registry.write().await;
        
        // Get session info
        let session_info = registry.get_session_info(session_id).await
            .ok_or_else(|| TermComError::Communication {
                message: format!("Session '{}' not found", session_id),
            })?;
        
        // Send command
        registry.send_data(session_id, command.as_bytes().to_vec()).await?;
        
        // Create and store message
        let mut message = Message::command(
            session_id.to_string(),
            session_info.device_name,
            command.to_string(),
            session_info.transport_type.to_string(),
        );
        
        // Set sequence number
        let sequence = self.next_sequence().await;
        message.set_sequence(sequence);
        
        self.add_message_to_history(message).await;
        
        debug!("Sent command '{}' to session '{}'", command, session_id);
        Ok(())
    }
    
    /// Get session information
    pub async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
        let mut registry = self.registry.write().await;
        registry.get_session_info(session_id).await
    }
    
    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let mut registry = self.registry.write().await;
        registry.list_all_sessions().await
    }
    
    /// Get available transport types
    pub async fn available_transports(&self) -> Vec<TransportType> {
        let registry = self.registry.read().await;
        registry.available_transports()
    }
    
    /// Get message history
    pub async fn get_message_history(&self) -> Vec<Message> {
        let history = self.message_history.read().await;
        history.iter().cloned().collect()
    }
    
    /// Get filtered message history
    pub async fn get_filtered_messages(&self, pattern: &MessagePattern) -> Vec<Message> {
        let history = self.message_history.read().await;
        history
            .iter()
            .filter(|msg| msg.matches_pattern(pattern))
            .cloned()
            .collect()
    }
    
    /// Clear message history
    pub async fn clear_history(&self) {
        let mut history = self.message_history.write().await;
        history.clear();
        info!("Message history cleared");
    }
    
    /// Get statistics
    pub async fn get_statistics(&self) -> CommunicationStats {
        let history = self.message_history.read().await;
        let sessions = self.list_sessions().await;
        
        let total_messages = history.len();
        let sent_messages = history.iter().filter(|m| matches!(m.message_type, MessageType::Sent)).count();
        let received_messages = history.iter().filter(|m| matches!(m.message_type, MessageType::Received)).count();
        let error_messages = history.iter().filter(|m| matches!(m.message_type, MessageType::Error)).count();
        
        let total_bytes_sent: usize = history
            .iter()
            .filter(|m| matches!(m.message_type, MessageType::Sent))
            .map(|m| m.data.len())
            .sum();
        
        let total_bytes_received: usize = history
            .iter()
            .filter(|m| matches!(m.message_type, MessageType::Received))
            .map(|m| m.data.len())
            .sum();
        
        CommunicationStats {
            total_sessions: sessions.len(),
            active_sessions: sessions.iter().filter(|s| matches!(s.status, crate::core::communication::transport::SessionStatus::Connected)).count(),
            total_messages,
            sent_messages,
            received_messages,
            error_messages,
            total_bytes_sent,
            total_bytes_received,
            uptime: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default(),
        }
    }
    
    /// Check if engine is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    // Private methods
    
    async fn start_message_processor(&self) {
        let message_receiver = Arc::clone(&self.message_receiver);
        let message_history = Arc::clone(&self.message_history);
        let max_history_size = self.max_history_size;
        let running = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            let mut receiver = message_receiver.write().await;
            
            while *running.read().await {
                if let Some(message) = receiver.recv().await {
                    let mut history = message_history.write().await;
                    
                    // Add message to history
                    history.push_back(message);
                    
                    // Trim history if needed
                    while history.len() > max_history_size {
                        history.pop_front();
                    }
                }
            }
        });
    }
    
    async fn add_message_to_history(&self, message: Message) {
        if let Err(_) = self.message_sender.send(message) {
            error!("Failed to send message to history processor");
        }
    }
    
    async fn next_sequence(&self) -> u64 {
        let mut counter = self.sequence_counter.write().await;
        *counter += 1;
        *counter
    }
}

/// Communication statistics
#[derive(Debug, Clone)]
pub struct CommunicationStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub total_messages: usize,
    pub sent_messages: usize,
    pub received_messages: usize,
    pub error_messages: usize,
    pub total_bytes_sent: usize,
    pub total_bytes_received: usize,
    pub uptime: Duration,
}

// Transport adapters to integrate with existing infrastructure

/// Serial transport adapter
struct SerialTransportAdapter {
    manager: SerialManager,
}

impl SerialTransportAdapter {
    fn new(max_sessions: usize) -> Self {
        Self {
            manager: SerialManager::new(max_sessions),
        }
    }
}

#[async_trait::async_trait]
impl Transport for SerialTransportAdapter {
    fn transport_type(&self) -> TransportType {
        TransportType::Serial
    }
    
    async fn create_session(&self, device_config: &DeviceConfig) -> TermComResult<String> {
        self.manager.create_session(device_config).await
    }
    
    async fn close_session(&self, session_id: &str) -> TermComResult<()> {
        self.manager.close_session(&session_id.to_string()).await
    }
    
    async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()> {
        self.manager.send_data(&session_id.to_string(), data).await
    }
    
    async fn send_command(&self, session_id: &str, command: &str) -> TermComResult<()> {
        self.manager.send_command(&session_id.to_string(), command).await
    }
    
    async fn receive_message(&mut self) -> Option<Message> {
        // This would need to be implemented to convert SerialMessage to Message
        // For now, return None as placeholder
        None
    }
    
    async fn is_session_connected(&self, session_id: &str) -> bool {
        self.manager.is_session_connected(&session_id.to_string()).await
    }
    
    async fn get_session_info(&self, session_id: &str) -> Option<crate::core::communication::transport::SessionInfo> {
        if let Some(info) = self.manager.get_session_info(&session_id.to_string()).await {
            Some(crate::core::communication::transport::SessionInfo {
                id: info.id,
                device_name: info.device_name,
                transport_type: TransportType::Serial,
                status: match info.status {
                    crate::infrastructure::serial::manager::SessionStatus::Connected => 
                        crate::core::communication::transport::SessionStatus::Connected,
                    crate::infrastructure::serial::manager::SessionStatus::Disconnected => 
                        crate::core::communication::transport::SessionStatus::Disconnected,
                    crate::infrastructure::serial::manager::SessionStatus::Error(e) => 
                        crate::core::communication::transport::SessionStatus::Error(e),
                },
                created_at: info.created_at,
                last_activity: info.last_activity,
                bytes_sent: 0, // These would need to be tracked in the serial manager
                bytes_received: 0,
                messages_sent: 0,
                messages_received: 0,
            })
        } else {
            None
        }
    }
    
    async fn list_sessions(&self) -> Vec<crate::core::communication::transport::SessionInfo> {
        self.manager.list_sessions().await
            .into_iter()
            .map(|info| crate::core::communication::transport::SessionInfo {
                id: info.id,
                device_name: info.device_name,
                transport_type: TransportType::Serial,
                status: match info.status {
                    crate::infrastructure::serial::manager::SessionStatus::Connected => 
                        crate::core::communication::transport::SessionStatus::Connected,
                    crate::infrastructure::serial::manager::SessionStatus::Disconnected => 
                        crate::core::communication::transport::SessionStatus::Disconnected,
                    crate::infrastructure::serial::manager::SessionStatus::Error(e) => 
                        crate::core::communication::transport::SessionStatus::Error(e),
                },
                created_at: info.created_at,
                last_activity: info.last_activity,
                bytes_sent: 0,
                bytes_received: 0,
                messages_sent: 0,
                messages_received: 0,
            })
            .collect()
    }
    
    async fn close_all_sessions(&self) -> TermComResult<()> {
        self.manager.close_all_sessions().await
    }
    
    async fn get_session_count(&self) -> usize {
        self.manager.get_session_count().await
    }
    
    fn get_max_sessions(&self) -> usize {
        self.manager.get_max_sessions()
    }
}

/// TCP transport adapter
struct TcpTransportAdapter {
    manager: TcpManager,
}

impl TcpTransportAdapter {
    fn new(max_sessions: usize) -> Self {
        Self {
            manager: TcpManager::new(max_sessions),
        }
    }
}

#[async_trait::async_trait]
impl Transport for TcpTransportAdapter {
    fn transport_type(&self) -> TransportType {
        TransportType::Tcp
    }
    
    async fn create_session(&self, device_config: &DeviceConfig) -> TermComResult<String> {
        self.manager.create_session(device_config).await
    }
    
    async fn close_session(&self, session_id: &str) -> TermComResult<()> {
        self.manager.close_session(&session_id.to_string()).await
    }
    
    async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()> {
        self.manager.send_data(&session_id.to_string(), data).await
    }
    
    async fn send_command(&self, session_id: &str, command: &str) -> TermComResult<()> {
        self.manager.send_command(&session_id.to_string(), command).await
    }
    
    async fn receive_message(&mut self) -> Option<Message> {
        // This would need to be implemented to convert TcpMessage to Message
        // For now, return None as placeholder
        None
    }
    
    async fn is_session_connected(&self, session_id: &str) -> bool {
        self.manager.is_session_connected(&session_id.to_string()).await
    }
    
    async fn get_session_info(&self, session_id: &str) -> Option<crate::core::communication::transport::SessionInfo> {
        if let Some(info) = self.manager.get_session_info(&session_id.to_string()).await {
            Some(crate::core::communication::transport::SessionInfo {
                id: info.id,
                device_name: info.device_name,
                transport_type: TransportType::Tcp,
                status: match info.status {
                    crate::infrastructure::tcp::manager::SessionStatus::Connected => 
                        crate::core::communication::transport::SessionStatus::Connected,
                    crate::infrastructure::tcp::manager::SessionStatus::Disconnected => 
                        crate::core::communication::transport::SessionStatus::Disconnected,
                    crate::infrastructure::tcp::manager::SessionStatus::Error(e) => 
                        crate::core::communication::transport::SessionStatus::Error(e),
                },
                created_at: info.created_at,
                last_activity: info.last_activity,
                bytes_sent: 0, // These would need to be tracked in the TCP manager
                bytes_received: 0,
                messages_sent: 0,
                messages_received: 0,
            })
        } else {
            None
        }
    }
    
    async fn list_sessions(&self) -> Vec<crate::core::communication::transport::SessionInfo> {
        self.manager.list_sessions().await
            .into_iter()
            .map(|info| crate::core::communication::transport::SessionInfo {
                id: info.id,
                device_name: info.device_name,
                transport_type: TransportType::Tcp,
                status: match info.status {
                    crate::infrastructure::tcp::manager::SessionStatus::Connected => 
                        crate::core::communication::transport::SessionStatus::Connected,
                    crate::infrastructure::tcp::manager::SessionStatus::Disconnected => 
                        crate::core::communication::transport::SessionStatus::Disconnected,
                    crate::infrastructure::tcp::manager::SessionStatus::Error(e) => 
                        crate::core::communication::transport::SessionStatus::Error(e),
                },
                created_at: info.created_at,
                last_activity: info.last_activity,
                bytes_sent: 0,
                bytes_received: 0,
                messages_sent: 0,
                messages_received: 0,
            })
            .collect()
    }
    
    async fn close_all_sessions(&self) -> TermComResult<()> {
        self.manager.close_all_sessions().await
    }
    
    async fn get_session_count(&self) -> usize {
        self.manager.get_session_count().await
    }
    
    fn get_max_sessions(&self) -> usize {
        self.manager.get_max_sessions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ConnectionConfig, DeviceConfig, ParityConfig, FlowControlConfig};
    
    fn create_test_serial_device() -> DeviceConfig {
        DeviceConfig {
            name: "test_serial".to_string(),
            description: "Test serial device".to_string(),
            connection: ConnectionConfig::Serial {
                port: "/dev/null".to_string(),
                baud_rate: 9600,
                data_bits: 8,
                stop_bits: 1,
                parity: ParityConfig::None,
                flow_control: FlowControlConfig::None,
            },
            commands: Vec::new(),
        }
    }
    
    #[tokio::test]
    async fn test_communication_engine_creation() {
        let engine = CommunicationEngine::new(1000, 10);
        
        assert!(!engine.is_running().await);
        assert_eq!(engine.available_transports().await.len(), 2);
        assert!(engine.available_transports().await.contains(&TransportType::Serial));
        assert!(engine.available_transports().await.contains(&TransportType::Tcp));
    }
    
    #[tokio::test]
    async fn test_engine_lifecycle() {
        let engine = CommunicationEngine::new(1000, 10);
        
        // Start engine
        assert!(engine.start().await.is_ok());
        assert!(engine.is_running().await);
        
        // Try to start again (should fail)
        assert!(engine.start().await.is_err());
        
        // Stop engine
        assert!(engine.stop().await.is_ok());
        assert!(!engine.is_running().await);
    }
    
    #[tokio::test]
    async fn test_session_creation_failure() {
        let engine = CommunicationEngine::new(1000, 10);
        assert!(engine.start().await.is_ok());
        
        let device = create_test_serial_device();
        
        // This should fail because /dev/null is not a valid serial port
        let result = engine.create_session(&device).await;
        assert!(result.is_err());
        
        assert!(engine.stop().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_message_history() {
        let engine = CommunicationEngine::new(1000, 10);
        
        // Start the engine first to enable message processing
        assert!(engine.start().await.is_ok());
        
        // Add a message manually (for testing)
        let message = Message::system(
            "test_session".to_string(),
            "test_device".to_string(),
            "Test message".to_string(),
            "test".to_string(),
        );
        
        engine.add_message_to_history(message).await;
        
        // Give some time for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let history = engine.get_message_history().await;
        assert!(!history.is_empty());
        
        // Stop the engine
        assert!(engine.stop().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_statistics() {
        let engine = CommunicationEngine::new(1000, 10);
        let stats = engine.get_statistics().await;
        
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_messages, 0);
    }
}