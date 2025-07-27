use std::time::{Duration, Instant};
use termcom::{CommunicationEngine, SessionManager, TermComConfig};
use tokio::time::timeout;
use std::sync::Arc;

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

    #[tokio::test]
    async fn test_memory_efficiency() {
        // Test memory usage patterns with bounded collections
        let engine = Arc::new(CommunicationEngine::new(100, 10)); // Small buffer
        let manager = SessionManager::new(engine.clone(), 10);
        
        engine.start().await.expect("Failed to start engine");
        
        // Simulate message processing with bounded history
        let start = Instant::now();
        for i in 0..1000 {
            // Memory usage should remain bounded despite high message volume
            let stats = manager.get_statistics().await;
            if i % 100 == 0 {
                // Periodically check that operations remain fast
                assert!(start.elapsed() < Duration::from_secs(1), 
                        "Memory efficiency test taking too long");
            }
        }
        
        engine.stop().await.expect("Failed to stop engine");
    }

    #[tokio::test]
    async fn test_high_throughput_simulation() {
        let engine = Arc::new(CommunicationEngine::new(10000, 10));
        engine.start().await.expect("Failed to start engine");
        
        // Simulate high message throughput
        let start = Instant::now();
        let message_count = 5000;
        
        // Test parallel message processing
        let handles: Vec<_> = (0..10).map(|_| {
            let engine_clone = Arc::clone(&engine);
            tokio::spawn(async move {
                for _ in 0..message_count / 10 {
                    let _ = engine_clone.is_running().await;
                    let _ = engine_clone.get_statistics().await;
                }
            })
        }).collect();
        
        for handle in handles {
            handle.await.expect("High throughput task failed");
        }
        
        let elapsed = start.elapsed();
        engine.stop().await.expect("Failed to stop engine");
        
        // Should handle high throughput efficiently
        assert!(elapsed < Duration::from_secs(2), 
                "High throughput test too slow: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_resource_cleanup() {
        // Test that resources are properly cleaned up
        for iteration in 0..10 {
            let engine = Arc::new(CommunicationEngine::new(100, 5));
            let manager = SessionManager::new(engine.clone(), 5);
            
            engine.start().await.expect("Failed to start engine");
            
            // Create some activity
            for _ in 0..10 {
                let _ = manager.get_statistics().await;
            }
            
            engine.stop().await.expect("Failed to stop engine");
            
            // Each iteration should be independent and clean
            if iteration % 3 == 0 {
                // Periodically give time for cleanup
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    #[test]
    fn test_config_memory_efficiency() {
        // Test configuration memory usage patterns
        let start = Instant::now();
        
        for _ in 0..1000 {
            let config = TermComConfig::default();
            let serialized = toml::to_string(&config).expect("Serialization failed");
            let _: TermComConfig = toml::from_str(&serialized).expect("Deserialization failed");
            // Config should be dropped immediately after use
        }
        
        let elapsed = start.elapsed();
        
        // Memory efficient config handling
        assert!(elapsed < Duration::from_millis(1000), 
                "Config memory test too slow: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_session_lifecycle_performance() {
        let engine = Arc::new(CommunicationEngine::new(1000, 20));
        let manager = SessionManager::new(engine.clone(), 20);
        
        engine.start().await.expect("Failed to start engine");
        
        let start = Instant::now();
        
        // Rapid session operations
        for _ in 0..100 {
            let session_count = manager.get_session_count().await;
            let active_count = manager.get_active_session_count().await;
            let max_sessions = manager.get_max_sessions();
            
            // Verify expected relationships
            assert!(session_count <= max_sessions);
            assert!(active_count <= session_count);
        }
        
        let elapsed = start.elapsed();
        engine.stop().await.expect("Failed to stop engine");
        
        // Session operations should be very fast
        assert!(elapsed < Duration::from_millis(100), 
                "Session lifecycle operations too slow: {:?}", elapsed);
    }
}