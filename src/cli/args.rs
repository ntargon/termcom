use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

/// Command line arguments for TermCom
#[derive(Parser, Debug)]
#[command(
    name = "termcom",
    version = env!("CARGO_PKG_VERSION"),
    about = "Terminal Communication Debug Tool for Embedded Devices",
    long_about = "A comprehensive communication debug tool for embedded device development supporting serial and TCP communication with session management."
)]
pub struct Args {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub output: OutputFormat,

    /// Command to execute
    #[command(subcommand)]
    pub command: Command,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Serial communication commands
    Serial(SerialArgs),
    /// TCP communication commands
    Tcp(TcpArgs),
    /// Session management commands
    Session(SessionArgs),
    /// Configuration management commands
    Config(ConfigArgs),
    /// Interactive TUI mode
    Tui,
    /// Display version information
    Version,
}

/// Output format options
#[derive(ValueEnum, Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output
    Json,
    /// Table output
    Table,
    /// CSV output
    Csv,
}

/// Serial communication arguments
#[derive(ClapArgs, Debug)]
pub struct SerialArgs {
    /// Serial port path
    #[arg(short, long)]
    pub port: String,

    /// Baud rate
    #[arg(short, long, default_value = "9600")]
    pub baud: u32,

    /// Data bits
    #[arg(long, default_value = "8")]
    pub data_bits: u8,

    /// Stop bits
    #[arg(long, default_value = "1")]
    pub stop_bits: u8,

    /// Parity (none, even, odd)
    #[arg(long, value_enum, default_value = "none")]
    pub parity: ParityArg,

    /// Flow control (none, software, hardware)
    #[arg(long, value_enum, default_value = "none")]
    pub flow_control: FlowControlArg,

    /// Serial subcommand
    #[command(subcommand)]
    pub command: SerialCommand,
}

/// TCP communication arguments
#[derive(ClapArgs, Debug)]
pub struct TcpArgs {
    /// TCP subcommand
    #[command(subcommand)]
    pub command: TcpCommand,
}

/// Session management arguments
#[derive(ClapArgs, Debug)]
pub struct SessionArgs {
    /// Session subcommand
    #[command(subcommand)]
    pub command: SessionCommand,
}

/// Configuration management arguments
#[derive(ClapArgs, Debug)]
pub struct ConfigArgs {
    /// Configuration subcommand
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Serial communication subcommands
#[derive(Subcommand, Debug)]
pub enum SerialCommand {
    /// Connect to a serial device
    Connect {
        /// Device name (optional)
        #[arg(short, long)]
        name: Option<String>,
        /// Session name
        #[arg(short, long)]
        session: Option<String>,
    },
    /// Send data to serial device
    Send {
        /// Data to send (hex or text)
        data: String,
        /// Session ID
        #[arg(short, long)]
        session: Option<String>,
        /// Data format (hex, text, base64)
        #[arg(short, long, value_enum, default_value = "text")]
        format: DataFormat,
    },
    /// List available serial ports
    List,
    /// Monitor serial communication
    Monitor {
        /// Session ID to monitor
        session: Option<String>,
        /// Output file for logging
        #[arg(short, long)]
        output: Option<String>,
    },
}

/// TCP communication subcommands
#[derive(Subcommand, Debug)]
pub enum TcpCommand {
    /// Connect as TCP client
    Connect {
        /// Host address
        host: String,
        /// Port number
        port: u16,
        /// Device name (optional)
        #[arg(short, long)]
        name: Option<String>,
        /// Session name
        #[arg(short, long)]
        session: Option<String>,
        /// Connection timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u64,
    },
    /// Start TCP server
    Server {
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,
        /// Port number
        port: u16,
        /// Device name (optional)
        #[arg(short, long)]
        name: Option<String>,
        /// Session name
        #[arg(short, long)]
        session: Option<String>,
    },
    /// Send data to TCP connection
    Send {
        /// Data to send
        data: String,
        /// Session ID
        #[arg(short, long)]
        session: Option<String>,
        /// Data format (hex, text, base64)
        #[arg(short, long, value_enum, default_value = "text")]
        format: DataFormat,
    },
    /// Monitor TCP communication
    Monitor {
        /// Session ID to monitor
        session: Option<String>,
        /// Output file for logging
        #[arg(short, long)]
        output: Option<String>,
    },
}

/// Session management subcommands
#[derive(Subcommand, Debug)]
pub enum SessionCommand {
    /// List all sessions
    List {
        /// Filter by session type
        #[arg(short, long, value_enum)]
        r#type: Option<SessionTypeArg>,
        /// Filter by status
        #[arg(short, long, value_enum)]
        status: Option<SessionStatusArg>,
        /// Filter by device name
        #[arg(short, long)]
        device: Option<String>,
    },
    /// Show session details
    Show {
        /// Session ID
        id: String,
        /// Include message history
        #[arg(short, long)]
        messages: bool,
        /// Include activity history
        #[arg(short, long)]
        activities: bool,
    },
    /// Start a session
    Start {
        /// Session ID
        id: String,
    },
    /// Stop a session
    Stop {
        /// Session ID
        id: String,
    },
    /// Remove a session
    Remove {
        /// Session ID
        id: String,
    },
    /// Create new session from config
    Create {
        /// Configuration file or device name
        config: String,
        /// Session name
        #[arg(short, long)]
        name: Option<String>,
        /// Session type
        #[arg(short, long, value_enum, default_value = "interactive")]
        r#type: SessionTypeArg,
    },
    /// Export session data
    Export {
        /// Session ID
        id: String,
        /// Output file
        #[arg(short, long)]
        output: String,
        /// Export format
        #[arg(short, long, value_enum, default_value = "json")]
        format: ExportFormat,
    },
    /// Session statistics
    Stats,
}

/// Configuration management subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Validate configuration
    Validate {
        /// Configuration file path
        file: Option<String>,
    },
    /// Create default configuration
    Init {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
        /// Global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// List device configurations
    Devices,
    /// Add device configuration
    AddDevice {
        /// Device name
        name: String,
        /// Device description
        #[arg(short, long)]
        description: Option<String>,
        /// Connection type (serial, tcp)
        #[arg(short, long, value_enum)]
        connection: ConnectionTypeArg,
    },
}

/// Parity configuration argument
#[derive(ValueEnum, Debug, Clone)]
pub enum ParityArg {
    None,
    Even,
    Odd,
}

/// Flow control configuration argument
#[derive(ValueEnum, Debug, Clone)]
pub enum FlowControlArg {
    None,
    Software,
    Hardware,
}

/// Data format argument
#[derive(ValueEnum, Debug, Clone)]
pub enum DataFormat {
    Text,
    Hex,
    Base64,
}

/// Session type argument
#[derive(ValueEnum, Debug, Clone)]
pub enum SessionTypeArg {
    Interactive,
    Automated,
    Monitoring,
    Testing,
}

/// Session status argument
#[derive(ValueEnum, Debug, Clone)]
pub enum SessionStatusArg {
    Initializing,
    Active,
    Disconnected,
    Closing,
    Closed,
    Error,
}

/// Connection type argument
#[derive(ValueEnum, Debug, Clone)]
pub enum ConnectionTypeArg {
    Serial,
    Tcp,
}

/// Export format argument
#[derive(ValueEnum, Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
    Log,
}

impl From<ParityArg> for crate::domain::config::ParityConfig {
    fn from(parity: ParityArg) -> Self {
        match parity {
            ParityArg::None => Self::None,
            ParityArg::Even => Self::Even,
            ParityArg::Odd => Self::Odd,
        }
    }
}

impl From<FlowControlArg> for crate::domain::config::FlowControlConfig {
    fn from(flow_control: FlowControlArg) -> Self {
        match flow_control {
            FlowControlArg::None => Self::None,
            FlowControlArg::Software => Self::Software,
            FlowControlArg::Hardware => Self::Hardware,
        }
    }
}

impl From<SessionTypeArg> for crate::core::session::SessionType {
    fn from(session_type: SessionTypeArg) -> Self {
        match session_type {
            SessionTypeArg::Interactive => Self::Interactive,
            SessionTypeArg::Automated => Self::Automated,
            SessionTypeArg::Monitoring => Self::Monitoring,
            SessionTypeArg::Testing => Self::Testing,
        }
    }
}

impl From<SessionStatusArg> for crate::core::session::SessionStatus {
    fn from(session_status: SessionStatusArg) -> Self {
        match session_status {
            SessionStatusArg::Initializing => Self::Initializing,
            SessionStatusArg::Active => Self::Active,
            SessionStatusArg::Disconnected => Self::Disconnected,
            SessionStatusArg::Closing => Self::Closing,
            SessionStatusArg::Closed => Self::Closed,
            SessionStatusArg::Error => Self::Error("Unknown error".to_string()),
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Csv => write!(f, "csv"),
        }
    }
}

impl std::fmt::Display for DataFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataFormat::Text => write!(f, "text"),
            DataFormat::Hex => write!(f, "hex"),
            DataFormat::Base64 => write!(f, "base64"),
        }
    }
}