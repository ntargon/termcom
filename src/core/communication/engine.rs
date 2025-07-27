use crate::core::communication::{
    message::{Message, MessagePattern, MessageType},
    transport::{Transport, TransportRegistry, TransportType, SessionInfo},
};
use crate::domain::{config::DeviceConfig, error::{TermComError, TermComResult}};
use crate::infrastructure::{serial::SerialManager, tcp::TcpManager};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{mpsc, RwLock};
use std::sync::{Arc, atomic::{AtomicUsize, AtomicU64, Ordering}};
use tracing::{debug, error, info, warn};

/// Central communication engine that manages all transport types
pub struct CommunicationEngine {
    registry: Arc<RwLock<TransportRegistry>>,
    message_history: Arc<RwLock<VecDeque<Message>>>,
    message_sender: mpsc::UnboundedSender<Message>,
    message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<Message>>>,
    sequence_counter: Arc<AtomicU64>,
    max_history_size: usize,
    running: Arc<RwLock<bool>>,
    start_time: Arc<RwLock<Option<Instant>>>,
    // Performance metrics
    total_bytes_sent: Arc<AtomicU64>,
    total_bytes_received: Arc<AtomicU64>,
    message_count: Arc<AtomicUsize>,
    // Memory optimization
    last_cleanup: Arc<RwLock<Instant>>,
    cleanup_interval: Duration,
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
            message_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history_size))),
            message_sender,
            message_receiver: Arc::new(RwLock::new(message_receiver)),
            sequence_counter: Arc::new(AtomicU64::new(0)),
            max_history_size,
            running: Arc::new(RwLock::new(false)),
            start_time: Arc::new(RwLock::new(None)),
            total_bytes_sent: Arc::new(AtomicU64::new(0)),
            total_bytes_received: Arc::new(AtomicU64::new(0)),
            message_count: Arc::new(AtomicUsize::new(0)),
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
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
        
        // Set start time
        {
            let mut start_time = self.start_time.write().await;
            *start_time = Some(Instant::now());
        }
        
        info!("Communication engine started");
        
        // Start message processing task
        self.start_message_processor().await;
        
        // Start periodic cleanup task
        self.start_cleanup_task().await;
        
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
        
        // Update statistics
        self.total_bytes_sent.fetch_add(data.len() as u64, Ordering::Relaxed);
        
        // Create and store message
        let mut message = Message::sent(
            session_id.to_string(),
            session_info.device_name,
            data.clone(),
            session_info.transport_type.to_string(),
        );
        
        // Set sequence number
        let sequence = self.next_sequence();
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
        let command_bytes = command.as_bytes().to_vec();
        registry.send_data(session_id, command_bytes.clone()).await?;
        
        // Update statistics
        self.total_bytes_sent.fetch_add(command_bytes.len() as u64, Ordering::Relaxed);
        
        // Create and store message
        let mut message = Message::command(
            session_id.to_string(),
            session_info.device_name,
            command.to_string(),
            session_info.transport_type.to_string(),
        );
        
        // Set sequence number
        let sequence = self.next_sequence();
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
        let sessions = self.list_sessions().await;
        
        // Use atomic counters for performance metrics
        let total_bytes_sent = self.total_bytes_sent.load(Ordering::Relaxed);
        let total_bytes_received = self.total_bytes_received.load(Ordering::Relaxed);
        let total_messages = self.message_count.load(Ordering::Relaxed);
        
        // Calculate uptime efficiently
        let uptime = if let Some(start) = *self.start_time.read().await {
            start.elapsed()
        } else {
            Duration::default()
        };
        
        // For detailed message counts, only read history if needed
        let (sent_messages, received_messages, error_messages) = {
            let history = self.message_history.read().await;
            let sent = history.iter().filter(|m| matches!(m.message_type, MessageType::Sent)).count();
            let received = history.iter().filter(|m| matches!(m.message_type, MessageType::Received)).count();
            let error = history.iter().filter(|m| matches!(m.message_type, MessageType::Error)).count();
            (sent, received, error)
        };
        
        CommunicationStats {
            total_sessions: sessions.len(),
            active_sessions: sessions.iter().filter(|s| matches!(s.status, crate::core::communication::transport::SessionStatus::Connected)).count(),
            total_messages,
            sent_messages,
            received_messages,
            error_messages,
            total_bytes_sent: total_bytes_sent as usize,
            total_bytes_received: total_bytes_received as usize,
            uptime,
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
        let message_count = Arc::clone(&self.message_count);
        let max_history_size = self.max_history_size;
        let running = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            let mut receiver = message_receiver.write().await;
            
            while *running.read().await {
                if let Some(message) = receiver.recv().await {
                    let mut history = message_history.write().await;
                    
                    // Add message to history
                    history.push_back(message);
                    
                    // Update total message count
                    message_count.fetch_add(1, Ordering::Relaxed);
                    
                    // Trim history if needed
                    while history.len() > max_history_size {
                        history.pop_front();
                    }
                    
                    // Release lock early for better concurrency
                    drop(history);
                }
            }
        });
    }
    
    async fn add_message_to_history(&self, message: Message) {
        if let Err(_) = self.message_sender.send(message) {
            error!("Failed to send message to history processor");
        }
    }
    
    fn next_sequence(&self) -> u64 {
        self.sequence_counter.fetch_add(1, Ordering::Relaxed)
    }
    
    /// Trigger periodic cleanup of old messages and optimize memory usage
    pub async fn cleanup_old_data(&self) {
        let mut last_cleanup = self.last_cleanup.write().await;
        let now = Instant::now();
        
        if now.duration_since(*last_cleanup) > self.cleanup_interval {
            *last_cleanup = now;
            drop(last_cleanup);
            
            let mut history = self.message_history.write().await;
            let initial_size = history.len();
            
            // Keep only the most recent messages, but ensure we don't exceed max size
            if history.len() > self.max_history_size {
                let excess = history.len() - self.max_history_size;
                for _ in 0..excess {
                    history.pop_front();
                }
            }
            
            // Shrink the VecDeque to save memory if it's significantly oversized
            if history.capacity() > self.max_history_size * 2 {
                history.shrink_to_fit();
            }
            
            let final_size = history.len();
            if initial_size != final_size {
                info!("Cleaned up message history: {} -> {} messages", initial_size, final_size);
            }
        }
    }
    
    /// Get memory usage information
    pub async fn get_memory_info(&self) -> MemoryInfo {
        let history = self.message_history.read().await;
        let message_count = history.len();
        let capacity = history.capacity();
        
        // Estimate memory usage (approximate)
        let estimated_bytes = message_count * std::mem::size_of::<Message>() + 
                            capacity * std::mem::size_of::<Message>();
        
        MemoryInfo {
            message_count,
            message_capacity: capacity,
            estimated_memory_bytes: estimated_bytes,
            max_history_size: self.max_history_size,
        }
    }
    
    async fn start_cleanup_task(&self) {
        let engine_ref = Arc::new(self.clone_for_cleanup());
        let cleanup_interval = self.cleanup_interval;
        let running = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            while *running.read().await {
                interval.tick().await;
                engine_ref.cleanup_old_data().await;
            }
        });
    }
    
    fn clone_for_cleanup(&self) -> CommunicationEngineRef {
        CommunicationEngineRef {
            message_history: Arc::clone(&self.message_history),
            last_cleanup: Arc::clone(&self.last_cleanup),
            max_history_size: self.max_history_size,
            cleanup_interval: self.cleanup_interval,
        }
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

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub message_count: usize,
    pub message_capacity: usize,
    pub estimated_memory_bytes: usize,
    pub max_history_size: usize,
}

/// Lightweight reference for cleanup tasks
struct CommunicationEngineRef {
    message_history: Arc<RwLock<VecDeque<Message>>>,
    last_cleanup: Arc<RwLock<Instant>>,
    max_history_size: usize,
    cleanup_interval: Duration,
}

impl CommunicationEngineRef {
    async fn cleanup_old_data(&self) {
        let mut last_cleanup = self.last_cleanup.write().await;
        let now = Instant::now();
        
        if now.duration_since(*last_cleanup) > self.cleanup_interval {
            *last_cleanup = now;
            drop(last_cleanup);
            
            let mut history = self.message_history.write().await;
            let initial_size = history.len();
            
            // Keep only the most recent messages
            if history.len() > self.max_history_size {
                let excess = history.len() - self.max_history_size;
                for _ in 0..excess {
                    history.pop_front();
                }
            }
            
            // Shrink capacity if oversized
            if history.capacity() > self.max_history_size * 2 {
                history.shrink_to_fit();
            }
            
            let final_size = history.len();
            if initial_size != final_size {
                info!("Cleaned up message history: {} -> {} messages", initial_size, final_size);
            }
        }
    }
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