use crate::cli::args::{
    Args, Command, ConfigCommand, DataFormat, SerialCommand, SessionCommand, TcpCommand,
};
use crate::cli::output::{ConsoleWriter, OutputWriter};
use crate::core::communication::{CommunicationEngine, TransportType};
use crate::core::session::{SessionManager, SessionFilter, SessionType};
use crate::domain::config::{
    DeviceConfig, TermComConfig, ConnectionConfig,
};
use crate::domain::error::TermComError;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Execute CLI command
pub async fn execute_command(args: Args) -> Result<(), TermComError> {
    let writer = ConsoleWriter::new(args.output.clone());
    
    // Load configuration
    let config = load_config(args.config.as_deref()).await?;
    
    // Initialize logging
    if !args.quiet {
        setup_logging(&config.global, args.verbose)?;
    }
    
    // Create communication engine and session manager
    let comm_engine = Arc::new(CommunicationEngine::new(1000, 10));
    let session_manager = Arc::new(RwLock::new(SessionManager::new(comm_engine.clone(), config.global.max_sessions)));
    
    match args.command {
        Command::Serial(serial_args) => {
            execute_serial_command(serial_args, &writer, &config, &comm_engine, &session_manager).await
        }
        Command::Tcp(tcp_args) => {
            execute_tcp_command(tcp_args, &writer, &config, &comm_engine, &session_manager).await
        }
        Command::Session(session_args) => {
            execute_session_command(session_args, &writer, &session_manager).await
        }
        Command::Config(config_args) => {
            execute_config_command(config_args, &writer, &config).await
        }
        Command::Tui => {
            writer.write_message("TUI mode not implemented yet")?;
            Ok(())
        }
        Command::Version => {
            writer.write_message(&format!("termcom {}", env!("CARGO_PKG_VERSION")))?;
            Ok(())
        }
    }
}

async fn execute_serial_command(
    args: crate::cli::args::SerialArgs,
    writer: &ConsoleWriter,
    config: &TermComConfig,
    _comm_engine: &Arc<CommunicationEngine>,
    session_manager: &Arc<RwLock<SessionManager>>,
) -> Result<(), TermComError> {
    match args.command {
        SerialCommand::Connect { name, session } => {
            let session_id = session.unwrap_or_else(|| Uuid::new_v4().to_string());
            let device_name = name.unwrap_or_else(|| format!("serial-{}", args.port));
            
            let device_config = DeviceConfig {
                name: device_name.clone(),
                description: format!("Serial device on {}", args.port),
                connection: ConnectionConfig::Serial {
                    port: args.port,
                    baud_rate: args.baud,
                    data_bits: args.data_bits,
                    stop_bits: args.stop_bits,
                    parity: args.parity.into(),
                    flow_control: args.flow_control.into(),
                },
                commands: Vec::new(),
            };
            
            let session_config = crate::core::session::SessionConfig {
                name: session_id.clone(),
                session_type: SessionType::Interactive,
                device_config,
                auto_reconnect: false,
                max_reconnect_attempts: 3,
                reconnect_delay_ms: 1000,
                timeout_ms: 0,
                max_history_size: 1000,
                log_activities: true,
                tags: Vec::new(),
                properties: std::collections::HashMap::new(),
            };
            
            let mut manager = session_manager.write().await;
            manager.create_session(session_config).await?;
            
            writer.write_message(&format!("Serial session '{}' created for device '{}'", session_id, device_name))?;
            Ok(())
        }
        SerialCommand::Send { data, session, format } => {
            let data_bytes = parse_data(&data, format)?;
            
            if let Some(session_id) = session {
                let manager = session_manager.read().await;
                if manager.has_session(&session_id).await {
                    writer.write_message(&format!("Sent {} bytes to session '{}'", data_bytes.len(), session_id))?;
                } else {
                    writer.write_error(&format!("Session '{}' not found", session_id))?;
                }
            } else {
                writer.write_error("Session ID required for send command")?;
            }
            Ok(())
        }
        SerialCommand::List => {
            let ports = serialport::available_ports()
                .map_err(|e| TermComError::Communication { message: format!("Failed to list serial ports: {}", e) })?;
            
            writer.write_message("Available serial ports:")?;
            for port in ports {
                writer.write_message(&format!("  {}", port.port_name))?;
            }
            Ok(())
        }
        SerialCommand::Monitor { session, output: _ } => {
            if let Some(session_id) = session {
                let manager = session_manager.read().await;
                if manager.has_session(&session_id).await {
                    writer.write_message(&format!("Monitoring session '{}' (Press Ctrl+C to stop)", session_id))?;
                    // TODO: Implement actual monitoring
                } else {
                    writer.write_error(&format!("Session '{}' not found", session_id))?;
                }
            } else {
                writer.write_error("Session ID required for monitor command")?;
            }
            Ok(())
        }
    }
}

async fn execute_tcp_command(
    args: crate::cli::args::TcpArgs,
    writer: &ConsoleWriter,
    _config: &TermComConfig,
    _comm_engine: &Arc<CommunicationEngine>,
    session_manager: &Arc<RwLock<SessionManager>>,
) -> Result<(), TermComError> {
    match args.command {
        TcpCommand::Connect { host, port, name, session, timeout } => {
            let session_id = session.unwrap_or_else(|| Uuid::new_v4().to_string());
            let device_name = name.unwrap_or_else(|| format!("tcp-{}:{}", host, port));
            
            let device_config = DeviceConfig {
                name: device_name.clone(),
                description: format!("TCP client connection to {}:{}", host, port),
                connection: ConnectionConfig::Tcp {
                    host: host.clone(),
                    port,
                    timeout_ms: timeout * 1000,
                    keep_alive: true,
                },
                commands: Vec::new(),
            };
            
            let session_config = crate::core::session::SessionConfig {
                name: session_id.clone(),
                session_type: SessionType::Interactive,
                device_config,
                auto_reconnect: false,
                max_reconnect_attempts: 3,
                reconnect_delay_ms: 1000,
                timeout_ms: 0,
                max_history_size: 1000,
                log_activities: true,
                tags: Vec::new(),
                properties: std::collections::HashMap::new(),
            };
            
            let mut manager = session_manager.write().await;
            manager.create_session(session_config).await?;
            
            writer.write_message(&format!("TCP session '{}' created for {}:{}", session_id, host, port))?;
            Ok(())
        }
        TcpCommand::Server { bind, port, name, session } => {
            let session_id = session.unwrap_or_else(|| Uuid::new_v4().to_string());
            let device_name = name.unwrap_or_else(|| format!("tcp-server-{}:{}", bind, port));
            
            let device_config = DeviceConfig {
                name: device_name.clone(),
                description: format!("TCP server listening on {}:{}", bind, port),
                connection: ConnectionConfig::Tcp {
                    host: bind.clone(),
                    port,
                    timeout_ms: 5000,
                    keep_alive: true,
                },
                commands: Vec::new(),
            };
            
            let session_config = crate::core::session::SessionConfig {
                name: session_id.clone(),
                session_type: SessionType::Interactive,
                device_config,
                auto_reconnect: false,
                max_reconnect_attempts: 3,
                reconnect_delay_ms: 1000,
                timeout_ms: 0,
                max_history_size: 1000,
                log_activities: true,
                tags: Vec::new(),
                properties: std::collections::HashMap::new(),
            };
            
            let mut manager = session_manager.write().await;
            manager.create_session(session_config).await?;
            
            writer.write_message(&format!("TCP server session '{}' created on {}:{}", session_id, bind, port))?;
            Ok(())
        }
        TcpCommand::Send { data, session, format } => {
            let data_bytes = parse_data(&data, format)?;
            
            if let Some(session_id) = session {
                let manager = session_manager.read().await;
                if manager.has_session(&session_id).await {
                    writer.write_message(&format!("Sent {} bytes to session '{}'", data_bytes.len(), session_id))?;
                } else {
                    writer.write_error(&format!("Session '{}' not found", session_id))?;
                }
            } else {
                writer.write_error("Session ID required for send command")?;
            }
            Ok(())
        }
        TcpCommand::Monitor { session, output: _ } => {
            if let Some(session_id) = session {
                let manager = session_manager.read().await;
                if manager.has_session(&session_id).await {
                    writer.write_message(&format!("Monitoring session '{}' (Press Ctrl+C to stop)", session_id))?;
                    // TODO: Implement actual monitoring
                } else {
                    writer.write_error(&format!("Session '{}' not found", session_id))?;
                }
            } else {
                writer.write_error("Session ID required for monitor command")?;
            }
            Ok(())
        }
    }
}

async fn execute_session_command(
    args: crate::cli::args::SessionArgs,
    writer: &ConsoleWriter,
    session_manager: &Arc<RwLock<SessionManager>>,
) -> Result<(), TermComError> {
    match args.command {
        SessionCommand::List { r#type, status, device } => {
            let manager = session_manager.read().await;
            
            let mut filter = SessionFilter::new();
            if let Some(session_type) = r#type {
                filter = filter.with_session_type(session_type.into());
            }
            if let Some(session_status) = status {
                filter = filter.with_status(session_status.into());
            }
            if let Some(device_name) = device {
                filter = filter.with_device_name(&device_name);
            }
            
            let sessions = manager.get_sessions_summary_filtered(&filter).await;
            writer.write_sessions(&sessions)?;
            Ok(())
        }
        SessionCommand::Show { id, messages: _, activities: _ } => {
            let manager = session_manager.read().await;
            if let Some(state) = manager.get_session_state(&id).await {
                writer.write_session_detail(&state)?;
            } else {
                writer.write_error(&format!("Session '{}' not found", id))?;
            }
            Ok(())
        }
        SessionCommand::Start { id } => {
            let mut manager = session_manager.write().await;
            manager.start_session(&id).await?;
            writer.write_message(&format!("Session '{}' started", id))?;
            Ok(())
        }
        SessionCommand::Stop { id } => {
            let mut manager = session_manager.write().await;
            manager.stop_session(&id).await?;
            writer.write_message(&format!("Session '{}' stopped", id))?;
            Ok(())
        }
        SessionCommand::Remove { id } => {
            let mut manager = session_manager.write().await;
            manager.remove_session(&id).await?;
            writer.write_message(&format!("Session '{}' removed", id))?;
            Ok(())
        }
        SessionCommand::Create { config: _, name: _, r#type: _ } => {
            writer.write_message("Session creation from config not implemented yet")?;
            Ok(())
        }
        SessionCommand::Export { id: _, output: _, format: _ } => {
            writer.write_message("Session export not implemented yet")?;
            Ok(())
        }
        SessionCommand::Stats => {
            let manager = session_manager.read().await;
            let stats = manager.get_global_statistics().await;
            
            writer.write_message("Session Statistics:")?;
            writer.write_message(&format!("  Total sessions: {}", stats.total_sessions))?;
            writer.write_message(&format!("  Active sessions: {}", stats.active_sessions))?;
            writer.write_message(&format!("  Total messages: {}", stats.total_messages))?;
            writer.write_message(&format!("  Total bytes sent: {}", stats.total_bytes_sent))?;
            writer.write_message(&format!("  Total bytes received: {}", stats.total_bytes_received))?;
            writer.write_message(&format!("  Total errors: {}", stats.total_errors))?;
            
            Ok(())
        }
    }
}

async fn execute_config_command(
    args: crate::cli::args::ConfigArgs,
    writer: &ConsoleWriter,
    config: &TermComConfig,
) -> Result<(), TermComError> {
    match args.command {
        ConfigCommand::Show => {
            writer.write_config(config)?;
            Ok(())
        }
        ConfigCommand::Validate { file } => {
            let config_path = file.unwrap_or_else(|| "termcom.toml".to_string());
            match load_config_from_file(&config_path).await {
                Ok(_) => writer.write_message(&format!("Configuration file '{}' is valid", config_path))?,
                Err(e) => writer.write_error(&format!("Configuration validation failed: {}", e))?,
            }
            Ok(())
        }
        ConfigCommand::Init { output, global } => {
            let config_path = if global {
                get_global_config_path()?
            } else {
                output.unwrap_or_else(|| "termcom.toml".to_string())
            };
            
            let default_config = TermComConfig::default();
            save_config_to_file(&default_config, &config_path).await?;
            writer.write_message(&format!("Default configuration saved to '{}'", config_path))?;
            Ok(())
        }
        ConfigCommand::Devices => {
            writer.write_devices(&config.devices)?;
            Ok(())
        }
        ConfigCommand::AddDevice { name: _, description: _, connection: _ } => {
            writer.write_message("Add device command not implemented yet")?;
            Ok(())
        }
    }
}

fn parse_data(data: &str, format: DataFormat) -> Result<Vec<u8>, TermComError> {
    match format {
        DataFormat::Text => Ok(data.as_bytes().to_vec()),
        DataFormat::Hex => {
            let cleaned = data.replace(' ', "").replace('\n', "");
            hex::decode(&cleaned)
                .map_err(|e| TermComError::InvalidInput(format!("Invalid hex data: {}", e)))
        }
        DataFormat::Base64 => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode(data)
                .map_err(|e| TermComError::InvalidInput(format!("Invalid base64 data: {}", e)))
        }
    }
}

async fn load_config(config_path: Option<&str>) -> Result<TermComConfig, TermComError> {
    if let Some(path) = config_path {
        load_config_from_file(path).await
    } else {
        // Try to load from standard locations
        let global_config = get_global_config_path().ok()
            .and_then(|path| std::fs::metadata(&path).ok())
            .map(|_| get_global_config_path().unwrap());
            
        let local_config = std::fs::metadata("termcom.toml").ok()
            .map(|_| "termcom.toml".to_string());
            
        if let Some(path) = local_config {
            load_config_from_file(&path).await
        } else if let Some(path) = global_config {
            load_config_from_file(&path).await
        } else {
            Ok(TermComConfig::default())
        }
    }
}

async fn load_config_from_file(path: &str) -> Result<TermComConfig, TermComError> {
    let content = tokio::fs::read_to_string(path).await
        .map_err(|e| TermComError::Configuration(format!("Failed to read config file '{}': {}", path, e)))?;
    
    toml::from_str(&content)
        .map_err(|e| TermComError::Configuration(format!("Failed to parse config file '{}': {}", path, e)))
}

async fn save_config_to_file(config: &TermComConfig, path: &str) -> Result<(), TermComError> {
    let content = toml::to_string_pretty(config)
        .map_err(|e| TermComError::Configuration(format!("Failed to serialize config: {}", e)))?;
    
    tokio::fs::write(path, content).await
        .map_err(|e| TermComError::Configuration(format!("Failed to write config file '{}': {}", path, e)))
}

fn get_global_config_path() -> Result<String, TermComError> {
    let home = std::env::var("HOME")
        .map_err(|_| TermComError::Configuration("HOME environment variable not set".to_string()))?;
    let config_dir = PathBuf::from(home).join(".config").join("termcom");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| TermComError::Configuration(format!("Failed to create config directory: {}", e)))?;
    Ok(config_dir.join("config.toml").to_string_lossy().to_string())
}

fn setup_logging(config: &crate::domain::config::GlobalConfig, verbose: bool) -> Result<(), TermComError> {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        match config.log_level.as_str() {
            "error" => tracing::Level::ERROR,
            "warn" => tracing::Level::WARN,
            "info" => tracing::Level::INFO,
            "debug" => tracing::Level::DEBUG,
            "trace" => tracing::Level::TRACE,
            _ => tracing::Level::INFO,
        }
    };
    
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();
        
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| TermComError::Configuration(format!("Failed to initialize logging: {}", e)))?;
        
    Ok(())
}