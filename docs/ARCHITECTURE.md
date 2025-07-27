# TermCom Architecture

This document provides a detailed overview of TermCom's software architecture, design patterns, and component relationships.

## Table of Contents

- [Overview](#overview)
- [Architectural Patterns](#architectural-patterns)
- [Layer Architecture](#layer-architecture)
- [Core Components](#core-components)
- [Data Flow](#data-flow)
- [Concurrency Model](#concurrency-model)
- [Error Handling Strategy](#error-handling-strategy)
- [Testing Strategy](#testing-strategy)
- [Performance Considerations](#performance-considerations)

## Overview

TermCom follows a clean architecture approach with clear separation of concerns, dependency inversion, and testability. The architecture is designed to be:

- **Modular**: Clear separation between different functional areas
- **Testable**: Easy to unit test and integration test
- **Extensible**: Easy to add new communication protocols
- **Maintainable**: Clear code organization and documentation
- **Performant**: Efficient async/await patterns and resource management

## Architectural Patterns

### 1. Clean Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Presentation Layer                        │
│  ┌─────────────────┐              ┌─────────────────────┐   │
│  │   CLI Module    │              │    TUI Module       │   │
│  │                 │              │                     │   │
│  │ • Args parsing  │              │ • Event handling    │   │
│  │ • Command exec  │              │ • UI rendering      │   │
│  │ • Output format │              │ • State management  │   │
│  └─────────────────┘              └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                     Application Layer                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                   Core Module                           │ │
│  │                                                         │ │
│  │ • Communication Engine    • Session Manager            │ │
│  │ • Message Processing      • Configuration Handler      │ │
│  │ • Business Logic          • Validation Rules           │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                     Domain Layer                            │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                  Domain Models                          │ │
│  │                                                         │ │
│  │ • Configuration Types     • Error Types                │ │
│  │ • Message Types          • Session Types               │ │
│  │ • Business Rules         • Domain Logic                │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                  Infrastructure Layer                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Serial    │  │     TCP     │  │      Config         │ │
│  │             │  │             │  │                     │ │
│  │ • SerialMgr │  │ • TcpMgr    │  │ • File I/O          │ │
│  │ • SerialCli │  │ • TcpCli    │  │ • TOML parsing      │ │
│  │ • Hardware  │  │ • Network   │  │ • Path management   │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 2. Repository Pattern

Configuration and session data access is abstracted through repository-like patterns:

```rust
// Configuration Repository
impl ConfigManager {
    fn load_config(&self) -> TermComResult<TermComConfig>
    fn save_config(&self, config: &TermComConfig) -> TermComResult<()>
}

// Session Repository (via SessionManager)
impl SessionManager {
    async fn create_session(&self, config: SessionConfig) -> TermComResult<String>
    async fn get_session(&self, id: &str) -> Option<Session>
    async fn list_sessions(&self) -> Vec<SessionSummary>
}
```

### 3. Strategy Pattern

Different communication protocols are implemented using the strategy pattern:

```rust
pub trait Transport: Send + Sync {
    async fn create_session(&self, config: &DeviceConfig) -> TermComResult<String>;
    async fn close_session(&self, session_id: &str) -> TermComResult<()>;
    async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()>;
    // ... other methods
}

// Concrete implementations
impl Transport for SerialManager { ... }
impl Transport for TcpManager { ... }
```

### 4. Observer Pattern

Event-driven communication for UI updates and logging:

```rust
// Message broadcasting
impl CommunicationEngine {
    async fn broadcast_message(&self, message: Message) {
        // Notify all subscribers (UI, loggers, etc.)
    }
}
```

## Layer Architecture

### Presentation Layer

**Responsibilities:**
- User interface (CLI and TUI)
- Input validation and parsing
- Output formatting and display
- User interaction handling

**Components:**
- `cli/`: Command-line interface implementation
- `tui/`: Terminal user interface implementation

### Application Layer

**Responsibilities:**
- Business logic orchestration
- Use case implementation
- Cross-cutting concerns (logging, validation)

**Components:**
- `core/communication/`: Communication engine and message handling
- `core/session/`: Session lifecycle management
- `core/config/`: Configuration validation and processing

### Domain Layer

**Responsibilities:**
- Core business entities
- Domain rules and constraints
- Business logic without external dependencies

**Components:**
- `domain/config.rs`: Configuration types and validation
- `domain/error.rs`: Error types and handling

### Infrastructure Layer

**Responsibilities:**
- External service integration
- I/O operations
- Framework-specific implementations

**Components:**
- `infrastructure/serial/`: Serial port communication
- `infrastructure/tcp/`: TCP network communication
- `infrastructure/config/`: File system configuration management
- `infrastructure/logging/`: Logging framework integration

## Core Components

### 1. CommunicationEngine

**Purpose**: Central coordinator for all communication activities.

```rust
pub struct CommunicationEngine {
    running: Arc<AtomicBool>,
    transport_registry: Arc<RwLock<TransportRegistry>>,
    message_history: Arc<RwLock<VecDeque<Message>>>,
    sequence_counter: Arc<RwLock<u64>>,
}
```

**Key Responsibilities:**
- Protocol abstraction and management
- Message routing and history
- Session lifecycle coordination
- Statistics collection

**Design Patterns:**
- Facade pattern (simplifies complex subsystem)
- Registry pattern (manages transport implementations)

### 2. SessionManager

**Purpose**: Manages multiple concurrent communication sessions.

```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    communication_engine: Arc<CommunicationEngine>,
    max_sessions: usize,
}
```

**Key Responsibilities:**
- Session creation and destruction
- Session state management
- Resource allocation and limits
- Session filtering and queries

**Design Patterns:**
- Manager pattern (coordinates session lifecycle)
- Factory pattern (creates configured sessions)

### 3. Transport Layer

**Purpose**: Protocol-specific communication implementation.

```rust
pub struct TransportRegistry {
    transports: HashMap<TransportType, Box<dyn Transport>>,
}

pub enum TransportType {
    Serial,
    Tcp,
}
```

**Key Responsibilities:**
- Protocol-specific implementation
- Connection management
- Data transmission and reception
- Error handling and recovery

**Design Patterns:**
- Strategy pattern (interchangeable protocols)
- Registry pattern (transport discovery)

### 4. Configuration System

**Purpose**: Hierarchical configuration management.

```rust
pub struct ConfigManager {
    global_config_path: PathBuf,
    project_config_path: Option<PathBuf>,
}
```

**Key Responsibilities:**
- Configuration file location and loading
- Hierarchical configuration merging
- Validation and error reporting
- Default value management

**Design Patterns:**
- Template method pattern (configuration loading steps)
- Builder pattern (configuration construction)

## Data Flow

### 1. Message Flow

```
┌─────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   User      │───▶│ Presentation    │───▶│ Application     │
│ Input/Cmd   │    │ Layer (CLI/TUI) │    │ Layer (Core)    │
└─────────────┘    └─────────────────┘    └─────────────────┘
                                                    │
                                                    ▼
┌─────────────────────────────────────────────────────────────┐
│                Communication Engine                          │
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐ │
│  │   Message   │───▶│  Transport  │───▶│   Hardware/     │ │
│  │  Processing │    │   Layer     │    │   Network       │ │
│  └─────────────┘    └─────────────┘    └─────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                                                    │
                                                    ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────┐
│    Response     │◀───│   Infrastructure│◀───│   External  │
│   Processing    │    │     Layer       │    │   Device    │
└─────────────────┘    └─────────────────┘    └─────────────┘
```

### 2. Session Lifecycle

```
┌─────────────┐
│   Create    │
│  Session    │
└──────┬──────┘
       │
       ▼
┌─────────────┐    ┌─────────────┐
│ Configure   │───▶│ Initialize  │
│ Transport   │    │ Connection  │
└─────────────┘    └──────┬──────┘
                          │
                          ▼
                   ┌─────────────┐
                   │   Active    │◀──┐
                   │  Session    │   │
                   └──────┬──────┘   │
                          │          │
                    ┌─────▼─────┐    │
                    │   Send/   │────┘
                    │  Receive  │
                    └───────────┘
                          │
                          ▼
                   ┌─────────────┐
                   │   Close     │
                   │  Session    │
                   └─────────────┘
```

### 3. Configuration Loading

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Application    │───▶│ ConfigManager   │───▶│  File System    │
│    Startup      │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Configuration Merge                            │
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │   Default   │───▶│   Global    │───▶│      Project        │ │
│  │   Config    │    │   Config    │    │      Config         │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
                    ┌─────────────────┐
                    │  Final Config   │
                    │   (In Memory)   │
                    └─────────────────┘
```

## Concurrency Model

### 1. Async/Await Pattern

TermCom uses Tokio's async runtime for efficient concurrency:

```rust
// All I/O operations are async
pub async fn send_data(&self, session_id: &str, data: Vec<u8>) -> TermComResult<()>
pub async fn receive_message(&mut self) -> Option<Message>
pub async fn create_session(&self, config: SessionConfig) -> TermComResult<String>
```

### 2. Shared State Management

Shared state is protected using appropriate synchronization primitives:

```rust
// Read-heavy data structures use RwLock
sessions: Arc<RwLock<HashMap<String, Session>>>
transport_registry: Arc<RwLock<TransportRegistry>>

// Simple atomic operations use AtomicBool/AtomicUsize
running: Arc<AtomicBool>
sequence_counter: Arc<AtomicU64>
```

### 3. Message Passing

Communication between components uses channels for loose coupling:

```rust
// Unbounded channels for message passing
let (sender, receiver) = mpsc::unbounded_channel::<Message>();

// Broadcast channels for event distribution
let (broadcast_tx, _) = broadcast::channel::<SystemEvent>(100);
```

### 4. Task Management

Long-running operations are spawned as separate tasks:

```rust
// Background tasks for continuous operations
tokio::spawn(async move {
    // Message processing loop
    while let Some(message) = receiver.recv().await {
        process_message(message).await;
    }
});
```

## Error Handling Strategy

### 1. Error Type Hierarchy

```rust
pub enum TermComError {
    // Infrastructure errors
    Communication { message: String },
    Io { source: std::io::Error },
    
    // Business logic errors
    Session { message: String },
    Config { message: String },
    InvalidInput(String),
    
    // Protocol errors
    Timeout,
    DeviceNotConnected,
    Protocol(String),
}
```

### 2. Error Propagation

Errors are propagated up the call stack using the `?` operator:

```rust
pub async fn send_command(&self, session_id: &str, command: &str) -> TermComResult<()> {
    let session = self.get_session(session_id)
        .ok_or_else(|| TermComError::Session { 
            message: format!("Session {} not found", session_id) 
        })?;
    
    session.send_command(command).await?;
    Ok(())
}
```

### 3. Error Recovery

Each layer implements appropriate error recovery strategies:

```rust
// Automatic retry with exponential backoff
async fn send_with_retry(&self, data: &[u8], max_retries: u32) -> TermComResult<()> {
    let mut retries = 0;
    let mut delay = Duration::from_millis(100);
    
    loop {
        match self.send_raw(data).await {
            Ok(_) => return Ok(()),
            Err(e) if retries < max_retries => {
                tokio::time::sleep(delay).await;
                retries += 1;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 4. Error Logging and Monitoring

Structured logging provides error context:

```rust
use tracing::{error, warn, info, debug};

// Error logging with context
error!(
    session_id = %session_id,
    error = %e,
    "Failed to send command to session"
);
```

## Testing Strategy

### 1. Unit Testing

Each component is tested in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_session_creation() {
        let engine = CommunicationEngine::new(100, 5);
        let manager = SessionManager::new(Arc::new(engine), 10);
        
        let config = SessionConfig::default();
        let session_id = manager.create_session(config).await.unwrap();
        
        assert!(manager.has_session(&session_id).await);
    }
}
```

### 2. Integration Testing

Components are tested together:

```rust
#[tokio::test]
async fn test_end_to_end_communication() {
    let config_manager = ConfigManager::new().unwrap();
    let config = config_manager.load_config().unwrap();
    
    let engine = Arc::new(CommunicationEngine::new(1000, 10));
    engine.start().await.unwrap();
    
    // Test actual communication flow
    let session_id = engine.create_session(&config.devices[0]).await.unwrap();
    engine.send_command(&session_id, "AT").await.unwrap();
}
```

### 3. Property Testing

Property-based testing for configuration validation:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_config_serialization_roundtrip(config in any::<TermComConfig>()) {
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: TermComConfig = toml::from_str(&serialized).unwrap();
        prop_assert_eq!(config, deserialized);
    }
}
```

### 4. Performance Testing

Performance benchmarks for critical paths:

```rust
#[tokio::test]
async fn test_message_throughput() {
    let start = Instant::now();
    
    for _ in 0..1000 {
        engine.send_data(&session_id, vec![0x42; 100]).await.unwrap();
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_secs(1), "Too slow: {:?}", elapsed);
}
```

## Performance Considerations

### 1. Memory Management

- **Message Buffering**: Bounded buffers prevent memory leaks
- **Session Limits**: Configurable maximum concurrent sessions
- **History Management**: Automatic cleanup of old messages

```rust
// Bounded message history
if self.message_history.len() > self.max_history_size {
    self.message_history.pop_front();
}
```

### 2. CPU Optimization

- **Lazy Evaluation**: Expensive operations are deferred
- **Batch Processing**: Multiple operations are batched
- **Efficient Data Structures**: HashMap for O(1) lookups

```rust
// Efficient session lookup
let session = self.sessions.get(&session_id)?;
```

### 3. I/O Optimization

- **Async I/O**: Non-blocking operations
- **Connection Pooling**: Reuse connections when possible
- **Buffer Management**: Efficient buffer allocation

```rust
// Async I/O with proper error handling
let mut buf = vec![0u8; 1024];
let n = stream.read(&mut buf).await?;
buf.truncate(n);
```

### 4. Network Optimization

- **Keep-Alive**: Persistent connections for TCP
- **Timeout Management**: Configurable timeouts
- **Retry Logic**: Exponential backoff for reliability

```rust
// TCP keep-alive configuration
socket.set_keepalive(Some(Duration::from_secs(60)))?;
```

This architecture enables TermCom to be maintainable, testable, and performant while providing a clear separation of concerns and extensibility for future enhancements.