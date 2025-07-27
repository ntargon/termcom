use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TermCom configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermComConfig {
    /// Global configuration
    pub global: GlobalConfig,
    /// Device configurations
    pub devices: Vec<DeviceConfig>,
}

/// Global configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Default log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Maximum number of sessions
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
    /// Default timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Auto-save session data
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
    /// History limit
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Device name
    pub name: String,
    /// Device description
    #[serde(default)]
    pub description: String,
    /// Connection type
    pub connection: ConnectionConfig,
    /// Custom commands
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConnectionConfig {
    #[serde(rename = "serial")]
    Serial {
        port: String,
        baud_rate: u32,
        #[serde(default = "default_data_bits")]
        data_bits: u8,
        #[serde(default = "default_stop_bits")]
        stop_bits: u8,
        #[serde(default = "default_parity")]
        parity: ParityConfig,
        #[serde(default = "default_flow_control")]
        flow_control: FlowControlConfig,
    },
    #[serde(rename = "tcp")]
    Tcp {
        host: String,
        port: u16,
        #[serde(default = "default_tcp_timeout")]
        timeout_ms: u64,
        #[serde(default)]
        keep_alive: bool,
    },
}

/// Parity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParityConfig {
    None,
    Odd,
    Even,
}

/// Flow control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowControlConfig {
    None,
    Hardware,
    Software,
}

/// Custom command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    /// Command name
    pub name: String,
    /// Command description
    #[serde(default)]
    pub description: String,
    /// Command template
    pub template: String,
    /// Expected response pattern (regex)
    #[serde(default)]
    pub response_pattern: Option<String>,
    /// Timeout in milliseconds
    #[serde(default = "default_command_timeout")]
    pub timeout_ms: u64,
}

// Default value functions
fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_sessions() -> usize {
    10
}

fn default_timeout() -> u64 {
    5000
}

fn default_auto_save() -> bool {
    true
}

fn default_history_limit() -> usize {
    1000
}

fn default_data_bits() -> u8 {
    8
}

fn default_stop_bits() -> u8 {
    1
}

fn default_parity() -> ParityConfig {
    ParityConfig::None
}

fn default_flow_control() -> FlowControlConfig {
    FlowControlConfig::None
}

fn default_tcp_timeout() -> u64 {
    3000
}

fn default_command_timeout() -> u64 {
    1000
}

impl Default for TermComConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            devices: Vec::new(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            max_sessions: default_max_sessions(),
            timeout_ms: default_timeout(),
            auto_save: default_auto_save(),
            history_limit: default_history_limit(),
        }
    }
}

impl Default for ParityConfig {
    fn default() -> Self {
        default_parity()
    }
}

impl Default for FlowControlConfig {
    fn default() -> Self {
        default_flow_control()
    }
}

/// Serial connection configuration type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: ParityConfig,
    pub flow_control: FlowControlConfig,
    pub timeout: std::time::Duration,
}

/// TCP connection configuration type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConfig {
    pub host: String,
    pub port: u16,
    pub timeout: std::time::Duration,
    pub keep_alive: bool,
    pub no_delay: bool,
}

impl From<SerialConfig> for ConnectionConfig {
    fn from(config: SerialConfig) -> Self {
        ConnectionConfig::Serial {
            port: config.port,
            baud_rate: config.baud_rate,
            data_bits: config.data_bits,
            stop_bits: config.stop_bits,
            parity: config.parity,
            flow_control: config.flow_control,
        }
    }
}

impl From<TcpConfig> for ConnectionConfig {
    fn from(config: TcpConfig) -> Self {
        ConnectionConfig::Tcp {
            host: config.host,
            port: config.port,
            timeout_ms: config.timeout.as_millis() as u64,
            keep_alive: config.keep_alive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = TermComConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let _deserialized: TermComConfig = toml::from_str(&toml_str).unwrap();
    }

    #[test]
    fn test_serial_config() {
        let serial_config = ConnectionConfig::Serial {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: ParityConfig::None,
            flow_control: FlowControlConfig::None,
        };
        
        let config = TermComConfig {
            global: GlobalConfig::default(),
            devices: vec![DeviceConfig {
                name: "test_device".to_string(),
                description: "Test device".to_string(),
                connection: serial_config,
                commands: Vec::new(),
            }],
        };
        
        let toml_str = toml::to_string(&config).unwrap();
        let _deserialized: TermComConfig = toml::from_str(&toml_str).unwrap();
    }

    #[test]
    fn test_tcp_config() {
        let tcp_config = ConnectionConfig::Tcp {
            host: "192.168.1.100".to_string(),
            port: 8080,
            timeout_ms: 3000,
            keep_alive: true,
        };
        
        let config = TermComConfig {
            global: GlobalConfig::default(),
            devices: vec![DeviceConfig {
                name: "tcp_device".to_string(),
                description: "TCP device".to_string(),
                connection: tcp_config,
                commands: Vec::new(),
            }],
        };
        
        let toml_str = toml::to_string(&config).unwrap();
        let _deserialized: TermComConfig = toml::from_str(&toml_str).unwrap();
    }
}