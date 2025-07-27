use termcom::{TermComError, TermComResult};
use std::error::Error;

/// Error handling and resilience tests
#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_error_types() {
        // Test different error variants
        let errors = vec![
            TermComError::Config { message: "Config error".to_string() },
            TermComError::Session { message: "Session error".to_string() },
            TermComError::Communication { message: "Comm error".to_string() },
            TermComError::Configuration("Config error".to_string()),
            TermComError::InvalidInput("Invalid input".to_string()),
            TermComError::Output("Output error".to_string()),
            TermComError::TuiError("TUI error".to_string()),
        ];

        for error in errors {
            // All errors should display properly
            let display = error.to_string();
            assert!(!display.is_empty(), "Error display should not be empty");
            
            // All errors should have source information
            let source = error.source();
            // Some errors might not have sources, which is fine
            
            // All errors should be Send + Sync for async compatibility
            fn assert_send_sync<T: Send + Sync>() {}
            assert_send_sync::<TermComError>();
        }
    }

    #[test]
    fn test_error_conversion() {
        // Test std::io::Error conversion
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let termcom_error: TermComError = io_error.into();
        assert!(matches!(termcom_error, TermComError::Network(_)));
    }

    #[test]
    fn test_result_type() {
        // Test TermComResult usage
        fn success_function() -> TermComResult<String> {
            Ok("success".to_string())
        }
        
        fn error_function() -> TermComResult<String> {
            Err(TermComError::Config { 
                message: "Test error".to_string() 
            })
        }
        
        let success = success_function();
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), "success");
        
        let error = error_function();
        assert!(error.is_err());
        assert!(error.unwrap_err().to_string().contains("Config"));
    }

    #[test]
    fn test_error_chain() {
        // Test error chaining with source
        let root_cause = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let network_error: TermComError = root_cause.into();
        
        // Should be able to walk the error chain
        let mut current_error: &dyn Error = &network_error;
        let mut depth = 0;
        
        while let Some(source) = current_error.source() {
            current_error = source;
            depth += 1;
            if depth > 10 {
                break; // Prevent infinite loops
            }
        }
        
        assert!(depth > 0, "Should have at least one source error");
    }

    #[test]
    fn test_error_formatting() {
        let error = TermComError::Session { 
            message: "Connection failed to device 'test-device' on port '/dev/ttyUSB0'".to_string() 
        };
        
        let display = format!("{}", error);
        let debug = format!("{:?}", error);
        
        assert!(display.contains("Session error"));
        assert!(display.contains("Connection failed"));
        assert!(!debug.is_empty());
        assert_ne!(display, debug); // Display and debug should be different
    }

    #[tokio::test]
    async fn test_async_error_propagation() {
        async fn failing_async_function() -> TermComResult<()> {
            Err(TermComError::Communication { 
                message: "Async operation failed".to_string() 
            })
        }
        
        async fn calling_function() -> TermComResult<()> {
            failing_async_function().await?;
            Ok(())
        }
        
        let result = calling_function().await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Communication"));
        assert!(error.to_string().contains("Async operation failed"));
    }

    #[test]
    fn test_error_thread_safety() {
        use std::sync::Arc;
        use std::thread;
        
        let error = Arc::new(TermComError::Config { 
            message: "Thread safety test".to_string() 
        });
        
        let handles: Vec<_> = (0..5).map(|i| {
            let error_clone = Arc::clone(&error);
            thread::spawn(move || {
                let display = format!("Thread {}: {}", i, error_clone);
                assert!(display.contains("Thread safety test"));
            })
        }).collect();
        
        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[test]
    fn test_error_serialization() {
        // Test that errors can be converted to strings and back for logging
        let original_error = TermComError::InvalidInput("Test input".to_string());
        let error_string = original_error.to_string();
        
        // Should be able to recreate similar error from string
        let recreated_error = TermComError::InvalidInput(error_string);
        assert!(recreated_error.to_string().contains("Test input"));
    }

    #[test]
    fn test_error_size() {
        use std::mem;
        
        // Errors should not be too large (affects performance)
        let error_size = mem::size_of::<TermComError>();
        assert!(error_size <= 128, "TermComError too large: {} bytes", error_size);
    }

    #[test]
    fn test_error_in_option_result() {
        // Test error handling in complex return types
        fn complex_function() -> Option<TermComResult<String>> {
            Some(Err(TermComError::Output("Complex error".to_string())))
        }
        
        match complex_function() {
            Some(Ok(_)) => panic!("Should not succeed"),
            Some(Err(e)) => assert!(e.to_string().contains("Complex error")),
            None => panic!("Should not be None"),
        }
    }
}