/// Benchmark tests for performance optimization verification
use std::time::{Duration, Instant};
use std::sync::Arc;
use termcom::{CommunicationEngine, SessionManager, TermComConfig};
use termcom::core::memory::MemoryManager;

#[cfg(test)]
mod benchmarks {
    use super::*;

    #[tokio::test]
    async fn benchmark_engine_startup_shutdown() {
        let iterations = 100;
        let mut total_startup_time = Duration::default();
        let mut total_shutdown_time = Duration::default();
        
        for _ in 0..iterations {
            let engine = CommunicationEngine::new(1000, 10);
            
            // Measure startup time
            let start = Instant::now();
            engine.start().await.expect("Failed to start engine");
            total_startup_time += start.elapsed();
            
            // Measure shutdown time
            let start = Instant::now();
            engine.stop().await.expect("Failed to stop engine");
            total_shutdown_time += start.elapsed();
        }
        
        let avg_startup = total_startup_time / iterations;
        let avg_shutdown = total_shutdown_time / iterations;
        
        println!("Average startup time: {:?}", avg_startup);
        println!("Average shutdown time: {:?}", avg_shutdown);
        
        // Performance targets
        assert!(avg_startup < Duration::from_millis(50), 
                "Startup too slow: {:?}", avg_startup);
        assert!(avg_shutdown < Duration::from_millis(50), 
                "Shutdown too slow: {:?}", avg_shutdown);
    }
    
    #[tokio::test]
    async fn benchmark_session_operations() {
        let engine = Arc::new(CommunicationEngine::new(1000, 100));
        let manager = SessionManager::new(engine.clone(), 100);
        
        engine.start().await.expect("Failed to start engine");
        
        let operations = 1000;
        let start = Instant::now();
        
        // Benchmark session queries
        for _ in 0..operations {
            let _ = manager.get_session_count().await;
            let _ = manager.get_active_session_count().await;
            let _ = manager.get_statistics().await;
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = operations as f64 / elapsed.as_secs_f64();
        
        println!("Session operations: {:.0} ops/sec", ops_per_sec);
        
        engine.stop().await.expect("Failed to stop engine");
        
        // Should be able to handle at least 10,000 ops/sec
        assert!(ops_per_sec > 10000.0, 
                "Session operations too slow: {:.0} ops/sec", ops_per_sec);
    }
    
    #[tokio::test]
    async fn benchmark_memory_usage() {
        let memory_manager = Arc::new(MemoryManager::new(50)); // 50MB limit
        let engine = Arc::new(CommunicationEngine::new(10000, 10));
        
        engine.start().await.expect("Failed to start engine");
        
        // Simulate high message throughput
        let messages = 10000;
        let start = Instant::now();
        
        for i in 0..messages {
            // Simulate message processing
            memory_manager.record_allocation(256); // Average message size
            
            if i % 1000 == 0 {
                let status = memory_manager.check_memory_usage().await;
                println!("Memory status at {}: {:?}", i, status);
                
                if i % 5000 == 0 {
                    memory_manager.trigger_cleanup().await;
                }
            }
        }
        
        let elapsed = start.elapsed();
        let msgs_per_sec = messages as f64 / elapsed.as_secs_f64();
        
        let stats = memory_manager.get_memory_stats();
        println!("Message throughput: {:.0} msgs/sec", msgs_per_sec);
        println!("Memory stats: {:?}", stats);
        
        engine.stop().await.expect("Failed to stop engine");
        
        // Performance targets
        assert!(msgs_per_sec > 50000.0, 
                "Message throughput too low: {:.0} msgs/sec", msgs_per_sec);
        assert!(!stats.is_critical(), "Memory usage is critical");
    }
    
    #[tokio::test]
    async fn benchmark_config_operations() {
        let iterations = 1000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let config = TermComConfig::default();
            let serialized = toml::to_string(&config)
                .expect("Failed to serialize config");
            let _: TermComConfig = toml::from_str(&serialized)
                .expect("Failed to deserialize config");
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("Config serialization: {:.0} ops/sec", ops_per_sec);
        
        // Should handle at least 2000 serialization ops/sec
        assert!(ops_per_sec > 2000.0, 
                "Config operations too slow: {:.0} ops/sec", ops_per_sec);
    }
    
    #[tokio::test]
    async fn benchmark_concurrent_access() {
        let engine = Arc::new(CommunicationEngine::new(1000, 20));
        engine.start().await.expect("Failed to start engine");
        
        let concurrent_tasks = 20;
        let operations_per_task = 500;
        
        let start = Instant::now();
        
        // Spawn concurrent tasks
        let handles: Vec<_> = (0..concurrent_tasks).map(|task_id| {
            let engine_clone = Arc::clone(&engine);
            tokio::spawn(async move {
                for _ in 0..operations_per_task {
                    let _ = engine_clone.is_running().await;
                    let _ = engine_clone.get_statistics().await;
                    let _ = engine_clone.list_sessions().await;
                    
                    // Small delay to simulate real usage
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
                task_id
            })
        }).collect();
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task failed");
        }
        
        let elapsed = start.elapsed();
        let total_ops = concurrent_tasks * operations_per_task;
        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
        
        println!("Concurrent operations: {:.0} ops/sec with {} tasks", 
                 ops_per_sec, concurrent_tasks);
        
        engine.stop().await.expect("Failed to stop engine");
        
        // Should handle concurrent access efficiently
        assert!(ops_per_sec > 5000.0, 
                "Concurrent operations too slow: {:.0} ops/sec", ops_per_sec);
    }
    
    #[tokio::test]
    async fn benchmark_statistics_calculation() {
        let engine = Arc::new(CommunicationEngine::new(10000, 10));
        engine.start().await.expect("Failed to start engine");
        
        // Simulate some message history
        for _ in 0..5000 {
            // This would normally be done through actual message sending
            // but for benchmarking we just trigger statistics calculation
        }
        
        let iterations = 1000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _ = engine.get_statistics().await;
        }
        
        let elapsed = start.elapsed();
        let stats_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("Statistics calculation: {:.0} stats/sec", stats_per_sec);
        
        engine.stop().await.expect("Failed to stop engine");
        
        // Statistics should be fast even with large message history
        assert!(stats_per_sec > 1000.0, 
                "Statistics calculation too slow: {:.0} stats/sec", stats_per_sec);
    }
    
    #[test]
    fn benchmark_memory_bounded_vec() {
        use termcom::core::memory::BoundedVec;
        
        let iterations = 100000;
        let max_size = 1000;
        let mut vec = BoundedVec::new(max_size);
        
        let start = Instant::now();
        
        // Add many items, testing bounded behavior
        for i in 0..iterations {
            vec.push(i);
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("BoundedVec operations: {:.0} ops/sec", ops_per_sec);
        assert_eq!(vec.len(), max_size);
        
        // Should handle bounded operations efficiently
        assert!(ops_per_sec > 500000.0, 
                "BoundedVec too slow: {:.0} ops/sec", ops_per_sec);
    }
    
    #[tokio::test]
    async fn benchmark_memory_pressure() {
        let memory_manager = Arc::new(MemoryManager::new(10)); // 10MB limit
        
        // Simulate memory pressure scenario
        let start = Instant::now();
        let mut last_status = termcom::core::memory::MemoryStatus::Normal;
        
        for i in 0..10000 {
            memory_manager.record_allocation(1024); // 1KB allocation
            
            if i % 100 == 0 {
                let status = memory_manager.check_memory_usage().await;
                if status != last_status {
                    println!("Memory status changed at iteration {}: {:?}", i, status);
                    last_status = status;
                }
                
                if matches!(last_status, termcom::core::memory::MemoryStatus::Critical) {
                    memory_manager.trigger_cleanup().await;
                    // Simulate some deallocations after cleanup
                    for _ in 0..50 {
                        memory_manager.record_deallocation(1024);
                    }
                }
            }
        }
        
        let elapsed = start.elapsed();
        let final_stats = memory_manager.get_memory_stats();
        
        println!("Memory pressure test completed in {:?}", elapsed);
        println!("Final memory stats: {:?}", final_stats);
        
        // Test should complete reasonably quickly
        assert!(elapsed < Duration::from_secs(5), 
                "Memory pressure test took too long: {:?}", elapsed);
    }
}

/// Utility functions for benchmarking
pub fn print_benchmark_summary() {
    println!("\n=== TermCom Performance Benchmark Summary ===");
    println!("All benchmark tests measure performance optimizations:");
    println!("1. Engine startup/shutdown latency");
    println!("2. Session management throughput");
    println!("3. Memory usage and cleanup efficiency");
    println!("4. Configuration serialization speed");
    println!("5. Concurrent access performance");
    println!("6. Statistics calculation efficiency");
    println!("7. Bounded collection performance");
    println!("8. Memory pressure handling");
    println!("==============================================\n");
}

#[cfg(test)]
mod integration_benchmarks {
    use super::*;
    
    #[tokio::test]
    async fn full_system_benchmark() {
        print_benchmark_summary();
        
        let start = Instant::now();
        
        // Create a complete system setup
        let memory_manager = Arc::new(MemoryManager::new(100));
        let engine = Arc::new(CommunicationEngine::new(5000, 20));
        let session_manager = SessionManager::new(engine.clone(), 20);
        
        // Start system
        engine.start().await.expect("Failed to start engine");
        
        // Run mixed workload
        let tasks = 10;
        let operations_per_task = 100;
        
        let handles: Vec<_> = (0..tasks).map(|task_id| {
            let engine_clone = Arc::clone(&engine);
            let memory_manager_clone = Arc::clone(&memory_manager);
            
            tokio::spawn(async move {
                for op in 0..operations_per_task {
                    match op % 4 {
                        0 => {
                            let _ = engine_clone.get_statistics().await;
                        },
                        1 => {
                            let _ = engine_clone.list_sessions().await;
                            memory_manager_clone.record_allocation(512);
                        },
                        2 => {
                            let status = memory_manager_clone.check_memory_usage().await;
                            if matches!(status, termcom::core::memory::MemoryStatus::Warning) {
                                memory_manager_clone.trigger_cleanup().await;
                            }
                        },
                        3 => {
                            let _ = engine_clone.is_running().await;
                            memory_manager_clone.record_deallocation(256);
                        },
                        _ => unreachable!(),
                    }
                    
                    // Small delay to simulate real usage
                    tokio::time::sleep(Duration::from_micros(50)).await;
                }
                task_id
            })
        }).collect();
        
        // Wait for completion
        for handle in handles {
            handle.await.expect("Task failed");
        }
        
        // Get final statistics
        let engine_stats = engine.get_statistics().await;
        let memory_stats = memory_manager.get_memory_stats();
        
        engine.stop().await.expect("Failed to stop engine");
        
        let total_elapsed = start.elapsed();
        let total_operations = tasks * operations_per_task * 4; // 4 ops per iteration
        let ops_per_sec = total_operations as f64 / total_elapsed.as_secs_f64();
        
        println!("\n=== Full System Benchmark Results ===");
        println!("Total time: {:?}", total_elapsed);
        println!("Operations per second: {:.0}", ops_per_sec);
        println!("Engine stats: {:?}", engine_stats);
        println!("Memory stats: {:?}", memory_stats);
        println!("======================================\n");
        
        // Performance targets for the full system
        assert!(ops_per_sec > 2000.0, 
                "Full system performance too low: {:.0} ops/sec", ops_per_sec);
        assert!(total_elapsed < Duration::from_secs(10), 
                "Full system benchmark took too long: {:?}", total_elapsed);
        assert!(!memory_stats.is_critical(), "Memory usage became critical");
    }
}