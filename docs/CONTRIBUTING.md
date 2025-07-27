# Contributing to TermCom

Thank you for your interest in contributing to TermCom! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Standards](#code-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)
- [Performance Guidelines](#performance-guidelines)
- [Security Considerations](#security-considerations)

## Code of Conduct

TermCom follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please read and follow these guidelines to ensure a welcoming environment for all contributors.

## Getting Started

### Prerequisites

- **Rust**: Version 1.70 or later
- **Git**: For version control
- **Operating System**: Linux, macOS, or Windows with appropriate development tools

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/termcom.git
   cd termcom
   ```

2. **Install Dependencies**
   ```bash
   # Rust toolchain (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Development tools
   rustup component add clippy rustfmt
   cargo install cargo-audit cargo-outdated
   ```

3. **Build the Project**
   ```bash
   cargo build
   ```

4. **Run Tests**
   ```bash
   cargo test
   ```

5. **Verify Setup**
   ```bash
   cargo run -- --help
   ```

### Project Structure

Understanding the codebase structure helps with navigation and contribution:

```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library root
├── cli/                    # Command-line interface
│   ├── args.rs            # Argument parsing
│   ├── commands.rs        # Command execution
│   ├── output.rs          # Output formatting
│   └── mod.rs
├── tui/                    # Terminal user interface
│   ├── app.rs             # TUI application
│   ├── event.rs           # Event handling
│   ├── state.rs           # UI state management
│   ├── ui.rs              # UI rendering
│   ├── widgets/           # UI components
│   └── mod.rs
├── core/                   # Business logic
│   ├── communication/     # Communication engine
│   ├── session/           # Session management
│   └── mod.rs
├── domain/                # Domain models
│   ├── config.rs          # Configuration types
│   ├── error.rs           # Error types
│   └── mod.rs
└── infrastructure/        # External integrations
    ├── serial/            # Serial communication
    ├── tcp/               # TCP communication
    ├── config/            # Configuration management
    ├── logging/           # Logging setup
    └── mod.rs
```

## Development Workflow

### 1. Feature Development

1. **Create a Feature Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes**
   - Follow the [Code Standards](#code-standards)
   - Add tests for new functionality
   - Update documentation if needed

3. **Test Your Changes**
   ```bash
   # Run all tests
   cargo test
   
   # Run clippy for linting
   cargo clippy -- -D warnings
   
   # Format code
   cargo fmt
   
   # Check for security vulnerabilities
   cargo audit
   ```

4. **Commit Changes**
   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

### 2. Commit Message Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

Examples:
```
feat: add TCP server mode support
fix: resolve serial port connection timeout
docs: update API documentation for SessionManager
test: add integration tests for communication engine
```

### 3. Branch Naming

Use descriptive branch names:
- `feature/tcp-server-mode`
- `fix/serial-timeout-handling`
- `docs/api-documentation`
- `refactor/session-management`

## Code Standards

### 1. Rust Style Guidelines

Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/):

```rust
// Use snake_case for functions and variables
fn create_session() -> Result<SessionId, TermComError> { ... }

// Use PascalCase for types and traits
pub struct SessionManager { ... }
pub trait Transport { ... }

// Use SCREAMING_SNAKE_CASE for constants
const MAX_RETRIES: u32 = 3;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
```

### 2. Error Handling

Use proper error handling patterns:

```rust
// Good: Use Result for fallible operations
pub async fn send_data(&self, data: Vec<u8>) -> TermComResult<()> {
    self.validate_data(&data)?;
    self.transport.send(data).await
}

// Good: Provide context for errors
.map_err(|e| TermComError::Communication { 
    message: format!("Failed to send data: {}", e) 
})?

// Avoid: Unwrapping in library code
// data.unwrap() // Don't do this
```

### 3. Documentation

Document all public APIs:

```rust
/// Creates a new communication session with the specified configuration.
///
/// # Arguments
///
/// * `config` - The session configuration containing device and connection details
///
/// # Returns
///
/// Returns a `Result` containing the session ID on success, or a `TermComError` on failure.
///
/// # Errors
///
/// This function will return an error if:
/// * The device configuration is invalid
/// * The maximum number of sessions is exceeded
/// * The transport layer fails to initialize
///
/// # Examples
///
/// ```rust
/// let config = SessionConfig::default();
/// let session_id = manager.create_session(config).await?;
/// ```
pub async fn create_session(&self, config: SessionConfig) -> TermComResult<String> {
    // Implementation
}
```

### 4. Async Best Practices

Follow async/await best practices:

```rust
// Good: Use async/await consistently
pub async fn process_messages(&self) -> TermComResult<()> {
    let messages = self.receive_messages().await?;
    for message in messages {
        self.handle_message(message).await?;
    }
    Ok(())
}

// Good: Use appropriate synchronization primitives
let sessions = self.sessions.read().await;
let session = sessions.get(&session_id);
drop(sessions); // Release lock early

// Avoid: Blocking operations in async context
// std::thread::sleep(duration); // Use tokio::time::sleep instead
```

## Testing Guidelines

### 1. Test Structure

Organize tests clearly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_session_creation_success() {
        // Arrange
        let manager = create_test_session_manager();
        let config = create_valid_session_config();
        
        // Act
        let result = manager.create_session(config).await;
        
        // Assert
        assert!(result.is_ok());
        let session_id = result.unwrap();
        assert!(manager.has_session(&session_id).await);
    }
    
    #[tokio::test]
    async fn test_session_creation_exceeds_limit() {
        // Test error conditions
    }
    
    // Helper functions
    fn create_test_session_manager() -> SessionManager {
        // Test setup
    }
    
    fn create_valid_session_config() -> SessionConfig {
        // Test data creation
    }
}
```

### 2. Test Categories

Write different types of tests:

```rust
// Unit tests - test individual components
#[tokio::test]
async fn test_message_parsing() {
    let message = Message::from_bytes(&[0x01, 0x02, 0x03]);
    assert_eq!(message.data, vec![0x01, 0x02, 0x03]);
}

// Integration tests - test component interactions
#[tokio::test]
async fn test_session_communication_flow() {
    let engine = CommunicationEngine::new(100, 5);
    let manager = SessionManager::new(Arc::new(engine), 10);
    
    // Test full workflow
}

// Property tests - test with generated data
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_config_serialization_roundtrip(
        config in any::<SessionConfig>()
    ) {
        let serialized = serde_json::to_string(&config)?;
        let deserialized: SessionConfig = serde_json::from_str(&serialized)?;
        prop_assert_eq!(config, deserialized);
    }
}
```

### 3. Mock and Test Utilities

Create test utilities for complex scenarios:

```rust
// Test utilities module
pub mod test_utils {
    use super::*;
    
    pub fn create_mock_transport() -> MockTransport {
        MockTransport::new()
    }
    
    pub fn create_test_config() -> TermComConfig {
        TermComConfig {
            global: GlobalConfig::default(),
            devices: vec![create_test_device()],
        }
    }
    
    pub fn create_test_device() -> DeviceConfig {
        DeviceConfig {
            name: "test_device".to_string(),
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
}
```

### 4. Performance Tests

Include performance benchmarks for critical paths:

```rust
#[tokio::test]
async fn test_message_throughput() {
    let engine = CommunicationEngine::new(10000, 10);
    let start = std::time::Instant::now();
    
    // Send 1000 messages
    for i in 0..1000 {
        let data = format!("test message {}", i);
        engine.send_data("test_session", data.into_bytes()).await.unwrap();
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(500), "Throughput too low: {:?}", elapsed);
}
```

## Documentation

### 1. Code Documentation

- Document all public APIs with rustdoc comments
- Include examples in documentation
- Explain complex algorithms and design decisions
- Keep documentation up to date with code changes

### 2. User Documentation

- Update README.md for user-facing changes
- Add examples for new features
- Update configuration documentation
- Include troubleshooting information

### 3. Architecture Documentation

- Update architecture diagrams for structural changes
- Document design decisions
- Explain integration points
- Maintain API documentation

## Pull Request Process

### 1. Before Submitting

Ensure your PR meets these criteria:

```bash
# All tests pass
cargo test

# Code is properly formatted
cargo fmt --check

# No clippy warnings
cargo clippy -- -D warnings

# Documentation builds
cargo doc --no-deps

# Security audit passes
cargo audit
```

### 2. PR Description Template

Use this template for PR descriptions:

```markdown
## Summary

Brief description of the changes made.

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Changes Made

- List specific changes
- Include technical details
- Mention any new dependencies

## Testing

- [ ] Added unit tests
- [ ] Added integration tests
- [ ] Manual testing performed
- [ ] Performance impact assessed

## Documentation

- [ ] Code comments updated
- [ ] User documentation updated
- [ ] API documentation updated

## Checklist

- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No breaking changes (or properly documented)
```

### 3. Review Process

1. **Automated Checks**: CI runs tests and checks
2. **Code Review**: Maintainers review code quality and design
3. **Testing**: Reviewers may test functionality
4. **Feedback**: Address any review comments
5. **Approval**: PR approved by maintainers
6. **Merge**: PR merged into main branch

## Issue Reporting

### 1. Bug Reports

Use this template for bug reports:

```markdown
## Bug Description

Clear description of the bug.

## Steps to Reproduce

1. Step one
2. Step two
3. Step three

## Expected Behavior

What should happen.

## Actual Behavior

What actually happens.

## Environment

- OS: [e.g., Ubuntu 20.04]
- Rust version: [e.g., 1.70.0]
- TermCom version: [e.g., 0.1.0]

## Additional Context

Any other relevant information.
```

### 2. Feature Requests

```markdown
## Feature Description

Clear description of the requested feature.

## Use Case

Why is this feature needed?

## Proposed Solution

How should this feature work?

## Alternatives Considered

What other approaches were considered?

## Additional Context

Any other relevant information.
```

## Performance Guidelines

### 1. Memory Usage

- Use bounded collections to prevent memory leaks
- Implement proper cleanup for long-running operations
- Monitor memory usage in tests

```rust
// Good: Bounded message history
if self.message_history.len() > MAX_HISTORY_SIZE {
    self.message_history.pop_front();
}

// Good: Explicit cleanup
impl Drop for Session {
    fn drop(&mut self) {
        // Clean up resources
    }
}
```

### 2. CPU Performance

- Use efficient algorithms and data structures
- Avoid unnecessary allocations
- Profile performance-critical code

```rust
// Good: Use HashMap for O(1) lookups
let session = self.sessions.get(&session_id)?;

// Good: Reuse allocations when possible
let mut buffer = Vec::with_capacity(1024);
```

### 3. I/O Performance

- Use async I/O for all blocking operations
- Implement proper timeout handling
- Buffer I/O operations when appropriate

```rust
// Good: Async I/O with timeouts
let result = tokio::time::timeout(Duration::from_secs(5), operation).await??;

// Good: Buffered I/O
let mut reader = BufReader::new(stream);
```

## Security Considerations

### 1. Input Validation

Always validate external input:

```rust
pub fn validate_port_number(port: u16) -> TermComResult<()> {
    if port == 0 {
        return Err(TermComError::InvalidInput("Port cannot be 0".to_string()));
    }
    Ok(())
}
```

### 2. Error Information

Don't leak sensitive information in error messages:

```rust
// Good: Generic error message
Err(TermComError::Authentication("Invalid credentials".to_string()))

// Bad: Specific information
// Err(TermComError::Authentication(format!("User {} not found", username)))
```

### 3. Dependency Management

- Regularly update dependencies
- Use `cargo audit` to check for vulnerabilities
- Minimize dependency footprint

```bash
# Regular security checks
cargo audit
cargo outdated
```

Thank you for contributing to TermCom! Your contributions help make embedded device development easier and more efficient for everyone.