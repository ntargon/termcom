use crate::domain::error::{TermComError, TermComResult};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct ServerMessage {
    pub timestamp: std::time::SystemTime,
    pub client_addr: SocketAddr,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ClientConnection {
    pub addr: SocketAddr,
    pub connected_at: std::time::SystemTime,
    pub bytes_received: u64,
    pub bytes_sent: u64,
}

pub struct EchoServer {
    listener: TcpListener,
    bind_addr: SocketAddr,
    clients: Arc<Mutex<Vec<ClientConnection>>>,
    message_sender: mpsc::UnboundedSender<ServerMessage>,
    message_receiver: mpsc::UnboundedReceiver<ServerMessage>,
    shutdown_sender: mpsc::Sender<()>,
    shutdown_receiver: mpsc::Receiver<()>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl EchoServer {
    pub async fn new(bind_addr: &str) -> TermComResult<Self> {
        let listener = TcpListener::bind(bind_addr).await
            .map_err(|e| TermComError::Communication {
                message: format!("Failed to bind to {}: {}", bind_addr, e),
            })?;
        
        let actual_addr = listener.local_addr()
            .map_err(|e| TermComError::Communication {
                message: format!("Failed to get local address: {}", e),
            })?;
        
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let (shutdown_sender, shutdown_receiver) = mpsc::channel(1);
        
        info!("Echo server created on {}", actual_addr);
        
        Ok(Self {
            listener,
            bind_addr: actual_addr,
            clients: Arc::new(Mutex::new(Vec::new())),
            message_sender,
            message_receiver,
            shutdown_sender,
            shutdown_receiver,
            server_handle: None,
        })
    }
    
    pub fn get_bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }
    
    pub async fn start(&mut self) -> TermComResult<()> {
        if self.server_handle.is_some() {
            return Err(TermComError::Communication {
                message: "Server is already running".to_string(),
            });
        }
        
        info!("Starting echo server on {}", self.bind_addr);
        
        let listener = std::mem::replace(&mut self.listener, 
            TcpListener::bind("0.0.0.0:0").await.unwrap()); // Placeholder
        let clients = Arc::clone(&self.clients);
        let message_sender = self.message_sender.clone();
        let mut shutdown_receiver = std::mem::replace(&mut self.shutdown_receiver,
            mpsc::channel(1).1); // Placeholder
        
        let server_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle new connections
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, addr)) => {
                                info!("New client connected: {}", addr);
                                
                                // Add client to tracking
                                {
                                    let mut clients_guard = clients.lock().await;
                                    clients_guard.push(ClientConnection {
                                        addr,
                                        connected_at: std::time::SystemTime::now(),
                                        bytes_received: 0,
                                        bytes_sent: 0,
                                    });
                                }
                                
                                // Spawn handler for this client
                                let clients_for_handler = Arc::clone(&clients);
                                let clients_for_cleanup = Arc::clone(&clients);
                                let message_sender_clone = message_sender.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_client(
                                        stream, 
                                        addr, 
                                        clients_for_handler, 
                                        message_sender_clone
                                    ).await {
                                        error!("Error handling client {}: {}", addr, e);
                                    }
                                    
                                    // Remove client from tracking
                                    let mut clients_guard = clients_for_cleanup.lock().await;
                                    clients_guard.retain(|c| c.addr != addr);
                                    info!("Client disconnected: {}", addr);
                                });
                            }
                            Err(e) => {
                                error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    
                    // Handle shutdown signal
                    _ = shutdown_receiver.recv() => {
                        info!("Received shutdown signal, stopping server");
                        break;
                    }
                }
            }
        });
        
        self.server_handle = Some(server_handle);
        info!("Echo server started successfully");
        
        Ok(())
    }
    
    async fn handle_client(
        mut stream: TcpStream,
        addr: SocketAddr,
        clients: Arc<Mutex<Vec<ClientConnection>>>,
        message_sender: mpsc::UnboundedSender<ServerMessage>,
    ) -> TermComResult<()> {
        let mut buffer = vec![0u8; 4096];
        
        loop {
            match tokio::time::timeout(
                Duration::from_secs(30), // 30 second timeout
                stream.read(&mut buffer)
            ).await {
                Ok(Ok(0)) => {
                    // Client disconnected
                    debug!("Client {} disconnected gracefully", addr);
                    break;
                }
                Ok(Ok(n)) => {
                    let data = buffer[..n].to_vec();
                    debug!("Received {} bytes from {}: {:?}", n, addr, 
                        String::from_utf8_lossy(&data));
                    
                    // Update client stats
                    {
                        let mut clients_guard = clients.lock().await;
                        if let Some(client) = clients_guard.iter_mut().find(|c| c.addr == addr) {
                            client.bytes_received += n as u64;
                        }
                    }
                    
                    // Send message to channel for monitoring
                    if let Err(e) = message_sender.send(ServerMessage {
                        timestamp: std::time::SystemTime::now(),
                        client_addr: addr,
                        data: data.clone(),
                    }) {
                        warn!("Failed to send message to channel: {}", e);
                    }
                    
                    // Echo the data back
                    match stream.write_all(&data).await {
                        Ok(_) => {
                            if let Err(e) = stream.flush().await {
                                error!("Failed to flush stream for {}: {}", addr, e);
                                break;
                            }
                            
                            // Update client stats
                            {
                                let mut clients_guard = clients.lock().await;
                                if let Some(client) = clients_guard.iter_mut().find(|c| c.addr == addr) {
                                    client.bytes_sent += n as u64;
                                }
                            }
                            
                            debug!("Echoed {} bytes back to {}", n, addr);
                        }
                        Err(e) => {
                            error!("Failed to write to stream for {}: {}", addr, e);
                            break;
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("Read error from {}: {}", addr, e);
                    break;
                }
                Err(_) => {
                    // Timeout - check if client is still alive
                    debug!("Read timeout for client {}", addr);
                    
                    // Try to send a keep-alive byte
                    if stream.write_all(&[]).await.is_err() {
                        debug!("Client {} appears to be disconnected (keep-alive failed)", addr);
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn stop(&mut self) -> TermComResult<()> {
        if let Some(handle) = self.server_handle.take() {
            info!("Stopping echo server");
            
            // Send shutdown signal
            if let Err(e) = self.shutdown_sender.send(()).await {
                warn!("Failed to send shutdown signal: {}", e);
            }
            
            // Wait for server task to complete
            if let Err(e) = handle.await {
                warn!("Server task completed with error: {}", e);
            }
            
            info!("Echo server stopped");
        }
        
        Ok(())
    }
    
    pub async fn receive_message(&mut self) -> Option<ServerMessage> {
        self.message_receiver.recv().await
    }
    
    pub async fn get_connected_clients(&self) -> Vec<ClientConnection> {
        let clients = self.clients.lock().await;
        clients.clone()
    }
    
    pub async fn get_client_count(&self) -> usize {
        let clients = self.clients.lock().await;
        clients.len()
    }
    
    pub async fn get_server_stats(&self) -> ServerStats {
        let clients = self.clients.lock().await;
        let total_bytes_received = clients.iter().map(|c| c.bytes_received).sum();
        let total_bytes_sent = clients.iter().map(|c| c.bytes_sent).sum();
        let client_count = clients.len();
        
        ServerStats {
            bind_addr: self.bind_addr,
            client_count,
            total_bytes_received,
            total_bytes_sent,
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default(),
        }
    }
    
    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct ServerStats {
    pub bind_addr: SocketAddr,
    pub client_count: usize,
    pub total_bytes_received: u64,
    pub total_bytes_sent: u64,
    pub uptime: Duration,
}

impl Drop for EchoServer {
    fn drop(&mut self) {
        if self.server_handle.is_some() {
            warn!("EchoServer dropped while still running - server may not shutdown gracefully");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    
    #[tokio::test]
    async fn test_echo_server_creation() {
        let server = EchoServer::new("127.0.0.1:0").await;
        assert!(server.is_ok());
        
        if let Ok(server) = server {
            assert!(!server.is_running());
            assert_eq!(server.get_client_count().await, 0);
        }
    }
    
    #[tokio::test]
    async fn test_echo_server_start_stop() {
        let mut server = EchoServer::new("127.0.0.1:0").await.unwrap();
        let addr = server.get_bind_addr();
        
        // Start server
        let start_result = server.start().await;
        assert!(start_result.is_ok());
        assert!(server.is_running());
        
        // Give server a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Try to start again (should fail)
        let double_start = server.start().await;
        assert!(double_start.is_err());
        
        // Stop server
        let stop_result = server.stop().await;
        assert!(stop_result.is_ok());
        assert!(!server.is_running());
    }
    
    #[tokio::test]
    async fn test_echo_functionality() {
        let mut server = EchoServer::new("127.0.0.1:0").await.unwrap();
        let addr = server.get_bind_addr();
        
        // Start server
        server.start().await.unwrap();
        
        // Give server a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Connect client and test echo
        let mut client = TcpStream::connect(addr).await.unwrap();
        
        let test_data = b"Hello, Echo Server!";
        client.write_all(test_data).await.unwrap();
        client.flush().await.unwrap();
        
        let mut response = vec![0u8; test_data.len()];
        client.read_exact(&mut response).await.unwrap();
        
        assert_eq!(response, test_data);
        
        // Check server stats
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(server.get_client_count().await, 1);
        
        drop(client);
        
        // Give server time to detect disconnection
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Stop server
        server.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_multiple_clients() {
        let mut server = EchoServer::new("127.0.0.1:0").await.unwrap();
        let addr = server.get_bind_addr();
        
        server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Connect multiple clients
        let mut clients = Vec::new();
        for i in 0..3 {
            let mut client = TcpStream::connect(addr).await.unwrap();
            let test_data = format!("Message from client {}", i);
            
            client.write_all(test_data.as_bytes()).await.unwrap();
            client.flush().await.unwrap();
            
            let mut response = vec![0u8; test_data.len()];
            client.read_exact(&mut response).await.unwrap();
            
            assert_eq!(response, test_data.as_bytes());
            clients.push(client);
        }
        
        // Check client count
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(server.get_client_count().await, 3);
        
        // Disconnect all clients
        drop(clients);
        
        // Give server time to detect disconnections
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        server.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_server_message_channel() {
        let mut server = EchoServer::new("127.0.0.1:0").await.unwrap();
        let addr = server.get_bind_addr();
        
        server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let mut client = TcpStream::connect(addr).await.unwrap();
        let test_data = b"Test message";
        
        client.write_all(test_data).await.unwrap();
        client.flush().await.unwrap();
        
        // Receive the echoed data
        let mut response = vec![0u8; test_data.len()];
        client.read_exact(&mut response).await.unwrap();
        
        // Check if we can receive the message through the channel
        tokio::time::sleep(Duration::from_millis(10)).await;
        let message = server.receive_message().await;
        assert!(message.is_some());
        
        if let Some(msg) = message {
            assert_eq!(msg.data, test_data);
        }
        
        drop(client);
        server.stop().await.unwrap();
    }
    
    #[test]
    fn test_server_message_creation() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let message = ServerMessage {
            timestamp: std::time::SystemTime::now(),
            client_addr: addr,
            data: vec![1, 2, 3],
        };
        
        assert_eq!(message.client_addr, addr);
        assert_eq!(message.data, vec![1, 2, 3]);
    }
    
    #[test]
    fn test_client_connection_creation() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let connection = ClientConnection {
            addr,
            connected_at: std::time::SystemTime::now(),
            bytes_received: 100,
            bytes_sent: 200,
        };
        
        assert_eq!(connection.addr, addr);
        assert_eq!(connection.bytes_received, 100);
        assert_eq!(connection.bytes_sent, 200);
    }
}