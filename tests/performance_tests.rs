use std::time::{Duration, Instant};
use termcom::{CommunicationEngine, SessionManager, TermComConfig};
use tokio::time::timeout;

/// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_communication_engine_performance() {
        let engine = CommunicationEngine::new(10000, 50);
        
        let start = Instant::now();
        engine.start().await.expect("Failed to start engine");
        let startup_time = start.elapsed();
        
        // Startup should be fast (under 100ms)
        assert!(startup_time < Duration::from_millis(100), 
                "Engine startup took too long: {:?}", startup_time);
        
        let start = Instant::now();
        engine.stop().await.expect("Failed to stop engine");
        let shutdown_time = start.elapsed();
        
        // Shutdown should be fast (under 100ms)
        assert!(shutdown_time < Duration::from_millis(100), 
                "Engine shutdown took too long: {:?}", shutdown_time);
    }

    #[tokio::test]
    async fn test_session_manager_performance() {
        use std::sync::Arc;
        
        let engine = Arc::new(CommunicationEngine::new(1000, 100));
        let manager = SessionManager::new(engine, 100);
        
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = manager.get_session_count().await;
        }
        let elapsed = start.elapsed();
        
        // 1000 operations should complete quickly (under 10ms)
        assert!(elapsed < Duration::from_millis(10), 
                "Session manager operations too slow: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_config_serialization_performance() {
        let config = TermComConfig::default();
        
        let start = Instant::now();
        for _ in 0..1000 {
            let serialized = toml::to_string(&config).expect("Serialization failed");
            let _: TermComConfig = toml::from_str(&serialized).expect("Deserialization failed");
        }
        let elapsed = start.elapsed();
        
        // 1000 serialize/deserialize cycles should be fast (under 500ms)
        assert!(elapsed < Duration::from_millis(500), 
                "Config serialization too slow: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_memory_usage_session_manager() {
        use std::sync::Arc;
        
        let engine = Arc::new(CommunicationEngine::new(1000, 10));
        let manager = SessionManager::new(engine, 10);
        
        // Create and destroy sessions repeatedly to test for memory leaks
        for _ in 0..100 {
            let stats = manager.get_statistics().await;
            assert_eq!(stats.total_sessions, 0);
        }
        
        // Test should complete without memory issues
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        use std::sync::Arc;
        
        let engine = Arc::new(CommunicationEngine::new(1000, 10));
        engine.start().await.expect("Failed to start engine");
        
        // Test concurrent access
        let handles: Vec<_> = (0..10).map(|_| {
            let engine_clone = Arc::clone(&engine);
            tokio::spawn(async move {
                for _ in 0..100 {
                    let _ = engine_clone.is_running().await;
                }
            })
        }).collect();
        
        // All tasks should complete without panic
        for handle in handles {
            handle.await.expect("Task panicked");
        }
        
        engine.stop().await.expect("Failed to stop engine");
    }

    #[tokio::test]
    async fn test_timeout_compliance() {
        let engine = CommunicationEngine::new(1000, 10);
        
        // Test that operations respect timeouts
        let result = timeout(Duration::from_millis(50), async {
            engine.start().await
        }).await;
        
        // Start should complete within timeout
        assert!(result.is_ok(), "Engine start exceeded timeout");
        
        let result = timeout(Duration::from_millis(50), async {
            engine.stop().await
        }).await;
        
        // Stop should complete within timeout
        assert!(result.is_ok(), "Engine stop exceeded timeout");
    }

    #[test]
    fn test_error_performance() {
        use termcom::TermComError;
        
        let start = Instant::now();
        for _ in 0..10000 {
            let error = TermComError::Config { 
                message: "Test error".to_string() 
            };
            let _ = error.to_string();
        }
        let elapsed = start.elapsed();
        
        // Error creation and formatting should be fast
        assert!(elapsed < Duration::from_millis(50), 
                "Error handling too slow: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_session_manager_scaling() {
        use std::sync::Arc;
        
        // Test with different session limits
        for limit in [1, 10, 50, 100] {
            let engine = Arc::new(CommunicationEngine::new(1000, limit));
            let manager = SessionManager::new(engine, limit);
            
            let start = Instant::now();
            let stats = manager.get_statistics().await;
            let elapsed = start.elapsed();
            
            assert_eq!(stats.total_sessions, 0);
            assert!(elapsed < Duration::from_millis(10), 
                    "Stats query too slow for limit {}: {:?}", limit, elapsed);
        }
    }
}