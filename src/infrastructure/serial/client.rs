use crate::domain::{config::ConnectionConfig, error::{TermComError, TermComResult}};
use serialport::SerialPort;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct SerialMessage {
    pub timestamp: std::time::SystemTime,
    pub direction: MessageDirection,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum MessageDirection {
    Sent,
    Received,
}

pub struct SerialClient {
    port: Arc<Mutex<Box<dyn SerialPort + Send>>>,
    tx_sender: mpsc::UnboundedSender<Vec<u8>>,
    message_receiver: mpsc::UnboundedReceiver<SerialMessage>,
    _tx_handle: tokio::task::JoinHandle<()>,
    _rx_handle: tokio::task::JoinHandle<()>,
}

impl SerialClient {
    pub async fn new(config: &ConnectionConfig) -> TermComResult<Self> {
        let serial_config = match config {
            ConnectionConfig::Serial { 
                port, 
                baud_rate, 
                data_bits, 
                stop_bits, 
                parity, 
                flow_control 
            } => {
                let mut builder = serialport::new(port, *baud_rate);
                
                builder = builder.data_bits(match data_bits {
                    5 => serialport::DataBits::Five,
                    6 => serialport::DataBits::Six,
                    7 => serialport::DataBits::Seven,
                    8 => serialport::DataBits::Eight,
                    _ => return Err(TermComError::Communication {
                        message: format!("Invalid data bits: {}", data_bits),
                    }),
                });
                
                builder = builder.stop_bits(match stop_bits {
                    1 => serialport::StopBits::One,
                    2 => serialport::StopBits::Two,
                    _ => return Err(TermComError::Communication {
                        message: format!("Invalid stop bits: {}", stop_bits),
                    }),
                });
                
                builder = builder.parity(match parity {
                    crate::domain::config::ParityConfig::None => serialport::Parity::None,
                    crate::domain::config::ParityConfig::Even => serialport::Parity::Even,
                    crate::domain::config::ParityConfig::Odd => serialport::Parity::Odd,
                });
                
                builder = builder.flow_control(match flow_control {
                    crate::domain::config::FlowControlConfig::None => serialport::FlowControl::None,
                    crate::domain::config::FlowControlConfig::Software => serialport::FlowControl::Software,
                    crate::domain::config::FlowControlConfig::Hardware => serialport::FlowControl::Hardware,
                });
                
                builder = builder.timeout(Duration::from_millis(100));
                
                builder
            }
            _ => return Err(TermComError::Communication {
                message: "Invalid connection type for serial client".to_string(),
            }),
        };
        
        let port = serial_config.open().map_err(|e| TermComError::Communication {
            message: format!("Failed to open serial port: {}", e),
        })?;
        
        info!("Serial port opened successfully");
        
        let port: Arc<Mutex<Box<dyn SerialPort + Send>>> = Arc::new(Mutex::new(port));
        let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel::<Vec<u8>>();
        let (message_sender, message_receiver) = mpsc::unbounded_channel::<SerialMessage>();
        let message_sender_tx = message_sender.clone();
        let message_sender_rx = message_sender;
        
        // Clone port for background tasks
        let port_tx = Arc::clone(&port);
        let port_rx = Arc::clone(&port);
        
        // TX task - handles outgoing messages
        let tx_handle = tokio::spawn(async move {
            while let Some(data) = tx_receiver.recv().await {
                let mut port = port_tx.lock().await;
                match port.write_all(&data) {
                    Ok(_) => {
                        debug!("Sent {} bytes over serial", data.len());
                        if let Err(e) = message_sender_tx.send(SerialMessage {
                            timestamp: std::time::SystemTime::now(),
                            direction: MessageDirection::Sent,
                            data,
                        }) {
                            error!("Failed to send message to channel: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to write to serial port: {}", e);
                    }
                }
            }
        });
        
        // RX task - handles incoming messages
        let rx_handle = tokio::spawn(async move {
            let mut buffer = vec![0u8; 1024];
            
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                let mut port = port_rx.lock().await;
                match port.read(&mut buffer) {
                    Ok(0) => {
                        // No data available, continue
                        continue;
                    }
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        debug!("Received {} bytes over serial", n);
                        
                        if let Err(e) = message_sender_rx.send(SerialMessage {
                            timestamp: std::time::SystemTime::now(),
                            direction: MessageDirection::Received,
                            data,
                        }) {
                            error!("Failed to send received message to channel: {}", e);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        // Timeout is expected, continue
                        continue;
                    }
                    Err(e) => {
                        error!("Failed to read from serial port: {}", e);
                        break;
                    }
                }
            }
        });
        
        Ok(Self {
            port,
            tx_sender,
            message_receiver,
            _tx_handle: tx_handle,
            _rx_handle: rx_handle,
        })
    }
    
    pub async fn send(&self, data: Vec<u8>) -> TermComResult<()> {
        self.tx_sender.send(data).map_err(|e| TermComError::Communication {
            message: format!("Failed to send data to serial tx channel: {}", e),
        })?;
        
        Ok(())
    }
    
    pub async fn receive(&mut self) -> Option<SerialMessage> {
        self.message_receiver.recv().await
    }
    
    pub async fn send_command(&self, command: &str) -> TermComResult<()> {
        let data = command.as_bytes().to_vec();
        self.send(data).await
    }
    
    pub async fn is_connected(&self) -> bool {
        // Try to access the port to check if it's still valid
        let _port = self.port.lock().await;
        // If we can lock it, assume it's still connected
        // More sophisticated connection checking could be added here
        true
    }
    
    pub async fn close(self) -> TermComResult<()> {
        // Drop the sender to signal background tasks to exit
        drop(self.tx_sender);
        
        // Wait for background tasks to complete
        if let Err(e) = self._tx_handle.await {
            warn!("TX task completed with error: {}", e);
        }
        
        self._rx_handle.abort();
        
        info!("Serial client closed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ParityConfig, FlowControlConfig};
    
    fn create_test_config() -> ConnectionConfig {
        ConnectionConfig::Serial {
            port: "/dev/null".to_string(), // Use /dev/null for testing
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: ParityConfig::None,
            flow_control: FlowControlConfig::None,
        }
    }
    
    #[tokio::test]
    async fn test_serial_client_creation_fails_gracefully() {
        let config = create_test_config();
        
        // This should fail because /dev/null is not a valid serial port
        let result = SerialClient::new(&config).await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_serial_message_creation() {
        let message = SerialMessage {
            timestamp: std::time::SystemTime::now(),
            direction: MessageDirection::Sent,
            data: vec![0x01, 0x02, 0x03],
        };
        
        assert_eq!(message.data.len(), 3);
        assert!(matches!(message.direction, MessageDirection::Sent));
    }
}