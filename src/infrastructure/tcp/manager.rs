use crate::domain::{config::DeviceConfig, error::{TermComError, TermComResult}};
use crate::infrastructure::tcp::client::{TcpClient, TcpMessage};
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
    pub peer_addr: Option<std::net::SocketAddr>,
    pub local_addr: Option<std::net::SocketAddr>,
}

#[derive(Debug, Clone)]
pub enum SessionStatus {
    Connected,
    Disconnected,
    Error(String),
}

pub struct SessionHandle {
    client: TcpClient,
    info: SessionInfo,
    message_sender: mpsc::UnboundedSender<TcpMessage>,
}

pub struct TcpManager {
    sessions: Arc<RwLock<HashMap<SessionId, SessionHandle>>>,
    max_sessions: usize,
    message_receiver: mpsc::UnboundedReceiver<TcpMessage>,
    message_sender: mpsc::UnboundedSender<TcpMessage>,
}

impl TcpManager {
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
        
        let client = TcpClient::new(&device_config.connection).await?;
        let session_id = format!("tcp_{}", uuid::Uuid::new_v4().simple());
        
        // Get connection info
        let peer_addr = client.get_peer_addr().await;
        let local_addr = client.get_local_addr().await;
        
        let session_info = SessionInfo {
            id: session_id.clone(),
            device_name: device_config.name.clone(),
            status: SessionStatus::Connected,
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
            peer_addr,
            local_addr,
        };
        
        let session_handle = SessionHandle {
            client,
            info: session_info,
            message_sender: self.message_sender.clone(),
        };
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session_handle);
        
        info!("Created TCP session '{}' for device '{}'", session_id, device_config.name);
        
        Ok(session_id)
    }
    
    pub async fn close_session(&self, session_id: &SessionId) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session_handle) = sessions.remove(session_id) {
            session_handle.client.close().await?;
            info!("Closed TCP session '{}'", session_id);
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
    
    pub async fn receive_message(&mut self) -> Option<TcpMessage> {
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
        
        info!("Closed all TCP sessions");
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
    
    pub async fn get_session_stats(&self, session_id: &SessionId) -> Option<SessionStats> {
        let sessions = self.sessions.read().await;
        if let Some(session_handle) = sessions.get(session_id) {
            let info = &session_handle.info;
            Some(SessionStats {
                session_id: info.id.clone(),
                device_name: info.device_name.clone(),
                status: info.status.clone(),
                created_at: info.created_at,
                last_activity: info.last_activity,
                peer_addr: info.peer_addr,
                local_addr: info.local_addr,
                uptime: std::time::SystemTime::now()
                    .duration_since(info.created_at)
                    .unwrap_or_default(),
            })
        } else {
            None
        }
    }
    
    pub async fn update_session_status(&self, session_id: &SessionId, status: SessionStatus) -> TermComResult<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session_handle) = sessions.get_mut(session_id) {
            session_handle.info.status = status;
            session_handle.info.last_activity = std::time::SystemTime::now();
            Ok(())
        } else {
            Err(TermComError::Communication {
                message: format!("Session '{}' not found", session_id),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: SessionId,
    pub device_name: String,
    pub status: SessionStatus,
    pub created_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
    pub peer_addr: Option<std::net::SocketAddr>,
    pub local_addr: Option<std::net::SocketAddr>,
    pub uptime: std::time::Duration,
}

impl Drop for TcpManager {
    fn drop(&mut self) {
        // Note: We can't use async in Drop, so we'll just log a warning
        // The actual cleanup should be done explicitly by calling close_all_sessions
        warn!("TcpManager dropped - sessions may not be properly closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ConnectionConfig, DeviceConfig};
    use tokio::net::TcpListener;
    use std::time::Duration;
    
    fn create_test_device_config(name: &str, port: u16) -> DeviceConfig {
        DeviceConfig {
            name: name.to_string(),
            description: "Test TCP device".to_string(),
            connection: ConnectionConfig::Tcp {
                host: "127.0.0.1".to_string(),
                port,
                timeout_ms: 1000,
                keep_alive: true,
            },
            commands: Vec::new(),
        }
    }
    
    #[tokio::test]
    async fn test_tcp_manager_creation() {
        let manager = TcpManager::new(5);
        assert_eq!(manager.get_max_sessions(), 5);
        assert_eq!(manager.get_session_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_session_creation_fails_gracefully() {
        let manager = TcpManager::new(5);
        let device_config = create_test_device_config("test_device", 0);
        
        // This should fail because port 0 is not connectable
        let result = manager.create_session(&device_config).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_session_creation_with_mock_server() {
        // Start a mock server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        // Start server task that accepts one connection
        let _server_handle = tokio::spawn(async move {
            if let Ok((_socket, _)) = listener.accept().await {
                // Just accept the connection
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        // Give server a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let manager = TcpManager::new(5);
        let device_config = create_test_device_config("test_device", addr.port());
        
        let result = manager.create_session(&device_config).await;
        assert!(result.is_ok());
        
        if let Ok(session_id) = result {
            assert_eq!(manager.get_session_count().await, 1);
            
            let session_info = manager.get_session_info(&session_id).await;
            assert!(session_info.is_some());
            
            if let Some(info) = session_info {
                assert_eq!(info.device_name, "test_device");
                assert!(matches!(info.status, SessionStatus::Connected));
            }
            
            // Clean up
            let _ = manager.close_session(&session_id).await;
        }
    }
    
    #[tokio::test]
    async fn test_session_info() {
        let session_info = SessionInfo {
            id: "test_session".to_string(),
            device_name: "test_device".to_string(),
            status: SessionStatus::Connected,
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
            peer_addr: None,
            local_addr: None,
        };
        
        assert_eq!(session_info.id, "test_session");
        assert_eq!(session_info.device_name, "test_device");
        assert!(matches!(session_info.status, SessionStatus::Connected));
    }
    
    #[tokio::test]
    async fn test_close_nonexistent_session() {
        let manager = TcpManager::new(5);
        let result = manager.close_session(&"nonexistent".to_string()).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_max_sessions_limit() {
        let manager = TcpManager::new(1); // Only allow 1 session
        
        // Start two mock servers
        let listener1 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr1 = listener1.local_addr().unwrap();
        let listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr2 = listener2.local_addr().unwrap();
        
        let _server1 = tokio::spawn(async move {
            if let Ok((_socket, _)) = listener1.accept().await {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        let _server2 = tokio::spawn(async move {
            if let Ok((_socket, _)) = listener2.accept().await {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let device1 = create_test_device_config("device1", addr1.port());
        let device2 = create_test_device_config("device2", addr2.port());
        
        // First session should succeed
        let result1 = manager.create_session(&device1).await;
        assert!(result1.is_ok());
        
        // Second session should fail due to limit
        let result2 = manager.create_session(&device2).await;
        assert!(result2.is_err());
        
        if let Err(e) = result2 {
            assert!(e.to_string().contains("Maximum number"));
        }
        
        // Clean up
        if let Ok(session_id) = result1 {
            let _ = manager.close_session(&session_id).await;
        }
    }
}