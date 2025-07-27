use crate::domain::{config::DeviceConfig, error::{TermComError, TermComResult}};
use crate::infrastructure::serial::client::{SerialClient, SerialMessage};
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use tracing::{error, info, warn};

pub type SessionId = String;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub device_name: String,
    pub status: SessionStatus,
    pub created_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub enum SessionStatus {
    Connected,
    Disconnected,
    Error(String),
}

pub struct SessionHandle {
    client: SerialClient,
    info: SessionInfo,
    #[allow(dead_code)]
    message_sender: mpsc::UnboundedSender<SerialMessage>,
}

pub struct SerialManager {
    sessions: Arc<RwLock<HashMap<SessionId, SessionHandle>>>,
    max_sessions: usize,
    message_receiver: mpsc::UnboundedReceiver<SerialMessage>,
    message_sender: mpsc::UnboundedSender<SerialMessage>,
}

impl SerialManager {
    pub fn new(max_sessions: usize) -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions,
            message_receiver,
            message_sender,
        }
    }
    
    pub async fn create_session(&self, device_config: &DeviceConfig) -> TermComResult<SessionId> {
        let sessions = self.sessions.read().await;
        
        if sessions.len() >= self.max_sessions {
            return Err(TermComError::Communication {
                message: format!("Maximum number of sessions ({}) reached", self.max_sessions),
            });
        }
        
        // Check if device is already connected
        if sessions.values().any(|handle| handle.info.device_name == device_config.name) {
            return Err(TermComError::Communication {
                message: format!("Device '{}' is already connected", device_config.name),
            });
        }
        
        drop(sessions);
        
        let client = SerialClient::new(&device_config.connection).await?;
        let session_id = format!("serial_{}", uuid::Uuid::new_v4().simple());
        
        let session_info = SessionInfo {
            id: session_id.clone(),
            device_name: device_config.name.clone(),
            status: SessionStatus::Connected,
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        };
        
        let session_handle = SessionHandle {
            client,
            info: session_info,
            message_sender: self.message_sender.clone(),
        };
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session_handle);
        
        info!("Created serial session '{}' for device '{}'", session_id, device_config.name);
        
        Ok(session_id)
    }
    
    pub async fn close_session(&self, session_id: &SessionId) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session_handle) = sessions.remove(session_id) {
            session_handle.client.close().await?;
            info!("Closed serial session '{}'", session_id);
            Ok(())
        } else {
            Err(TermComError::Communication {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    pub async fn send_data(&self, session_id: &SessionId, data: Vec<u8>) -> TermComResult<()> {
        let sessions = self.sessions.read().await;
        
        if let Some(session_handle) = sessions.get(session_id) {
            session_handle.client.send(data).await?;
            
            // Update last activity
            drop(sessions);
            let mut sessions = self.sessions.write().await;
            if let Some(session_handle) = sessions.get_mut(session_id) {
                session_handle.info.last_activity = std::time::SystemTime::now();
            }
            
            Ok(())
        } else {
            Err(TermComError::Communication {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
    
    pub async fn send_command(&self, session_id: &SessionId, command: &str) -> TermComResult<()> {
        let data = command.as_bytes().to_vec();
        self.send_data(session_id, data).await
    }
    
    pub async fn get_session_info(&self, session_id: &SessionId) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|handle| handle.info.clone())
    }
    
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|handle| handle.info.clone()).collect()
    }
    
    pub async fn receive_message(&mut self) -> Option<SerialMessage> {
        self.message_receiver.recv().await
    }
    
    pub async fn close_all_sessions(&self) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        let session_ids: Vec<_> = sessions.keys().cloned().collect();
        
        for session_id in session_ids {
            if let Some(session_handle) = sessions.remove(&session_id) {
                if let Err(e) = session_handle.client.close().await {
                    error!("Failed to close session '{}': {}", session_id, e);
                }
            }
        }
        
        info!("Closed all serial sessions");
        Ok(())
    }
    
    pub async fn is_session_connected(&self, session_id: &SessionId) -> bool {
        let sessions = self.sessions.read().await;
        if let Some(session_handle) = sessions.get(session_id) {
            session_handle.client.is_connected().await
        } else {
            false
        }
    }
    
    pub async fn get_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
    
    pub fn get_max_sessions(&self) -> usize {
        self.max_sessions
    }
}

impl Drop for SerialManager {
    fn drop(&mut self) {
        // Note: We can't use async in Drop, so we'll just log a warning
        // The actual cleanup should be done explicitly by calling close_all_sessions
        warn!("SerialManager dropped - sessions may not be properly closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ConnectionConfig, ParityConfig, FlowControlConfig};
    
    fn create_test_device_config(name: &str) -> DeviceConfig {
        DeviceConfig {
            name: name.to_string(),
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
        }
    }
    
    #[tokio::test]
    async fn test_serial_manager_creation() {
        let manager = SerialManager::new(5);
        assert_eq!(manager.get_max_sessions(), 5);
        assert_eq!(manager.get_session_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_session_creation_fails_gracefully() {
        let manager = SerialManager::new(5);
        let device_config = create_test_device_config("test_device");
        
        // This should fail because /dev/null is not a valid serial port
        let result = manager.create_session(&device_config).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_session_info() {
        let session_info = SessionInfo {
            id: "test_session".to_string(),
            device_name: "test_device".to_string(),
            status: SessionStatus::Connected,
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        };
        
        assert_eq!(session_info.id, "test_session");
        assert_eq!(session_info.device_name, "test_device");
        assert!(matches!(session_info.status, SessionStatus::Connected));
    }
    
    #[tokio::test]
    async fn test_close_nonexistent_session() {
        let manager = SerialManager::new(5);
        let result = manager.close_session(&"nonexistent".to_string()).await;
        assert!(result.is_err());
    }
}