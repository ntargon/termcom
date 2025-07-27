// CLI module - Command line interface
pub mod args;
pub mod commands;
pub mod output;

pub use args::{Args, Command, OutputFormat};
pub use commands::execute_command;
pub use output::OutputWriter;