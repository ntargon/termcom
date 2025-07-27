use crate::cli::args::OutputFormat;
use crate::core::communication::TransportType;
use crate::core::session::{SessionState, SessionSummary};
use crate::domain::config::{DeviceConfig, TermComConfig, ConnectionConfig};
use serde_json;
use std::io::{self, Write};
use tabled::{Table, Tabled};

/// Output writer trait for different formats
pub trait OutputWriter {
    fn write_sessions(&self, sessions: &[SessionSummary]) -> Result<(), OutputError>;
    fn write_session_detail(&self, session: &SessionState) -> Result<(), OutputError>;
    fn write_config(&self, config: &TermComConfig) -> Result<(), OutputError>;
    fn write_devices(&self, devices: &[DeviceConfig]) -> Result<(), OutputError>;
    fn write_message(&self, message: &str) -> Result<(), OutputError>;
    fn write_error(&self, error: &str) -> Result<(), OutputError>;
}

/// Output formatting errors
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error("Table formatting error: {0}")]
    TableError(String),
}

impl From<OutputError> for crate::domain::error::TermComError {
    fn from(err: OutputError) -> Self {
        Self::Output(err.to_string())
    }
}

/// Console output writer
pub struct ConsoleWriter {
    format: OutputFormat,
}

impl ConsoleWriter {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }
}

impl OutputWriter for ConsoleWriter {
    fn write_sessions(&self, sessions: &[SessionSummary]) -> Result<(), OutputError> {
        match self.format {
            OutputFormat::Text => {
                for session in sessions {
                    println!("Session: {} ({})", session.session_id, session.session_type);
                    println!("  Device: {}", session.device_name);
                    println!("  Status: {}", session.status);
                    println!("  Started: {:?}", session.created_at);
                    println!("  Messages: {}", session.message_count);
                    println!("  Data: {} bytes sent, {} bytes received", session.bytes_sent, session.bytes_received);
                    println!();
                }
            }
            OutputFormat::Json => {
                let output = serde_json::to_string_pretty(sessions)?;
                println!("{}", output);
            }
            OutputFormat::Table => {
                if !sessions.is_empty() {
                    let table_data: Vec<SessionTableRow> = sessions.iter().map(SessionTableRow::from).collect();
                    let table = Table::new(table_data);
                    println!("{}", table);
                }
            }
            OutputFormat::Csv => {
                println!("id,device_name,session_type,status,message_count,bytes_sent,bytes_received");
                for session in sessions {
                    println!("{},{},{},{},{},{},{}", 
                        session.session_id, 
                        session.device_name, 
                        session.session_type,
                        session.status,
                        session.message_count,
                        session.bytes_sent,
                        session.bytes_received
                    );
                }
            }
        }
        Ok(())
    }

    fn write_session_detail(&self, session: &SessionState) -> Result<(), OutputError> {
        match self.format {
            OutputFormat::Text => {
                println!("Session Details:");
                println!("  ID: {}", session.session_id);
                println!("  Device: {}", session.device_name);
                println!("  Type: {}", session.metadata.transport_type);
                println!("  Status: {}", session.status);
                println!("  Transport: {}", session.metadata.transport_type);
                
                println!("  Started: {:?}", session.created_at);
                
                println!("  Statistics:");
                println!("    Messages sent: {}", session.statistics.messages_sent);
                println!("    Messages received: {}", session.statistics.messages_received);
                println!("    Bytes sent: {}", session.statistics.bytes_sent);
                println!("    Bytes received: {}", session.statistics.bytes_received);
                println!("    Errors: {}", session.statistics.error_count);
                
                if session.statistics.avg_response_time_ms > 0.0 {
                    println!("    Average response time: {:.2}ms", session.statistics.avg_response_time_ms);
                }
                
                println!("  Configuration:");
                for (key, value) in &session.metadata.connection_params {
                    println!("    {}: {}", key, value);
                }
            }
            OutputFormat::Json => {
                let output = serde_json::to_string_pretty(session)?;
                println!("{}", output);
            }
            OutputFormat::Table => {
                let table_data = vec![SessionDetailRow::from(session)];
                let table = Table::new(table_data);
                println!("{}", table);
            }
            OutputFormat::Csv => {
                println!("id,device_name,session_type,status,transport_type,messages_sent,messages_received,bytes_sent,bytes_received,error_count");
                println!("{},{},{},{},{},{},{},{},{},{}", 
                    session.session_id, 
                    session.device_name, 
                    session.metadata.transport_type,
                    session.status,
                    session.metadata.transport_type,
                    session.statistics.messages_sent,
                    session.statistics.messages_received,
                    session.statistics.bytes_sent,
                    session.statistics.bytes_received,
                    session.statistics.error_count
                );
            }
        }
        Ok(())
    }

    fn write_config(&self, config: &TermComConfig) -> Result<(), OutputError> {
        match self.format {
            OutputFormat::Text => {
                println!("TermCom Configuration:");
                println!("  Log level: {}", config.global.log_level);
                println!("  Max sessions: {}", config.global.max_sessions);
                println!("  Timeout: {}ms", config.global.timeout_ms);
                println!("  Auto save: {}", config.global.auto_save);
                println!("  History limit: {}", config.global.history_limit);
                
                if !config.devices.is_empty() {
                    println!("  Devices:");
                    for device in &config.devices {
                        let desc = if device.description.is_empty() { "No description" } else { &device.description };
                        println!("    {}: {}", device.name, desc);
                    }
                }
            }
            OutputFormat::Json => {
                let output = serde_json::to_string_pretty(config)?;
                println!("{}", output);
            }
            OutputFormat::Table => {
                if !config.devices.is_empty() {
                    let table_data: Vec<DeviceTableRow> = config.devices.iter().map(DeviceTableRow::from).collect();
                    let table = Table::new(table_data);
                    println!("{}", table);
                }
            }
            OutputFormat::Csv => {
                println!("name,description,transport_type");
                for device in &config.devices {
                    println!("{},{},{}", 
                        device.name, 
                        device.description,
                        get_transport_type(&device.connection)
                    );
                }
            }
        }
        Ok(())
    }

    fn write_devices(&self, devices: &[DeviceConfig]) -> Result<(), OutputError> {
        match self.format {
            OutputFormat::Text => {
                for device in devices {
                    println!("Device: {}", device.name);
                    let desc = if device.description.is_empty() { "No description" } else { &device.description };
                    println!("  Description: {}", desc);
                    println!("  Transport: {}", get_transport_type(&device.connection));
                    println!();
                }
            }
            OutputFormat::Json => {
                let output = serde_json::to_string_pretty(devices)?;
                println!("{}", output);
            }
            OutputFormat::Table => {
                if !devices.is_empty() {
                    let table_data: Vec<DeviceTableRow> = devices.iter().map(DeviceTableRow::from).collect();
                    let table = Table::new(table_data);
                    println!("{}", table);
                }
            }
            OutputFormat::Csv => {
                println!("name,description,transport_type");
                for device in devices {
                    println!("{},{},{}", 
                        device.name, 
                        device.description,
                        get_transport_type(&device.connection)
                    );
                }
            }
        }
        Ok(())
    }

    fn write_message(&self, message: &str) -> Result<(), OutputError> {
        match self.format {
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "message": message,
                    "level": "info"
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            _ => {
                println!("{}", message);
            }
        }
        Ok(())
    }

    fn write_error(&self, error: &str) -> Result<(), OutputError> {
        match self.format {
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "error": error,
                    "level": "error"
                });
                eprintln!("{}", serde_json::to_string_pretty(&output)?);
            }
            _ => {
                eprintln!("Error: {}", error);
            }
        }
        Ok(())
    }
}

/// Table row for session summary
#[derive(Tabled)]
struct SessionTableRow {
    id: String,
    device: String,
    r#type: String,
    status: String,
    messages: u64,
    sent: u64,
    received: u64,
}

impl From<&SessionSummary> for SessionTableRow {
    fn from(session: &SessionSummary) -> Self {
        Self {
            id: session.session_id.clone(),
            device: session.device_name.clone(),
            r#type: session.session_type.to_string(),
            status: session.status.to_string(),
            messages: session.message_count as u64,
            sent: session.bytes_sent,
            received: session.bytes_received,
        }
    }
}

/// Table row for session details
#[derive(Tabled)]
struct SessionDetailRow {
    id: String,
    device: String,
    r#type: String,
    status: String,
    transport: String,
    sent: u64,
    received: u64,
    errors: u64,
}

impl From<&SessionState> for SessionDetailRow {
    fn from(session: &SessionState) -> Self {
        Self {
            id: session.session_id.clone(),
            device: session.device_name.clone(),
            r#type: session.metadata.transport_type.clone(),
            status: session.status.to_string(),
            transport: session.metadata.transport_type.clone(),
            sent: session.statistics.messages_sent,
            received: session.statistics.messages_received,
            errors: session.statistics.error_count,
        }
    }
}

/// Table row for device configuration
#[derive(Tabled)]
struct DeviceTableRow {
    name: String,
    description: String,
    transport: String,
}

impl From<&DeviceConfig> for DeviceTableRow {
    fn from(device: &DeviceConfig) -> Self {
        Self {
            name: device.name.clone(),
            description: device.description.clone(),
            transport: get_transport_type(&device.connection).to_string(),
        }
    }
}

/// File output writer
pub struct FileWriter {
    path: String,
    format: OutputFormat,
}

impl FileWriter {
    pub fn new(path: String, format: OutputFormat) -> Self {
        Self { path, format }
    }

    fn write_to_file(&self, content: &str) -> Result<(), OutputError> {
        let mut file = std::fs::File::create(&self.path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

/// Helper function to get transport type from connection config
fn get_transport_type(connection: &ConnectionConfig) -> TransportType {
    match connection {
        ConnectionConfig::Serial { .. } => TransportType::Serial,
        ConnectionConfig::Tcp { .. } => TransportType::Tcp,
    }
}

impl OutputWriter for FileWriter {
    fn write_sessions(&self, sessions: &[SessionSummary]) -> Result<(), OutputError> {
        let content = match self.format {
            OutputFormat::Json => serde_json::to_string_pretty(sessions)?,
            OutputFormat::Csv => {
                let mut csv = "id,device_name,session_type,status,message_count,bytes_sent,bytes_received\n".to_string();
                for session in sessions {
                    csv.push_str(&format!("{},{},{},{},{},{},{}\n", 
                        session.session_id, 
                        session.device_name, 
                        session.session_type,
                        session.status,
                        session.message_count,
                        session.bytes_sent,
                        session.bytes_received
                    ));
                }
                csv
            }
            _ => {
                return Err(OutputError::TableError("File output only supports JSON and CSV formats".to_string()));
            }
        };
        self.write_to_file(&content)
    }

    fn write_session_detail(&self, session: &SessionState) -> Result<(), OutputError> {
        let content = match self.format {
            OutputFormat::Json => serde_json::to_string_pretty(session)?,
            _ => {
                return Err(OutputError::TableError("File output only supports JSON format for session details".to_string()));
            }
        };
        self.write_to_file(&content)
    }

    fn write_config(&self, config: &TermComConfig) -> Result<(), OutputError> {
        let content = match self.format {
            OutputFormat::Json => serde_json::to_string_pretty(config)?,
            _ => {
                return Err(OutputError::TableError("File output only supports JSON format for configuration".to_string()));
            }
        };
        self.write_to_file(&content)
    }

    fn write_devices(&self, devices: &[DeviceConfig]) -> Result<(), OutputError> {
        let content = match self.format {
            OutputFormat::Json => serde_json::to_string_pretty(devices)?,
            OutputFormat::Csv => {
                let mut csv = "name,description,transport_type\n".to_string();
                for device in devices {
                    csv.push_str(&format!("{},{},{}\n", 
                        device.name, 
                        device.description,
                        get_transport_type(&device.connection)
                    ));
                }
                csv
            }
            _ => {
                return Err(OutputError::TableError("File output only supports JSON and CSV formats".to_string()));
            }
        };
        self.write_to_file(&content)
    }

    fn write_message(&self, message: &str) -> Result<(), OutputError> {
        self.write_to_file(message)
    }

    fn write_error(&self, error: &str) -> Result<(), OutputError> {
        self.write_to_file(&format!("Error: {}", error))
    }
}