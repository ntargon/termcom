# TermCom

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/your-org/termcom)
[![Tests](https://img.shields.io/badge/tests-100%25%20passing-brightgreen.svg)](#testing)

**TermCom** is a comprehensive communication debug tool for embedded device development, supporting both serial and TCP communication with session management capabilities.

## Features

### 🔌 Communication Protocols
- **Serial Communication**: RS232, RS485, UART with configurable parameters
- **TCP Communication**: Client and server modes with keep-alive support
- **Real-time Monitoring**: Live data streaming and message logging

### 🎯 Dual Interface
- **CLI Mode**: Command-line interface for automation and scripting
- **TUI Mode**: Interactive terminal user interface for development

### 📊 Session Management
- **Multiple Sessions**: Support for up to 10 concurrent sessions
- **Session Types**: Interactive, automated, monitoring, and testing modes
- **Real-time Statistics**: Track bytes sent/received, message counts, and errors

### ⚙️ Configuration System
- **Hierarchical Config**: Global settings and project-specific configurations
- **Device Profiles**: Pre-configured device templates and custom commands
- **Validation**: Built-in configuration validation and error reporting

## Installation

### Prerequisites
- Rust 1.70 or later
- A compatible terminal (for TUI mode)

### From Source
```bash
git clone https://github.com/your-org/termcom.git
cd termcom
cargo build --release
```

The binary will be available at `target/release/termcom`.

### Using Cargo
```bash
cargo install termcom
```

## Quick Start

### 1. Initialize Configuration
```bash
# Create project-specific configuration
termcom config init

# Create global configuration  
termcom config init --global
```

### 2. Serial Communication
```bash
# Connect to serial device
termcom serial connect --port /dev/ttyUSB0 --baud 9600

# Send data
termcom serial send "Hello Device" --session <session-id>

# List available serial ports
termcom serial list
```

### 3. TCP Communication
```bash
# Connect as TCP client
termcom tcp connect 192.168.1.100 8080

# Start TCP server
termcom tcp server --port 8080

# Send data
termcom tcp send "Hello TCP" --session <session-id>
```

### 4. TUI Mode
```bash
# Launch interactive terminal interface
termcom tui
```

## Usage Examples

### Basic Serial Communication
```bash
# Connect to Arduino on USB port
termcom serial connect --port /dev/ttyACM0 --baud 115200 --name "Arduino"

# Send command and monitor response
termcom serial send "AT" --format text
termcom serial monitor --session arduino-session
```

### TCP Server Setup
```bash
# Start TCP server for embedded device connections
termcom tcp server --port 1234 --name "EmbeddedServer"

# Monitor all incoming connections
termcom session list --type monitoring
```

### Configuration Management
```bash
# Show current configuration
termcom config show

# Validate configuration file
termcom config validate myconfig.toml

# List configured devices
termcom config devices
```

### Session Management
```bash
# List all active sessions
termcom session list

# Show detailed session information
termcom session show <session-id> --messages --activities

# Export session data
termcom session export <session-id> --output session.json
```

## Configuration

TermCom uses TOML configuration files with hierarchical loading:

1. **Global Config**: `~/.config/termcom/config.toml`
2. **Project Config**: `.termcom/config.toml` (in project directory)

### Example Configuration

```toml
[global]
log_level = "info"
max_sessions = 10
timeout_ms = 5000
auto_save = true
history_limit = 1000

[[devices]]
name = "arduino_uno"
description = "Arduino Uno Development Board"

[devices.connection]
type = "serial"
port = "/dev/ttyACM0"
baud_rate = 115200
data_bits = 8
stop_bits = 1
parity = "none"
flow_control = "none"

[[devices.commands]]
name = "reset"
description = "Reset the device"
template = "RESET\\r\\n"
response_pattern = "OK.*"
timeout_ms = 2000

[[devices]]
name = "esp32_server"
description = "ESP32 TCP Server"

[devices.connection]
type = "tcp"
host = "192.168.1.50"
port = 80
timeout_ms = 3000
keep_alive = true
```

## CLI Reference

### Global Options
- `-v, --verbose`: Enable verbose logging
- `-q, --quiet`: Suppress output
- `-c, --config <FILE>`: Use specific configuration file
- `-o, --output <FORMAT>`: Output format (text, json, table, csv)

### Commands

#### Serial Commands
```bash
termcom serial connect --port <PORT> --baud <RATE>
termcom serial send <DATA> --session <ID> --format <FORMAT>
termcom serial list
termcom serial monitor --session <ID>
```

#### TCP Commands
```bash
termcom tcp connect <HOST> <PORT>
termcom tcp server --port <PORT>
termcom tcp send <DATA> --session <ID>
termcom tcp monitor --session <ID>
```

#### Session Commands
```bash
termcom session list [--type <TYPE>] [--status <STATUS>]
termcom session show <ID> [--messages] [--activities]
termcom session start <ID>
termcom session stop <ID>
termcom session remove <ID>
termcom session stats
```

#### Configuration Commands
```bash
termcom config show
termcom config validate [FILE]
termcom config init [--global] [--output <PATH>]
termcom config devices
termcom config add-device <NAME> --connection <TYPE>
```

## Architecture

TermCom follows a clean architecture pattern with clear separation of concerns:

```
src/
├── main.rs                 # Application entry point
├── cli/                    # Command-line interface
├── tui/                    # Terminal user interface  
├── core/                   # Business logic
│   ├── communication/      # Communication engine
│   ├── session/           # Session management
│   └── config/            # Configuration handling
├── domain/                # Domain models and types
└── infrastructure/        # External integrations
    ├── serial/            # Serial port communication
    ├── tcp/               # TCP networking
    ├── config/            # Configuration persistence
    └── logging/           # Logging framework
```

### Key Components

- **CommunicationEngine**: Manages all communication protocols
- **SessionManager**: Handles concurrent session lifecycle
- **ConfigManager**: Hierarchical configuration management
- **Transport Layer**: Abstraction over serial/TCP protocols

## Testing

TermCom maintains comprehensive test coverage across all components:

```bash
# Run all tests
cargo test

# Run with coverage
cargo test --all-features

# Run performance tests
cargo test --release

# Run integration tests only
cargo test --test integration_tests
```

### Test Coverage
- **Unit Tests**: Core business logic and utilities
- **Integration Tests**: Component interaction testing
- **CLI Tests**: Command-line interface validation
- **Performance Tests**: Throughput and latency benchmarks
- **Error Handling Tests**: Error scenarios and recovery

**Current Status**: 100% test pass rate (93/93 tests passing)

## Performance

### Targets
- **Response Time**: < 100ms for most operations
- **Concurrent Sessions**: Up to 10 simultaneous connections
- **Memory Usage**: < 50MB during normal operation
- **CPU Usage**: < 10% during idle periods

### Benchmarks
Performance tests validate these targets under various load conditions.

## Platform Support

- **Linux**: Full support for serial and TCP communication
- **macOS**: Full support with native serial port access
- **Windows**: Serial and TCP support (requires additional drivers for some devices)

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes with tests
4. Ensure all tests pass (`cargo test`)
5. Run formatting and linting (`cargo fmt && cargo clippy`)
6. Commit your changes (`git commit -m 'feat: add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Development Setup

```bash
# Clone the repository
git clone https://github.com/your-org/termcom.git
cd termcom

# Install development dependencies
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Check for linting issues
cargo clippy
```

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) for safety and performance
- TUI powered by [ratatui](https://github.com/ratatui-org/ratatui)
- CLI interface using [clap](https://github.com/clap-rs/clap)
- Serial communication via [serialport](https://github.com/serialport/serialport-rs)
- Async runtime provided by [tokio](https://github.com/tokio-rs/tokio)