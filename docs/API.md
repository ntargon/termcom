# TermCom API Documentation

This document provides comprehensive API documentation for TermCom's library interface.

## Table of Contents

- [Core Modules](#core-modules)
- [Communication API](#communication-api)
- [Session Management](#session-management)
- [Configuration API](#configuration-api)
- [Error Handling](#error-handling)
- [Examples](#examples)

## Core Modules

### Overview

TermCom is structured around several core modules that provide clean separation of concerns:

```rust
use termcom::{
    CommunicationEngine,
    SessionManager,
    SessionType,
    TermComConfig,
    TermComError,
    TermComResult,
};
```

## Communication API

### CommunicationEngine

The `CommunicationEngine` is the central component for managing all communication protocols.

#### Constructor

```rust
impl CommunicationEngine {
    pub fn new(message_buffer_size: usize, max_connections: usize) -> Self
}
```

**Parameters:**
- `message_buffer_size`: Size of the internal message buffer
- `max_connections`: Maximum number of concurrent connections

#### Core Methods

```rust
// Start the communication engine
pub async fn start(&self) -> TermComResult<()>

// Stop the communication engine
pub async fn stop(&self) -> TermComResult<()>

// Create a new session
pub async fn create_session(&self, config: &DeviceConfig) -> TermComResult<String>

// Close an existing session
pub async fn close_session(&self, session_id: &str) -> TermComResult<()>

// Send data to a session
pub async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()>

// Send a command to a session
pub async fn send_command(&self, session_id: &str, command: &str) -> TermComResult<()>
```

#### Information Methods

```rust
// Get session information
pub async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo>

// List all active sessions
pub async fn list_sessions(&self) -> Vec<SessionInfo>

// Get available transport types
pub async fn available_transports(&self) -> Vec<TransportType>

// Check if engine is running
pub async fn is_running(&self) -> bool

// Get communication statistics
pub async fn get_statistics(&self) -> CommunicationStats
```

### Message System

#### Message Structure

```rust
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub timestamp: SystemTime,
    pub message_type: MessageType,
    pub data: Vec<u8>,
    pub metadata: HashMap<String, String>,
}
```

#### Message Types

```rust
pub enum MessageType {
    Sent,           // Outgoing message
    Received,       // Incoming message
    Command,        // Command execution
    Response,       // Command response
    Error,          // Error message
    System,         // System notification
}
```

#### Message Builder

```rust
impl Message {
    pub fn sent(session_id: String, data: Vec<u8>) -> Self
    pub fn received(session_id: String, data: Vec<u8>) -> Self
    pub fn command(session_id: String, command: String) -> Self
    pub fn response(session_id: String, response: String) -> Self
    pub fn error(session_id: String, error: String) -> Self
    
    // Utility methods
    pub fn data_as_string(&self) -> Option<String>
    pub fn data_as_hex(&self) -> String
    pub fn add_tag(&mut self, tag: String)
    pub fn add_property(&mut self, key: String, value: String)
}
```

## Session Management

### SessionManager

The `SessionManager` handles multiple concurrent communication sessions.

#### Constructor

```rust
impl SessionManager {
    pub fn new(
        communication_engine: Arc<CommunicationEngine>, 
        max_sessions: usize
    ) -> Self
}
```

#### Session Operations

```rust
// Create a new session
pub async fn create_session(&self, config: SessionConfig) -> TermComResult<String>

// Start a session
pub async fn start_session(&self, session_id: &str) -> TermComResult<()>

// Stop a session
pub async fn stop_session(&self, session_id: &str) -> TermComResult<()>

// Remove a session
pub async fn remove_session(&self, session_id: &str) -> TermComResult<()>

// Check if session exists
pub async fn has_session(&self, session_id: &str) -> bool

// Get session state
pub async fn get_session_state(&self, session_id: &str) -> Option<SessionState>
```

#### Session Filtering and Queries

```rust
// List sessions with optional filtering
pub async fn list_sessions_filtered(&self, filter: &SessionFilter) -> Vec<SessionSummary>

// Get sessions summary with filtering
pub async fn get_sessions_summary_filtered(&self, filter: &SessionFilter) -> Vec<SessionSummary>

// Find sessions by device name
pub async fn find_sessions_by_device(&self, device_name: &str) -> Vec<String>

// Find sessions by name pattern
pub async fn find_sessions_by_name(&self, pattern: &str) -> Vec<String>
```

#### Statistics and Management

```rust
// Get session count
pub async fn get_session_count(&self) -> usize

// Get active session count
pub async fn get_active_session_count(&self) -> usize

// Get global statistics
pub async fn get_global_statistics(&self) -> GlobalStatistics

// Get maximum sessions allowed
pub fn get_max_sessions(&self) -> usize
```

### Session Configuration

```rust
pub struct SessionConfig {
    pub name: String,
    pub session_type: SessionType,
    pub device_config: DeviceConfig,
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub timeout_ms: u64,
    pub max_history_size: usize,
    pub log_activities: bool,
    pub tags: Vec<String>,
    pub properties: HashMap<String, String>,
}
```

### Session Types

```rust
pub enum SessionType {
    Interactive,    // Manual interaction
    Automated,      // Script-driven
    Monitoring,     // Passive monitoring
    Testing,        // Test execution
}
```

### Session Filter

```rust
pub struct SessionFilter {
    pub session_type: Option<SessionType>,
    pub status: Option<SessionStatus>,
    pub device_name: Option<String>,
    pub name_pattern: Option<String>,
    pub tags: Vec<String>,
}

impl SessionFilter {
    pub fn new() -> Self
    pub fn with_session_type(mut self, session_type: SessionType) -> Self
    pub fn with_status(mut self, status: SessionStatus) -> Self
    pub fn with_device_name(mut self, device_name: &str) -> Self
    pub fn with_name_pattern(mut self, pattern: &str) -> Self
    pub fn with_tag(mut self, tag: &str) -> Self
}
```

## Configuration API

### TermComConfig

Main configuration structure for the application.

```rust
pub struct TermComConfig {
    pub global: GlobalConfig,
    pub devices: Vec<DeviceConfig>,
}

impl Default for TermComConfig {
    fn default() -> Self
}
```

### GlobalConfig

Global application settings.

```rust
pub struct GlobalConfig {
    pub log_level: String,           // "error", "warn", "info", "debug", "trace"
    pub max_sessions: usize,         // Maximum concurrent sessions
    pub timeout_ms: u64,             // Default timeout in milliseconds
    pub auto_save: bool,             // Auto-save session data
    pub history_limit: usize,        // Message history limit
}
```

### DeviceConfig

Device-specific configuration.

```rust
pub struct DeviceConfig {
    pub name: String,
    pub description: String,
    pub connection: ConnectionConfig,
    pub commands: Vec<CustomCommand>,
}
```

### ConnectionConfig

Connection protocol configuration.

```rust
pub enum ConnectionConfig {
    Serial {
        port: String,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: ParityConfig,
        flow_control: FlowControlConfig,
    },
    Tcp {
        host: String,
        port: u16,
        timeout_ms: u64,
        keep_alive: bool,
    },
}
```

### ParityConfig and FlowControlConfig

```rust
pub enum ParityConfig {
    None,
    Odd,
    Even,
}

pub enum FlowControlConfig {
    None,
    Hardware,
    Software,
}
```

### CustomCommand

Pre-defined command configuration.

```rust
pub struct CustomCommand {
    pub name: String,
    pub description: String,
    pub template: String,
    pub response_pattern: Option<String>,
    pub timeout_ms: u64,
}
```

### ConfigManager

Configuration file management.

```rust
impl ConfigManager {
    pub fn new() -> TermComResult<Self>
    
    // Load configuration from files
    pub fn load_config(&self) -> TermComResult<TermComConfig>
    
    // Load from specific path
    pub fn load_config_from_path(&self, path: &Path) -> TermComResult<TermComConfig>
    
    // Save configuration to files
    pub fn save_config(&self, config: &TermComConfig) -> TermComResult<()>
    
    // Save to specific path
    pub fn save_config_to_path(&self, path: &Path, config: &TermComConfig) -> TermComResult<()>
    
    // Initialize project configuration
    pub fn init_project_config(&self, path: &Path) -> TermComResult<()>
    
    // Get configuration paths
    pub fn get_global_config_path_ref(&self) -> &PathBuf
    pub fn get_project_config_path(&self) -> Option<&PathBuf>
}
```

## Error Handling

### TermComError

Comprehensive error enumeration for all failure modes.

```rust
pub enum TermComError {
    Communication { message: String },
    Session { message: String },
    Config { message: String },
    Io { source: std::io::Error },
    Serialization { source: toml::de::Error },
    InvalidInput(String),
    Timeout,
    DeviceNotConnected,
    Protocol(String),
}

pub type TermComResult<T> = Result<T, TermComError>;
```

### Error Conversion

TermComError implements `From` for common error types:

```rust
impl From<std::io::Error> for TermComError
impl From<toml::de::Error> for TermComError
impl From<serialport::Error> for TermComError
```

### Error Display

```rust
impl std::fmt::Display for TermComError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
}

impl std::error::Error for TermComError
```

## Examples

### Basic Communication Setup

```rust
use termcom::{CommunicationEngine, TermComConfig, DeviceConfig, ConnectionConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create communication engine
    let engine = Arc::new(CommunicationEngine::new(1000, 10));
    
    // Start the engine
    engine.start().await?;
    
    // Create device configuration
    let device_config = DeviceConfig {
        name: "Arduino".to_string(),
        description: "Arduino Uno".to_string(),
        connection: ConnectionConfig::Serial {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: ParityConfig::None,
            flow_control: FlowControlConfig::None,
        },
        commands: Vec::new(),
    };
    
    // Create session
    let session_id = engine.create_session(&device_config).await?;
    
    // Send data
    engine.send_command(&session_id, "AT\r\n").await?;
    
    // Cleanup
    engine.close_session(&session_id).await?;
    engine.stop().await?;
    
    Ok(())
}
```

### Session Management

```rust
use termcom::{SessionManager, SessionConfig, SessionType, DeviceConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Arc::new(CommunicationEngine::new(1000, 10));
    let session_manager = Arc::new(RwLock::new(SessionManager::new(engine.clone(), 10)));
    
    // Create session configuration
    let session_config = SessionConfig {
        name: "debug_session".to_string(),
        session_type: SessionType::Interactive,
        device_config: device_config, // From previous example
        auto_reconnect: true,
        max_reconnect_attempts: 3,
        reconnect_delay_ms: 1000,
        timeout_ms: 5000,
        max_history_size: 1000,
        log_activities: true,
        tags: vec!["debug".to_string()],
        properties: std::collections::HashMap::new(),
    };
    
    // Create and start session
    let session_id = {
        let mut manager = session_manager.write().await;
        manager.create_session(session_config).await?
    };
    
    {
        let manager = session_manager.read().await;
        manager.start_session(&session_id).await?;
    }
    
    // List active sessions
    let sessions = {
        let manager = session_manager.read().await;
        manager.list_sessions().await
    };
    
    println!("Active sessions: {}", sessions.len());
    
    Ok(())
}
```

### Configuration Management

```rust
use termcom::{ConfigManager, TermComConfig, GlobalConfig, DeviceConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration manager
    let config_manager = ConfigManager::new()?;
    
    // Load existing configuration
    let mut config = config_manager.load_config()?;
    
    // Modify configuration
    config.global.max_sessions = 20;
    config.global.log_level = "debug".to_string();
    
    // Add new device
    let new_device = DeviceConfig {
        name: "ESP32".to_string(),
        description: "ESP32 Development Board".to_string(),
        connection: ConnectionConfig::Tcp {
            host: "192.168.1.100".to_string(),
            port: 80,
            timeout_ms: 3000,
            keep_alive: true,
        },
        commands: Vec::new(),
    };
    
    config.devices.push(new_device);
    
    // Save configuration
    config_manager.save_config(&config)?;
    
    println!("Configuration updated successfully");
    Ok(())
}
```

### Error Handling

```rust
use termcom::{TermComError, TermComResult};

async fn handle_communication() -> TermComResult<()> {
    match engine.send_command(&session_id, "INVALID").await {
        Ok(_) => println!("Command sent successfully"),
        Err(TermComError::Communication { message }) => {
            eprintln!("Communication error: {}", message);
        },
        Err(TermComError::Timeout) => {
            eprintln!("Operation timed out");
        },
        Err(TermComError::DeviceNotConnected) => {
            eprintln!("Device is not connected");
        },
        Err(e) => {
            eprintln!("Other error: {}", e);
        }
    }
    
    Ok(())
}
```

### Message Filtering

```rust
use termcom::{SessionFilter, SessionType, SessionStatus};

async fn filter_sessions() -> TermComResult<()> {
    let filter = SessionFilter::new()
        .with_session_type(SessionType::Interactive)
        .with_status(SessionStatus::Active)
        .with_device_name("Arduino")
        .with_tag("debug");
    
    let sessions = {
        let manager = session_manager.read().await;
        manager.list_sessions_filtered(&filter).await
    };
    
    for session in sessions {
        println!("Session: {} - {}", session.id, session.name);
    }
    
    Ok(())
}
```

This API documentation provides a comprehensive guide to using TermCom as a library. For CLI usage, refer to the main README.md file.