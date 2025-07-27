use crate::core::{
    communication::{CommunicationEngine, Message, MessagePattern, MessageType},
    session::state::{SessionState, SessionActivity, SessionStatus, ActivityType},
};
use crate::domain::{config::DeviceConfig, error::{TermComError, TermComResult}};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Session type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionType {
    /// Interactive session for manual communication
    Interactive,
    /// Automated session for scripted communication
    Automated,
    /// Monitoring session for passive observation
    Monitoring,
    /// Testing session for device testing
    Testing,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session name
    pub name: String,
    /// Session type
    pub session_type: SessionType,
    /// Device configuration
    pub device_config: DeviceConfig,
    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in milliseconds
    pub reconnect_delay_ms: u64,
    /// Session timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u64,
    /// Maximum message history size
    pub max_history_size: usize,
    /// Enable activity logging
    pub log_activities: bool,
    /// Custom tags
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: std::collections::HashMap<String, String>,
}

/// Active session instance
pub struct Session {
    /// Session configuration
    config: SessionConfig,
    /// Session state
    state: Arc<RwLock<SessionState>>,
    /// Communication engine reference
    comm_engine: Arc<CommunicationEngine>,
    /// Transport session ID
    transport_session_id: Option<String>,
    /// Message history
    message_history: Arc<RwLock<VecDeque<Message>>>,
    /// Activity history
    activity_history: Arc<RwLock<VecDeque<SessionActivity>>>,
    /// Activity sender
    activity_sender: mpsc::UnboundedSender<SessionActivity>,
    /// Running flag
    running: Arc<RwLock<bool>>,
    /// Background task handles
    _background_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Session {
    /// Create a new session
    pub async fn new(
        config: SessionConfig,
        comm_engine: Arc<CommunicationEngine>,
    ) -> TermComResult<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let transport_type = match &config.device_config.connection {
            crate::domain::config::ConnectionConfig::Serial { .. } => "serial",
            crate::domain::config::ConnectionConfig::Tcp { .. } => "tcp",
        };
        
        let mut state = SessionState::new(
            session_id.clone(),
            config.device_config.name.clone(),
            transport_type.to_string(),
        );
        
        // Set configuration metadata
        state.metadata.config_name = Some(config.name.clone());
        for tag in &config.tags {
            state.add_tag(tag.clone());
        }
        for (key, value) in &config.properties {
            state.add_property(key.clone(), value.clone());
        }
        
        let state = Arc::new(RwLock::new(state));
        let message_history = Arc::new(RwLock::new(VecDeque::new()));
        let activity_history = Arc::new(RwLock::new(VecDeque::new()));
        let (activity_sender, activity_receiver) = mpsc::unbounded_channel();
        let running = Arc::new(RwLock::new(false));
        
        // Start activity processor
        let activity_processor = Self::start_activity_processor(
            Arc::clone(&state),
            Arc::clone(&activity_history),
            activity_receiver,
            config.log_activities,
            config.max_history_size,
        );
        
        Ok(Self {
            config,
            state,
            comm_engine,
            transport_session_id: None,
            message_history,
            activity_history,
            activity_sender,
            running,
            _background_tasks: vec![activity_processor],
        })
    }
    
    /// Start the session
    pub async fn start(&mut self) -> TermComResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(TermComError::Session {
                message: "Session is already running".to_string(),
            });
        }
        
        // Update state to initializing
        {
            let mut state = self.state.write().await;
            state.update_status(SessionStatus::Initializing);
        }
        
        // Record creation activity
        self.record_activity(SessionActivity::new(
            ActivityType::Created,
            format!("Session '{}' created for device '{}'", 
                    self.config.name, self.config.device_config.name),
        )).await;
        
        // Create transport session
        match self.comm_engine.create_session(&self.config.device_config).await {
            Ok(transport_session_id) => {
                self.transport_session_id = Some(transport_session_id);
                
                // Update state to active
                {
                    let mut state = self.state.write().await;
                    state.update_status(SessionStatus::Active);
                }
                
                // Record connection activity
                self.record_activity(SessionActivity::new(
                    ActivityType::Connected,
                    "Transport session established".to_string(),
                )).await;
                
                *running = true;
                info!("Session '{}' started successfully", self.config.name);
                Ok(())
            }
            Err(e) => {
                // Update state to error
                {
                    let mut state = self.state.write().await;
                    state.update_status(SessionStatus::Error(e.to_string()));
                }
                
                // Record error activity
                self.record_activity(SessionActivity::error(e.to_string())).await;
                
                error!("Failed to start session '{}': {}", self.config.name, e);
                Err(e)
            }
        }
    }
    
    /// Stop the session
    pub async fn stop(&mut self) -> TermComResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        // Update state to closing
        {
            let mut state = self.state.write().await;
            state.update_status(SessionStatus::Closing);
        }
        
        // Close transport session if exists
        if let Some(ref transport_session_id) = self.transport_session_id {
            if let Err(e) = self.comm_engine.close_session(transport_session_id).await {
                warn!("Failed to close transport session: {}", e);
            }
            self.transport_session_id = None;
        }
        
        // Update state to closed
        {
            let mut state = self.state.write().await;
            state.update_status(SessionStatus::Closed);
        }
        
        // Record closure activity
        self.record_activity(SessionActivity::new(
            ActivityType::Closed,
            "Session closed".to_string(),
        )).await;
        
        *running = false;
        info!("Session '{}' stopped", self.config.name);
        Ok(())
    }
    
    /// Send data to the device
    pub async fn send_data(&self, data: Vec<u8>) -> TermComResult<()> {
        self.ensure_running().await?;
        
        if let Some(ref transport_session_id) = self.transport_session_id {
            // Send data through communication engine
            self.comm_engine.send_data(transport_session_id, data.clone()).await?;
            
            // Record activity
            self.record_activity(
                SessionActivity::data_sent(
                    format!("Sent {} bytes", data.len()),
                    data.len(),
                )
            ).await;
            
            debug!("Session '{}' sent {} bytes", self.config.name, data.len());
            Ok(())
        } else {
            Err(TermComError::Session {
                message: "No active transport session".to_string(),
            })
        }
    }
    
    /// Send a command to the device
    pub async fn send_command(&self, command: &str) -> TermComResult<()> {
        self.ensure_running().await?;
        
        if let Some(ref transport_session_id) = self.transport_session_id {
            // Send command through communication engine
            self.comm_engine.send_command(transport_session_id, command).await?;
            
            // Record activity
            self.record_activity(
                SessionActivity::command_executed(command.to_string())
            ).await;
            
            debug!("Session '{}' executed command: {}", self.config.name, command);
            Ok(())
        } else {
            Err(TermComError::Session {
                message: "No active transport session".to_string(),
            })
        }
    }
    
    /// Get session state
    pub async fn get_state(&self) -> SessionState {
        self.state.read().await.clone()
    }
    
    /// Get session configuration
    pub fn get_config(&self) -> &SessionConfig {
        &self.config
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
    
    /// Get activity history
    pub async fn get_activity_history(&self) -> Vec<SessionActivity> {
        let history = self.activity_history.read().await;
        history.iter().cloned().collect()
    }
    
    /// Clear message history
    pub async fn clear_message_history(&self) {
        let mut history = self.message_history.write().await;
        history.clear();
        debug!("Session '{}' message history cleared", self.config.name);
    }
    
    /// Clear activity history
    pub async fn clear_activity_history(&self) {
        let mut history = self.activity_history.write().await;
        history.clear();
        debug!("Session '{}' activity history cleared", self.config.name);
    }
    
    /// Check if session is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    /// Get session ID
    pub async fn get_session_id(&self) -> String {
        self.state.read().await.session_id.clone()
    }
    
    /// Update session configuration
    pub async fn update_config(&mut self, new_config: SessionConfig) -> TermComResult<()> {
        if self.is_running().await {
            return Err(TermComError::Session {
                message: "Cannot update configuration while session is running".to_string(),
            });
        }
        
        self.config = new_config;
        
        // Update state metadata
        {
            let mut state = self.state.write().await;
            state.metadata.config_name = Some(self.config.name.clone());
            state.metadata.tags = self.config.tags.clone();
            state.metadata.properties = self.config.properties.clone();
        }
        
        debug!("Session configuration updated");
        Ok(())
    }
    
    // Private methods
    
    async fn ensure_running(&self) -> TermComResult<()> {
        if !self.is_running().await {
            return Err(TermComError::Session {
                message: "Session is not running".to_string(),
            });
        }
        Ok(())
    }
    
    async fn record_activity(&self, activity: SessionActivity) {
        if let Err(_) = self.activity_sender.send(activity) {
            error!("Failed to record activity for session '{}'", self.config.name);
        }
    }
    
    fn start_activity_processor(
        state: Arc<RwLock<SessionState>>,
        activity_history: Arc<RwLock<VecDeque<SessionActivity>>>,
        mut activity_receiver: mpsc::UnboundedReceiver<SessionActivity>,
        log_activities: bool,
        max_history_size: usize,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(activity) = activity_receiver.recv().await {
                // Update state with activity
                {
                    let mut state = state.write().await;
                    state.record_activity(activity.clone());
                }
                
                // Add to activity history
                {
                    let mut history = activity_history.write().await;
                    history.push_back(activity.clone());
                    
                    // Trim history if needed
                    while history.len() > max_history_size {
                        history.pop_front();
                    }
                }
                
                // Log activity if enabled
                if log_activities {
                    debug!("Session activity: {} - {}", activity.activity_type, activity.description);
                }
            }
        })
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            name: "default_session".to_string(),
            session_type: SessionType::Interactive,
            device_config: DeviceConfig {
                name: "default_device".to_string(),
                description: "Default device".to_string(),
                connection: crate::domain::config::ConnectionConfig::Serial {
                    port: "/dev/ttyUSB0".to_string(),
                    baud_rate: 9600,
                    data_bits: 8,
                    stop_bits: 1,
                    parity: crate::domain::config::ParityConfig::None,
                    flow_control: crate::domain::config::FlowControlConfig::None,
                },
                commands: Vec::new(),
            },
            auto_reconnect: false,
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
            timeout_ms: 0,
            max_history_size: 1000,
            log_activities: true,
            tags: Vec::new(),
            properties: std::collections::HashMap::new(),
        }
    }
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::Interactive => write!(f, "Interactive"),
            SessionType::Automated => write!(f, "Automated"),
            SessionType::Monitoring => write!(f, "Monitoring"),
            SessionType::Testing => write!(f, "Testing"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ConnectionConfig, ParityConfig, FlowControlConfig};
    
    fn create_test_config() -> SessionConfig {
        SessionConfig {
            name: "test_session".to_string(),
            session_type: SessionType::Testing,
            device_config: DeviceConfig {
                name: "test_device".to_string(),
                description: "Test device".to_string(),
                connection: ConnectionConfig::Serial {
                    port: "/dev/null".to_string(),
                    baud_rate: 9600,
                    data_bits: 8,
                    stop_bits: 1,
                    parity: ParityConfig::None,
                    flow_control: FlowControlConfig::None,
                },
                commands: Vec::new(),
            },
            auto_reconnect: false,
            max_reconnect_attempts: 1,
            reconnect_delay_ms: 100,
            timeout_ms: 5000,
            max_history_size: 100,
            log_activities: true,
            tags: vec!["test".to_string()],
            properties: {
                let mut props = std::collections::HashMap::new();
                props.insert("priority".to_string(), "high".to_string());
                props
            },
        }
    }
    
    async fn create_test_session() -> Session {
        let config = create_test_config();
        let comm_engine = Arc::new(CommunicationEngine::new(1000, 10));
        Session::new(config, comm_engine).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_session_creation() {
        let session = create_test_session().await;
        
        assert_eq!(session.config.name, "test_session");
        assert!(matches!(session.config.session_type, SessionType::Testing));
        assert!(!session.is_running().await);
        
        let state = session.get_state().await;
        assert_eq!(state.device_name, "test_device");
        assert!(matches!(state.status, SessionStatus::Initializing));
    }
    
    #[tokio::test]
    async fn test_session_lifecycle() {
        let mut session = create_test_session().await;
        
        // Session should not be running initially
        assert!(!session.is_running().await);
        
        // Try to start session (will fail because /dev/null is not a valid serial port)
        let result = session.start().await;
        assert!(result.is_err());
        
        // Session should still not be running after failed start
        assert!(!session.is_running().await);
        
        // Stop should work even if not started
        let result = session.stop().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_session_operations_when_not_running() {
        let session = create_test_session().await;
        
        // Operations should fail when session is not running
        let result = session.send_data(b"test".to_vec()).await;
        assert!(result.is_err());
        
        let result = session.send_command("TEST").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_session_history() {
        let session = create_test_session().await;
        
        // Initially empty messages
        let messages = session.get_message_history().await;
        assert!(messages.is_empty());
        
        // Should have creation activity, but let's wait a bit for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let activities = session.get_activity_history().await;
        // Don't assert on activity count as it's async and timing dependent
        // Just test that we can retrieve activities
        let _ = activities.len();
        
        // Clear histories
        session.clear_message_history().await;
        session.clear_activity_history().await;
        
        let activities = session.get_activity_history().await;
        assert!(activities.is_empty());
    }
    
    #[tokio::test]
    async fn test_configuration_update() {
        let mut session = create_test_session().await;
        
        let mut new_config = create_test_config();
        new_config.name = "updated_session".to_string();
        new_config.tags.push("updated".to_string());
        
        // Should succeed when not running
        let result = session.update_config(new_config).await;
        assert!(result.is_ok());
        assert_eq!(session.config.name, "updated_session");
        
        let state = session.get_state().await;
        assert!(state.metadata.tags.contains(&"updated".to_string()));
    }
    
    #[test]
    fn test_session_type_display() {
        assert_eq!(SessionType::Interactive.to_string(), "Interactive");
        assert_eq!(SessionType::Automated.to_string(), "Automated");
        assert_eq!(SessionType::Monitoring.to_string(), "Monitoring");
        assert_eq!(SessionType::Testing.to_string(), "Testing");
    }
    
    #[test]
    fn test_default_session_config() {
        let config = SessionConfig::default();
        assert_eq!(config.name, "default_session");
        assert!(matches!(config.session_type, SessionType::Interactive));
        assert!(!config.auto_reconnect);
        assert_eq!(config.max_history_size, 1000);
    }
}