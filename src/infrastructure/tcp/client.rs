use crate::domain::{config::ConnectionConfig, error::{TermComError, TermComResult}};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct TcpMessage {
    pub timestamp: std::time::SystemTime,
    pub direction: MessageDirection,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum MessageDirection {
    Sent,
    Received,
}

pub struct TcpClient {
    stream: Arc<Mutex<TcpStream>>,
    tx_sender: mpsc::UnboundedSender<Vec<u8>>,
    message_receiver: mpsc::UnboundedReceiver<TcpMessage>,
    _tx_handle: tokio::task::JoinHandle<()>,
    _rx_handle: tokio::task::JoinHandle<()>,
}

impl TcpClient {
    pub async fn new(config: &ConnectionConfig) -> TermComResult<Self> {
        let (host, port, timeout_ms, keep_alive) = match config {
            ConnectionConfig::Tcp { 
                host, 
                port, 
                timeout_ms, 
                keep_alive 
            } => (host.clone(), *port, *timeout_ms, *keep_alive),
            _ => return Err(TermComError::Communication {
                message: "Invalid connection type for TCP client".to_string(),
            }),
        };
        
        // Connect with timeout
        let stream = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            TcpStream::connect((host.as_str(), port))
        ).await
        .map_err(|_| TermComError::Communication {
            message: format!("Connection timeout to {}:{}", host, port),
        })?
        .map_err(|e| TermComError::Communication {
            message: format!("Failed to connect to {}:{}: {}", host, port, e),
        })?;
        
        // Configure socket options
        if keep_alive {
            if let Err(e) = stream.set_nodelay(true) {
                warn!("Failed to set TCP_NODELAY: {}", e);
            }
        }
        
        info!("TCP connection established to {}:{}", host, port);
        
        let stream = Arc::new(Mutex::new(stream));
        let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel::<Vec<u8>>();
        let (message_sender, message_receiver) = mpsc::unbounded_channel::<TcpMessage>();
        let message_sender_tx = message_sender.clone();
        let message_sender_rx = message_sender;
        
        // Clone stream for background tasks
        let stream_tx = Arc::clone(&stream);
        let stream_rx = Arc::clone(&stream);
        
        // TX task - handles outgoing messages
        let tx_handle = tokio::spawn(async move {
            while let Some(data) = tx_receiver.recv().await {
                let mut stream = stream_tx.lock().await;
                match stream.write_all(&data).await {
                    Ok(_) => {
                        if let Err(e) = stream.flush().await {
                            error!("Failed to flush TCP stream: {}", e);
                        } else {
                            debug!("Sent {} bytes over TCP", data.len());
                            if let Err(e) = message_sender_tx.send(TcpMessage {
                                timestamp: std::time::SystemTime::now(),
                                direction: MessageDirection::Sent,
                                data,
                            }) {
                                error!("Failed to send message to channel: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to write to TCP stream: {}", e);
                        break;
                    }
                }
            }
        });
        
        // RX task - handles incoming messages
        let rx_handle = tokio::spawn(async move {
            let mut buffer = vec![0u8; 4096];
            
            loop {
                let mut stream = stream_rx.lock().await;
                
                match tokio::time::timeout(
                    Duration::from_millis(100),
                    stream.read(&mut buffer)
                ).await {
                    Ok(Ok(0)) => {
                        // Connection closed by peer
                        info!("TCP connection closed by peer");
                        break;
                    }
                    Ok(Ok(n)) => {
                        let data = buffer[..n].to_vec();
                        debug!("Received {} bytes over TCP", n);
                        
                        if let Err(e) = message_sender_rx.send(TcpMessage {
                            timestamp: std::time::SystemTime::now(),
                            direction: MessageDirection::Received,
                            data,
                        }) {
                            error!("Failed to send received message to channel: {}", e);
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Failed to read from TCP stream: {}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout - continue reading
                        continue;
                    }
                }
            }
        });
        
        Ok(Self {
            stream,
            tx_sender,
            message_receiver,
            _tx_handle: tx_handle,
            _rx_handle: rx_handle,
        })
    }
    
    pub async fn send(&self, data: Vec<u8>) -> TermComResult<()> {
        self.tx_sender.send(data).map_err(|e| TermComError::Communication {
            message: format!("Failed to send data to TCP tx channel: {}", e),
        })?;
        
        Ok(())
    }
    
    pub async fn receive(&mut self) -> Option<TcpMessage> {
        self.message_receiver.recv().await
    }
    
    pub async fn send_command(&self, command: &str) -> TermComResult<()> {
        let data = command.as_bytes().to_vec();
        self.send(data).await
    }
    
    pub async fn is_connected(&self) -> bool {
        // Try to peek at the stream to check if it's still connected
        let _stream = self.stream.lock().await;
        // Simple check - if we can lock the stream, assume it's connected
        // More sophisticated connection checking could be added here
        true
    }
    
    pub async fn close(self) -> TermComResult<()> {
        // Drop the sender to signal background tasks to exit
        drop(self.tx_sender);
        
        // Shutdown the stream
        {
            let mut stream = self.stream.lock().await;
            if let Err(e) = stream.shutdown().await {
                warn!("Failed to shutdown TCP stream: {}", e);
            }
        }
        
        // Wait for background tasks to complete
        if let Err(e) = self._tx_handle.await {
            warn!("TX task completed with error: {}", e);
        }
        
        self._rx_handle.abort();
        
        info!("TCP client closed");
        Ok(())
    }
    
    pub async fn get_peer_addr(&self) -> Option<std::net::SocketAddr> {
        let stream = self.stream.lock().await;
        stream.peer_addr().ok()
    }
    
    pub async fn get_local_addr(&self) -> Option<std::net::SocketAddr> {
        let stream = self.stream.lock().await;
        stream.local_addr().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::ConnectionConfig;
    use tokio::net::TcpListener;
    
    fn create_test_config(port: u16) -> ConnectionConfig {
        ConnectionConfig::Tcp {
            host: "127.0.0.1".to_string(),
            port,
            timeout_ms: 1000,
            keep_alive: true,
        }
    }
    
    #[tokio::test]
    async fn test_tcp_client_creation_fails_gracefully() {
        let config = create_test_config(0); // Invalid port
        
        // This should fail because port 0 is not connectable
        let result = TcpClient::new(&config).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_tcp_client_with_mock_server() {
        // Start a simple echo server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        // Start server task
        let _server_handle = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0; 1024];
                if let Ok(n) = socket.read(&mut buf).await {
                    let _ = socket.write_all(&buf[0..n]).await;
                }
            }
        });
        
        // Give server a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let config = ConnectionConfig::Tcp {
            host: addr.ip().to_string(),
            port: addr.port(),
            timeout_ms: 1000,
            keep_alive: true,
        };
        
        let client = TcpClient::new(&config).await;
        assert!(client.is_ok());
        
        if let Ok(client) = client {
            let _ = client.close().await;
        }
    }
    
    #[test]
    fn test_tcp_message_creation() {
        let message = TcpMessage {
            timestamp: std::time::SystemTime::now(),
            direction: MessageDirection::Sent,
            data: vec![0x01, 0x02, 0x03],
        };
        
        assert_eq!(message.data.len(), 3);
        assert!(matches!(message.direction, MessageDirection::Sent));
    }
    
    #[tokio::test]
    async fn test_tcp_client_timeout() {
        // Try to connect to a non-routable address (should timeout)
        let config = ConnectionConfig::Tcp {
            host: "192.0.2.1".to_string(), // TEST-NET-1 (RFC 5737) - should be non-routable
            port: 12345,
            timeout_ms: 100, // Very short timeout
            keep_alive: false,
        };
        
        let result = TcpClient::new(&config).await;
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("timeout") || e.to_string().contains("Connection"));
        }
    }
}