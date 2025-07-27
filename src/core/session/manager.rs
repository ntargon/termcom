use crate::core::{
    communication::CommunicationEngine,
    session::{
        session::{Session, SessionConfig, SessionType},
        state::{SessionState, SessionStatus},
    },
};
use crate::domain::error::{TermComError, TermComResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Session manager for handling multiple sessions
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Communication engine
    comm_engine: Arc<CommunicationEngine>,
    /// Maximum number of sessions
    max_sessions: usize,
    /// Session counter for generating unique IDs
    session_counter: Arc<RwLock<u64>>,
}

/// Session summary information
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub name: String,
    pub device_name: String,
    pub session_type: SessionType,
    pub status: SessionStatus,
    pub created_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
    pub uptime: std::time::Duration,
    pub message_count: usize,
    pub activity_count: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Session filter for querying sessions
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub session_type: Option<SessionType>,
    pub status: Option<SessionStatus>,
    pub device_name: Option<String>,
    pub tags: Vec<String>,
    pub name_pattern: Option<String>,
}

impl SessionFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Filter by session type
    pub fn with_session_type(mut self, session_type: SessionType) -> Self {
        self.session_type = Some(session_type);
        self
    }
    
    /// Filter by status
    pub fn with_status(mut self, status: SessionStatus) -> Self {
        self.status = Some(status);
        self
    }
    
    /// Filter by device name
    pub fn with_device_name(mut self, device_name: &str) -> Self {
        self.device_name = Some(device_name.to_string());
        self
    }
    
    /// Filter by name pattern
    pub fn with_name_pattern(mut self, pattern: &str) -> Self {
        self.name_pattern = Some(pattern.to_string());
        self
    }
    
    /// Add tag filter
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
}

/// Global session statistics
#[derive(Debug, Clone, Default)]
pub struct GlobalStatistics {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub total_messages: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_errors: u64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(comm_engine: Arc<CommunicationEngine>, max_sessions: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            comm_engine,
            max_sessions,
            session_counter: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Create a new session
    pub async fn create_session(&self, config: SessionConfig) -> TermComResult<String> {
        let sessions = self.sessions.read().await;
        
        // Check session limit
        if sessions.len() >= self.max_sessions {
            return Err(TermComError::Session {
                message: format!("Maximum number of sessions ({}) reached", self.max_sessions),
            });
        }
        
        // Check for duplicate session names
        if sessions.values().any(|s| s.get_config().name == config.name) {
            return Err(TermComError::Session {
                message: format!("Session with name '{}' already exists", config.name),
            });
        }
        
        drop(sessions);
        
        // Create new session
        let session = Session::new(config.clone(), Arc::clone(&self.comm_engine)).await?;
        let session_id = session.get_session_id().await;
        
        // Add to sessions map
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        
        // Increment counter
        {
            let mut counter = self.session_counter.write().await;
            *counter += 1;
        }
        
        info!("Created session '{}' with ID '{}'", config.name, session_id);
        Ok(session_id)
    }
    
    /// Start a session
    pub async fn start_session(&self, session_id: &str) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.start().await?;
            info!("Started session '{}'", session_id);
            Ok(())
        } else {
            Err(TermComError::Session {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    /// Stop a session
    pub async fn stop_session(&self, session_id: &str) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.stop().await?;
            info!("Stopped session '{}'", session_id);
            Ok(())
        } else {
            Err(TermComError::Session {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(mut session) = sessions.remove(session_id) {
            // Stop session if running
            if session.is_running().await {
                if let Err(e) = session.stop().await {
                    warn!("Failed to stop session before removal: {}", e);
                }
            }
            
            info!("Removed session '{}'", session_id);
            Ok(())
        } else {
            Err(TermComError::Session {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    /// Check if session exists
    pub async fn has_session(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(session_id)
    }
    
    /// Send data to a session
    pub async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            session.send_data(data).await
        } else {
            Err(TermComError::Session {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    /// Send command to a session
    pub async fn send_command(&self, session_id: &str, command: &str) -> TermComResult<()> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            session.send_command(command).await
        } else {
            Err(TermComError::Session {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    /// Get session state
    pub async fn get_session_state(&self, session_id: &str) -> Option<SessionState> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            Some(session.get_state().await)
        } else {
            None
        }
    }
    
    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().await;
        let mut summaries = Vec::new();
        
        for session in sessions.values() {
            if let Ok(summary) = self.create_session_summary(session).await {
                summaries.push(summary);
            }
        }
        
        summaries
    }
    
    /// List sessions with filter
    pub async fn list_sessions_filtered(&self, filter: &SessionFilter) -> Vec<SessionSummary> {
        let summaries = self.list_sessions().await;
        
        summaries
            .into_iter()
            .filter(|summary| self.matches_filter(summary, filter))
            .collect()
    }
    
    /// Get session count
    pub async fn get_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
    
    /// Get active session count
    pub async fn get_active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        let mut count = 0;
        
        for session in sessions.values() {
            if session.is_running().await {
                count += 1;
            }
        }
        
        count
    }
    
    /// Get maximum sessions
    pub fn get_max_sessions(&self) -> usize {
        self.max_sessions
    }
    
    /// Stop all sessions
    pub async fn stop_all_sessions(&self) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        let mut errors = Vec::new();
        
        for (session_id, session) in sessions.iter_mut() {
            if session.is_running().await {
                if let Err(e) = session.stop().await {
                    errors.push(format!("Session '{}': {}", session_id, e));
                }
            }
        }
        
        if !errors.is_empty() {
            return Err(TermComError::Session {
                message: format!("Errors stopping sessions: {}", errors.join(", ")),
            });
        }
        
        info!("Stopped all sessions");
        Ok(())
    }
    
    /// Remove all sessions
    pub async fn remove_all_sessions(&self) -> TermComResult<()> {
        // First stop all sessions
        self.stop_all_sessions().await?;
        
        // Then remove them
        let mut sessions = self.sessions.write().await;
        let count = sessions.len();
        sessions.clear();
        
        info!("Removed {} sessions", count);
        Ok(())
    }
    
    /// Start all sessions
    pub async fn start_all_sessions(&self) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        let mut errors = Vec::new();
        
        for (session_id, session) in sessions.iter_mut() {
            if !session.is_running().await {
                if let Err(e) = session.start().await {
                    errors.push(format!("Session '{}': {}", session_id, e));
                }
            }
        }
        
        if !errors.is_empty() {
            return Err(TermComError::Session {
                message: format!("Errors starting sessions: {}", errors.join(", ")),
            });
        }
        
        info!("Started all sessions");
        Ok(())
    }
    
    /// Get session statistics
    pub async fn get_statistics(&self) -> SessionManagerStats {
        let sessions = self.sessions.read().await;
        let mut stats = SessionManagerStats::default();
        
        stats.total_sessions = sessions.len();
        
        for session in sessions.values() {
            let state = session.get_state().await;
            
            match state.status {
                SessionStatus::Active => stats.active_sessions += 1,
                SessionStatus::Disconnected => stats.disconnected_sessions += 1,
                SessionStatus::Error(_) => stats.error_sessions += 1,
                SessionStatus::Closed => stats.closed_sessions += 1,
                _ => stats.other_sessions += 1,
            }
            
            stats.total_bytes_sent += state.statistics.bytes_sent;
            stats.total_bytes_received += state.statistics.bytes_received;
            stats.total_messages_sent += state.statistics.messages_sent;
            stats.total_messages_received += state.statistics.messages_received;
            stats.total_errors += state.statistics.error_count;
        }
        
        stats
    }
    
    /// Get global statistics across all sessions
    pub async fn get_global_statistics(&self) -> GlobalStatistics {
        let sessions = self.sessions.read().await;
        let mut stats = GlobalStatistics::default();
        
        stats.total_sessions = sessions.len();
        
        for session in sessions.values() {
            let state = session.get_state().await;
            
            if matches!(state.status, SessionStatus::Active) {
                stats.active_sessions += 1;
            }
            
            stats.total_messages += state.statistics.messages_sent + state.statistics.messages_received;
            stats.total_bytes_sent += state.statistics.bytes_sent;
            stats.total_bytes_received += state.statistics.bytes_received;
            stats.total_errors += state.statistics.error_count;
        }
        
        stats
    }
    
    /// Get filtered session summaries
    pub async fn get_sessions_summary_filtered(&self, filter: &SessionFilter) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().await;
        let mut summaries = Vec::new();
        
        for session in sessions.values() {
            let state = session.get_state().await;
            let config = session.get_config();
            
            // Apply filters
            if let Some(session_type) = &filter.session_type {
                if &config.session_type != session_type {
                    continue;
                }
            }
            
            if let Some(status) = &filter.status {
                if &state.status != status {
                    continue;
                }
            }
            
            if let Some(device_name) = &filter.device_name {
                if &config.device_config.name != device_name {
                    continue;
                }
            }
            
            if let Some(pattern) = &filter.name_pattern {
                if !config.name.contains(pattern) {
                    continue;
                }
            }
            
            if !filter.tags.is_empty() {
                let has_tag = filter.tags.iter().any(|tag| config.tags.contains(tag));
                if !has_tag {
                    continue;
                }
            }
            
            let summary = SessionSummary {
                session_id: state.session_id.clone(),
                name: config.name.clone(),
                device_name: config.device_config.name.clone(),
                session_type: config.session_type.clone(),
                status: state.status.clone(),
                created_at: state.created_at,
                last_activity: state.last_activity,
                uptime: state.created_at.elapsed().unwrap_or_default(),
                message_count: state.statistics.messages_sent as usize + state.statistics.messages_received as usize,
                activity_count: 0, // TODO: calculate from activity history
                bytes_sent: state.statistics.bytes_sent,
                bytes_received: state.statistics.bytes_received,
            };
            
            summaries.push(summary);
        }
        
        summaries
    }
    
    /// Find sessions by name pattern
    pub async fn find_sessions_by_name(&self, pattern: &str) -> Vec<String> {
        let sessions = self.sessions.read().await;
        let mut results = Vec::new();
        
        for (session_id, session) in sessions.iter() {
            let config = session.get_config();
            if config.name.contains(pattern) {
                results.push(session_id.clone());
            }
        }
        
        results
    }
    
    /// Find sessions by device name
    pub async fn find_sessions_by_device(&self, device_name: &str) -> Vec<String> {
        let sessions = self.sessions.read().await;
        let mut results = Vec::new();
        
        for (session_id, session) in sessions.iter() {
            let config = session.get_config();
            if config.device_config.name == device_name {
                results.push(session_id.clone());
            }
        }
        
        results
    }
    
    /// Update session configuration
    pub async fn update_session_config(
        &self,
        session_id: &str,
        new_config: SessionConfig,
    ) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.update_config(new_config).await?;
            debug!("Updated configuration for session '{}'", session_id);
            Ok(())
        } else {
            Err(TermComError::Session {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    // Private methods
    
    async fn create_session_summary(&self, session: &Session) -> TermComResult<SessionSummary> {
        let state = session.get_state().await;
        let config = session.get_config();
        let message_history = session.get_message_history().await;
        let activity_history = session.get_activity_history().await;
        
        Ok(SessionSummary {
            session_id: state.session_id.clone(),
            name: config.name.clone(),
            device_name: state.device_name.clone(),
            session_type: config.session_type.clone(),
            status: state.status.clone(),
            created_at: state.created_at,
            last_activity: state.last_activity,
            uptime: state.get_uptime(),
            message_count: message_history.len(),
            activity_count: activity_history.len(),
            bytes_sent: state.statistics.bytes_sent,
            bytes_received: state.statistics.bytes_received,
        })
    }
    
    fn matches_filter(&self, summary: &SessionSummary, filter: &SessionFilter) -> bool {
        // Check session type
        if let Some(ref session_type) = filter.session_type {
            if summary.session_type != *session_type {
                return false;
            }
        }
        
        // Check status
        if let Some(ref status) = filter.status {
            if std::mem::discriminant(&summary.status) != std::mem::discriminant(status) {
                return false;
            }
        }
        
        // Check device name
        if let Some(ref device_name) = filter.device_name {
            if summary.device_name != *device_name {
                return false;
            }
        }
        
        // Check name pattern
        if let Some(ref pattern) = filter.name_pattern {
            if !summary.name.contains(pattern) {
                return false;
            }
        }
        
        // For tags, we would need to access the session state, which is more complex
        // For now, we'll skip tag filtering in summaries
        
        true
    }
}

/// Session manager statistics
#[derive(Debug, Clone, Default)]
pub struct SessionManagerStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub disconnected_sessions: usize,
    pub error_sessions: usize,
    pub closed_sessions: usize,
    pub other_sessions: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_errors: u64,
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::session::SessionConfig;
    use crate::domain::config::{ConnectionConfig, DeviceConfig, ParityConfig, FlowControlConfig};
    
    fn create_test_device_config(name: &str) -> DeviceConfig {
        DeviceConfig {
            name: name.to_string(),
            description: format!("Test device {}", name),
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
    
    fn create_test_session_config(name: &str, device_name: &str) -> SessionConfig {
        SessionConfig {
            name: name.to_string(),
            session_type: SessionType::Testing,
            device_config: create_test_device_config(device_name),
            auto_reconnect: false,
            max_reconnect_attempts: 1,
            reconnect_delay_ms: 100,
            timeout_ms: 5000,
            max_history_size: 100,
            log_activities: true,
            tags: vec!["test".to_string()],
            properties: std::collections::HashMap::new(),
        }
    }
    
    async fn create_test_manager() -> SessionManager {
        let comm_engine = Arc::new(CommunicationEngine::new(1000, 10));
        SessionManager::new(comm_engine, 5)
    }
    
    #[tokio::test]
    async fn test_session_manager_creation() {
        let manager = create_test_manager().await;
        
        assert_eq!(manager.get_max_sessions(), 5);
        assert_eq!(manager.get_session_count().await, 0);
        assert_eq!(manager.get_active_session_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_session_creation() {
        let manager = create_test_manager().await;
        let config = create_test_session_config("test_session", "test_device");
        
        let result = manager.create_session(config).await;
        assert!(result.is_ok());
        
        let session_id = result.unwrap();
        assert!(!session_id.is_empty());
        assert_eq!(manager.get_session_count().await, 1);
        
        // Check session exists
        assert!(manager.has_session(&session_id).await);
    }
    
    #[tokio::test]
    async fn test_duplicate_session_names() {
        let manager = create_test_manager().await;
        let config1 = create_test_session_config("duplicate", "device1");
        let config2 = create_test_session_config("duplicate", "device2");
        
        // First session should succeed
        let result1 = manager.create_session(config1).await;
        assert!(result1.is_ok());
        
        // Second session with same name should fail
        let result2 = manager.create_session(config2).await;
        assert!(result2.is_err());
    }
    
    #[tokio::test]
    async fn test_session_limit() {
        let manager = create_test_manager().await; // Max 5 sessions
        
        // Create 5 sessions (should succeed)
        for i in 0..5 {
            let config = create_test_session_config(&format!("session_{}", i), &format!("device_{}", i));
            let result = manager.create_session(config).await;
            assert!(result.is_ok());
        }
        
        // 6th session should fail
        let config = create_test_session_config("session_6", "device_6");
        let result = manager.create_session(config).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = create_test_manager().await;
        let config = create_test_session_config("lifecycle_test", "test_device");
        
        // Create session
        let session_id = manager.create_session(config).await.unwrap();
        
        // Start session (will fail due to /dev/null not being a valid serial port)
        let result = manager.start_session(&session_id).await;
        assert!(result.is_err());
        
        // Stop session
        let result = manager.stop_session(&session_id).await;
        assert!(result.is_ok());
        
        // Remove session
        let result = manager.remove_session(&session_id).await;
        assert!(result.is_ok());
        assert_eq!(manager.get_session_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_session_operations_on_nonexistent() {
        let manager = create_test_manager().await;
        
        let result = manager.start_session("nonexistent").await;
        assert!(result.is_err());
        
        let result = manager.stop_session("nonexistent").await;
        assert!(result.is_err());
        
        let result = manager.remove_session("nonexistent").await;
        assert!(result.is_err());
        
        let result = manager.send_data("nonexistent", b"test".to_vec()).await;
        assert!(result.is_err());
        
        let result = manager.send_command("nonexistent", "TEST").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_session_listing() {
        let manager = create_test_manager().await;
        
        // Create a few sessions
        for i in 0..3 {
            let config = create_test_session_config(&format!("session_{}", i), &format!("device_{}", i));
            manager.create_session(config).await.unwrap();
        }
        
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 3);
        
        // Test filtering
        let filter = SessionFilter::new().with_session_type(SessionType::Testing);
        let filtered = manager.list_sessions_filtered(&filter).await;
        assert_eq!(filtered.len(), 3); // All are testing sessions
    }
    
    #[tokio::test]
    async fn test_find_sessions() {
        let manager = create_test_manager().await;
        
        let config1 = create_test_session_config("test_session_1", "device_A");
        let config2 = create_test_session_config("prod_session_1", "device_A");
        let config3 = create_test_session_config("test_session_2", "device_B");
        
        manager.create_session(config1).await.unwrap();
        manager.create_session(config2).await.unwrap();
        manager.create_session(config3).await.unwrap();
        
        // Find by name pattern
        let results = manager.find_sessions_by_name("test").await;
        assert_eq!(results.len(), 2);
        
        // Find by device name
        let results = manager.find_sessions_by_device("device_A").await;
        assert_eq!(results.len(), 2);
    }
    
    #[tokio::test]
    async fn test_session_statistics() {
        let manager = create_test_manager().await;
        
        // Initially empty
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_sessions, 0);
        
        // Create some sessions
        for i in 0..3 {
            let config = create_test_session_config(&format!("session_{}", i), &format!("device_{}", i));
            manager.create_session(config).await.unwrap();
        }
        
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_sessions, 3);
    }
    
    #[tokio::test]
    async fn test_bulk_operations() {
        let manager = create_test_manager().await;
        
        // Create multiple sessions
        for i in 0..3 {
            let config = create_test_session_config(&format!("session_{}", i), &format!("device_{}", i));
            manager.create_session(config).await.unwrap();
        }
        
        assert_eq!(manager.get_session_count().await, 3);
        
        // Stop all sessions
        let result = manager.stop_all_sessions().await;
        assert!(result.is_ok());
        
        // Remove all sessions
        let result = manager.remove_all_sessions().await;
        assert!(result.is_ok());
        assert_eq!(manager.get_session_count().await, 0);
    }
}