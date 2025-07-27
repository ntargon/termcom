use std::time::Duration;
use termcom::{TermComConfig, TermComError, SessionType};
use tokio::time::timeout;
use toml;

/// Integration tests for TermCom library
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_config_serialization() {
        let config = TermComConfig::default();
        let toml_str = toml::to_string(&config).expect("Failed to serialize config");
        let deserialized: TermComConfig = toml::from_str(&toml_str)
            .expect("Failed to deserialize config");
        
        assert_eq!(config.global.max_sessions, deserialized.global.max_sessions);
        assert_eq!(config.global.log_level, deserialized.global.log_level);
    }

    #[test]
    fn test_session_type_display() {
        assert_eq!(SessionType::Interactive.to_string(), "Interactive");
        assert_eq!(SessionType::Automated.to_string(), "Automated");
        assert_eq!(SessionType::Monitoring.to_string(), "Monitoring");
        assert_eq!(SessionType::Testing.to_string(), "Testing");
    }

    #[test]
    fn test_error_display() {
        let error = TermComError::Config { 
            message: "Invalid configuration".to_string() 
        };
        assert!(error.to_string().contains("Configuration error"));
        assert!(error.to_string().contains("Invalid configuration"));
    }

    #[tokio::test]
    async fn test_communication_engine_lifecycle() {
        use termcom::CommunicationEngine;
        
        let engine = CommunicationEngine::new(1000, 10);
        
        // Test engine state
        assert!(!engine.is_running().await);
        
        // Test start/stop
        let start_result = engine.start().await;
        assert!(start_result.is_ok());
        assert!(engine.is_running().await);
        
        let stop_result = engine.stop().await;
        assert!(stop_result.is_ok());
        assert!(!engine.is_running().await);
    }

    #[tokio::test]
    async fn test_session_manager_basic_operations() {
        use termcom::{SessionManager, CommunicationEngine};
        use std::sync::Arc;
        
        let engine = Arc::new(CommunicationEngine::new(1000, 10));
        let manager = SessionManager::new(engine, 10);
        
        // Test initial state
        assert_eq!(manager.get_session_count().await, 0);
        assert_eq!(manager.get_max_sessions(), 10);
        
        // Test statistics
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.active_sessions, 0);
    }

    #[tokio::test]
    async fn test_timeout_behavior() {
        // Test that long-running operations can be timed out
        let result = timeout(Duration::from_millis(100), async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            "completed"
        }).await;
        
        assert!(result.is_err()); // Should timeout
    }

    #[test]
    fn test_config_defaults() {
        let config = TermComConfig::default();
        
        assert_eq!(config.global.log_level, "info");
        assert_eq!(config.global.max_sessions, 10);
        assert_eq!(config.global.timeout_ms, 5000);
        assert!(config.global.auto_save);
        assert_eq!(config.global.history_limit, 1000);
        assert!(config.devices.is_empty());
    }
}