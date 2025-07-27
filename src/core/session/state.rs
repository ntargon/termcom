use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use std::collections::HashMap;

/// Session state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session ID
    pub session_id: String,
    /// Device name
    pub device_name: String,
    /// Current status
    pub status: SessionStatus,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Session statistics
    pub statistics: SessionStatistics,
    /// Session metadata
    pub metadata: SessionMetadata,
}

/// Session status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    /// Session is initializing
    Initializing,
    /// Session is active and connected
    Active,
    /// Session is temporarily disconnected
    Disconnected,
    /// Session is being closed
    Closing,
    /// Session is closed
    Closed,
    /// Session encountered an error
    Error(String),
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Number of errors
    pub error_count: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Last response time in milliseconds
    pub last_response_time_ms: Option<u64>,
    /// Connection uptime
    pub uptime: Duration,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Transport type
    pub transport_type: String,
    /// Connection parameters
    pub connection_params: HashMap<String, String>,
    /// Custom tags
    pub tags: Vec<String>,
    /// User-defined properties
    pub properties: HashMap<String, String>,
    /// Session configuration name
    pub config_name: Option<String>,
}

/// Session activity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionActivity {
    /// Activity timestamp
    pub timestamp: SystemTime,
    /// Activity type
    pub activity_type: ActivityType,
    /// Activity description
    pub description: String,
    /// Related data size
    pub data_size: Option<usize>,
    /// Duration for timed activities
    pub duration: Option<Duration>,
}

/// Activity type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivityType {
    /// Session was created
    Created,
    /// Connection established
    Connected,
    /// Data was sent
    DataSent,
    /// Data was received
    DataReceived,
    /// Command was executed
    CommandExecuted,
    /// Response was received
    ResponseReceived,
    /// Connection was lost
    ConnectionLost,
    /// Session was closed
    Closed,
    /// Error occurred
    Error,
    /// Custom activity
    Custom(String),
}

impl SessionState {
    /// Create a new session state
    pub fn new(session_id: String, device_name: String, transport_type: String) -> Self {
        let now = SystemTime::now();
        
        Self {
            session_id,
            device_name,
            status: SessionStatus::Initializing,
            created_at: now,
            last_activity: now,
            statistics: SessionStatistics::default(),
            metadata: SessionMetadata {
                transport_type,
                connection_params: HashMap::new(),
                tags: Vec::new(),
                properties: HashMap::new(),
                config_name: None,
            },
        }
    }
    
    /// Update session status
    pub fn update_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.last_activity = SystemTime::now();
    }
    
    /// Record activity
    pub fn record_activity(&mut self, activity: SessionActivity) {
        self.last_activity = activity.timestamp;
        
        // Update statistics based on activity type
        match activity.activity_type {
            ActivityType::DataSent => {
                self.statistics.messages_sent += 1;
                if let Some(size) = activity.data_size {
                    self.statistics.bytes_sent += size as u64;
                }
            }
            ActivityType::DataReceived => {
                self.statistics.messages_received += 1;
                if let Some(size) = activity.data_size {
                    self.statistics.bytes_received += size as u64;
                }
            }
            ActivityType::Error => {
                self.statistics.error_count += 1;
            }
            ActivityType::ResponseReceived => {
                self.statistics.messages_received += 1;
                if let Some(duration) = activity.duration {
                    let duration_ms = duration.as_millis() as u64;
                    self.statistics.last_response_time_ms = Some(duration_ms);
                    
                    // Update average response time
                    let total_responses = self.statistics.messages_received;
                    if total_responses > 1 {
                        self.statistics.avg_response_time_ms = 
                            (self.statistics.avg_response_time_ms * (total_responses - 1) as f64 + duration_ms as f64) / total_responses as f64;
                    } else {
                        self.statistics.avg_response_time_ms = duration_ms as f64;
                    }
                }
            }
            _ => {}
        }
    }
    
    /// Add a tag to the session
    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
        }
    }
    
    /// Add a property to the session
    pub fn add_property(&mut self, key: String, value: String) {
        self.metadata.properties.insert(key, value);
    }
    
    /// Set connection parameter
    pub fn set_connection_param(&mut self, key: String, value: String) {
        self.metadata.connection_params.insert(key, value);
    }
    
    /// Get session uptime
    pub fn get_uptime(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default()
    }
    
    /// Get idle time (time since last activity)
    pub fn get_idle_time(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.last_activity)
            .unwrap_or_default()
    }
    
    /// Check if session is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }
    
    /// Check if session is closed
    pub fn is_closed(&self) -> bool {
        matches!(self.status, SessionStatus::Closed)
    }
    
    /// Check if session has error
    pub fn has_error(&self) -> bool {
        matches!(self.status, SessionStatus::Error(_))
    }
    
    /// Get error message if any
    pub fn get_error_message(&self) -> Option<&str> {
        match &self.status {
            SessionStatus::Error(msg) => Some(msg),
            _ => None,
        }
    }
    
    /// Update statistics manually
    pub fn update_statistics<F>(&mut self, updater: F)
    where
        F: FnOnce(&mut SessionStatistics),
    {
        updater(&mut self.statistics);
        self.last_activity = SystemTime::now();
    }
}

impl Default for SessionStatistics {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            error_count: 0,
            avg_response_time_ms: 0.0,
            last_response_time_ms: None,
            uptime: Duration::new(0, 0),
        }
    }
}

impl SessionActivity {
    /// Create a new activity
    pub fn new(activity_type: ActivityType, description: String) -> Self {
        Self {
            timestamp: SystemTime::now(),
            activity_type,
            description,
            data_size: None,
            duration: None,
        }
    }
    
    /// Create activity with data size
    pub fn with_data_size(mut self, size: usize) -> Self {
        self.data_size = Some(size);
        self
    }
    
    /// Create activity with duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
    
    /// Create a data sent activity
    pub fn data_sent(description: String, size: usize) -> Self {
        Self::new(ActivityType::DataSent, description).with_data_size(size)
    }
    
    /// Create a data received activity
    pub fn data_received(description: String, size: usize) -> Self {
        Self::new(ActivityType::DataReceived, description).with_data_size(size)
    }
    
    /// Create a command executed activity
    pub fn command_executed(command: String) -> Self {
        Self::new(ActivityType::CommandExecuted, format!("Executed command: {}", command))
    }
    
    /// Create a response received activity
    pub fn response_received(description: String, duration: Duration) -> Self {
        Self::new(ActivityType::ResponseReceived, description).with_duration(duration)
    }
    
    /// Create an error activity
    pub fn error(error_message: String) -> Self {
        Self::new(ActivityType::Error, format!("Error: {}", error_message))
    }
    
    /// Create a custom activity
    pub fn custom(activity_name: String, description: String) -> Self {
        Self::new(ActivityType::Custom(activity_name), description)
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Initializing => write!(f, "Initializing"),
            SessionStatus::Active => write!(f, "Active"),
            SessionStatus::Disconnected => write!(f, "Disconnected"),
            SessionStatus::Closing => write!(f, "Closing"),
            SessionStatus::Closed => write!(f, "Closed"),
            SessionStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityType::Created => write!(f, "Created"),
            ActivityType::Connected => write!(f, "Connected"),
            ActivityType::DataSent => write!(f, "Data Sent"),
            ActivityType::DataReceived => write!(f, "Data Received"),
            ActivityType::CommandExecuted => write!(f, "Command Executed"),
            ActivityType::ResponseReceived => write!(f, "Response Received"),
            ActivityType::ConnectionLost => write!(f, "Connection Lost"),
            ActivityType::Closed => write!(f, "Closed"),
            ActivityType::Error => write!(f, "Error"),
            ActivityType::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_state_creation() {
        let state = SessionState::new(
            "test_session".to_string(),
            "test_device".to_string(),
            "serial".to_string(),
        );
        
        assert_eq!(state.session_id, "test_session");
        assert_eq!(state.device_name, "test_device");
        assert_eq!(state.metadata.transport_type, "serial");
        assert!(matches!(state.status, SessionStatus::Initializing));
        assert!(state.is_active() == false);
        assert!(state.is_closed() == false);
    }
    
    #[test]
    fn test_status_updates() {
        let mut state = SessionState::new(
            "test_session".to_string(),
            "test_device".to_string(),
            "tcp".to_string(),
        );
        
        state.update_status(SessionStatus::Active);
        assert!(state.is_active());
        assert!(!state.has_error());
        
        state.update_status(SessionStatus::Error("Connection failed".to_string()));
        assert!(state.has_error());
        assert_eq!(state.get_error_message(), Some("Connection failed"));
        
        state.update_status(SessionStatus::Closed);
        assert!(state.is_closed());
    }
    
    #[test]
    fn test_activity_recording() {
        let mut state = SessionState::new(
            "test_session".to_string(),
            "test_device".to_string(),
            "serial".to_string(),
        );
        
        // Record data sent activity
        let activity = SessionActivity::data_sent("Test data".to_string(), 100);
        state.record_activity(activity);
        
        assert_eq!(state.statistics.messages_sent, 1);
        assert_eq!(state.statistics.bytes_sent, 100);
        
        // Record data received activity
        let activity = SessionActivity::data_received("Response data".to_string(), 50);
        state.record_activity(activity);
        
        assert_eq!(state.statistics.messages_received, 1);
        assert_eq!(state.statistics.bytes_received, 50);
        
        // Record error activity
        let activity = SessionActivity::error("Test error".to_string());
        state.record_activity(activity);
        
        assert_eq!(state.statistics.error_count, 1);
    }
    
    #[test]
    fn test_response_time_tracking() {
        let mut state = SessionState::new(
            "test_session".to_string(),
            "test_device".to_string(),
            "tcp".to_string(),
        );
        
        // Record first response
        let activity = SessionActivity::response_received(
            "First response".to_string(),
            Duration::from_millis(100),
        );
        state.record_activity(activity);
        
        assert_eq!(state.statistics.last_response_time_ms, Some(100));
        assert_eq!(state.statistics.avg_response_time_ms, 100.0);
        
        // Record second response
        let activity = SessionActivity::response_received(
            "Second response".to_string(),
            Duration::from_millis(200),
        );
        state.record_activity(activity);
        
        assert_eq!(state.statistics.last_response_time_ms, Some(200));
        assert_eq!(state.statistics.avg_response_time_ms, 150.0);
    }
    
    #[test]
    fn test_metadata_management() {
        let mut state = SessionState::new(
            "test_session".to_string(),
            "test_device".to_string(),
            "serial".to_string(),
        );
        
        // Add tags
        state.add_tag("important".to_string());
        state.add_tag("test".to_string());
        state.add_tag("important".to_string()); // Duplicate should be ignored
        
        assert_eq!(state.metadata.tags.len(), 2);
        assert!(state.metadata.tags.contains(&"important".to_string()));
        assert!(state.metadata.tags.contains(&"test".to_string()));
        
        // Add properties
        state.add_property("priority".to_string(), "high".to_string());
        state.add_property("owner".to_string(), "user1".to_string());
        
        assert_eq!(state.metadata.properties.len(), 2);
        assert_eq!(state.metadata.properties.get("priority"), Some(&"high".to_string()));
        
        // Set connection parameters
        state.set_connection_param("baud_rate".to_string(), "9600".to_string());
        state.set_connection_param("data_bits".to_string(), "8".to_string());
        
        assert_eq!(state.metadata.connection_params.len(), 2);
    }
    
    #[test]
    fn test_time_tracking() {
        let state = SessionState::new(
            "test_session".to_string(),
            "test_device".to_string(),
            "tcp".to_string(),
        );
        
        // Test uptime (should be very small since just created)
        let uptime = state.get_uptime();
        assert!(uptime.as_millis() < 100);
        
        // Test idle time (should be very small since just created)
        let idle_time = state.get_idle_time();
        assert!(idle_time.as_millis() < 100);
    }
}