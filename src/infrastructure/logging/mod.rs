// Logging module - Logging infrastructure
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::io;

/// Initialize logging system
pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("termcom=info,warn,error"));
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(io::stderr)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .init();
    
    tracing::info!("TermCom logging system initialized");
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_logging_init() {
        // Test that logging initialization doesn't panic
        assert!(init_logging().is_ok());
    }
}