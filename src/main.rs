// TermCom - Embedded Device Communication Debug Tool
mod cli;
mod tui;
mod core;
mod domain;
mod infrastructure;

use clap::Parser;
use cli::args::Args;
use cli::commands::execute_command;
use domain::error::TermComError;

#[tokio::main]
async fn main() -> Result<(), TermComError> {
    let args = Args::parse();
    
    match execute_command(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
