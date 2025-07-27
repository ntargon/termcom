// TermCom - Embedded Device Communication Debug Tool
mod cli;
mod tui;
mod core;
mod domain;
mod infrastructure;

use domain::error::TermComResult;
use infrastructure::logging;

fn main() -> TermComResult<()> {
    // Initialize logging system
    logging::init_logging().map_err(|e| domain::error::TermComError::Config { 
        message: format!("Failed to initialize logging: {}", e) 
    })?;
    
    logging::info!("TermCom - Embedded Device Communication Debug Tool");
    logging::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    
    Ok(())
}
