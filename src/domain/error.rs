use thiserror::Error;

/// TermCom unified error type
#[derive(Error, Debug)]
pub enum TermComError {
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("Session error: {message}")]
    Session { message: String },
    
    #[error("Communication timeout")]
    Timeout,
    
    #[error("Invalid data format: {0}")]
    InvalidData(String),
    
    #[error("Device not connected")]
    DeviceNotConnected,
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Communication error: {message}")]
    Communication { message: String },
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Output error: {0}")]
    Output(String),
}

pub type TermComResult<T> = Result<T, TermComError>;
