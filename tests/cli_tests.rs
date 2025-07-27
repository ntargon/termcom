use std::process::Command;
use std::str;

/// CLI interface tests
#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn test_cli_help() {
        let output = Command::new("cargo")
            .args(["run", "--", "--help"])
            .output()
            .expect("Failed to execute command");

        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
        
        // Check that help contains expected sections
        assert!(stdout.contains("comprehensive communication debug tool"));
        assert!(stdout.contains("Usage:"));
        assert!(stdout.contains("Commands:"));
        assert!(stdout.contains("serial"));
        assert!(stdout.contains("tcp"));
        assert!(stdout.contains("session"));
        assert!(stdout.contains("config"));
        assert!(stdout.contains("tui"));
    }

    #[test]
    fn test_cli_version() {
        let output = Command::new("cargo")
            .args(["run", "--", "version"])
            .output()
            .expect("Failed to execute command");

        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
        assert!(stdout.contains("0.1.0") || output.status.success());
    }

    #[test]
    fn test_cli_serial_help() {
        let output = Command::new("cargo")
            .args(["run", "--", "serial", "--help"])
            .output()
            .expect("Failed to execute command");

        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
        
        // Check serial-specific help
        assert!(stdout.contains("Serial communication commands") || 
                stdout.contains("--port") ||
                stdout.contains("--baud"));
    }

    #[test]
    fn test_cli_tcp_help() {
        let output = Command::new("cargo")
            .args(["run", "--", "tcp", "--help"])
            .output()
            .expect("Failed to execute command");

        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
        
        // Check TCP-specific help
        assert!(stdout.contains("TCP communication commands") || 
                stdout.contains("connect") ||
                stdout.contains("server"));
    }

    #[test]
    fn test_cli_session_help() {
        let output = Command::new("cargo")
            .args(["run", "--", "session", "--help"])
            .output()
            .expect("Failed to execute command");

        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
        
        // Check session management help
        assert!(stdout.contains("Session management commands") || 
                stdout.contains("list") ||
                stdout.contains("show"));
    }

    #[test]
    fn test_cli_config_help() {
        let output = Command::new("cargo")
            .args(["run", "--", "config", "--help"])
            .output()
            .expect("Failed to execute command");

        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
        
        // Check config management help
        assert!(stdout.contains("Configuration management commands") || 
                stdout.contains("show") ||
                stdout.contains("init"));
    }

    #[test]
    fn test_cli_invalid_command() {
        let output = Command::new("cargo")
            .args(["run", "--", "invalid-command"])
            .output()
            .expect("Failed to execute command");

        // Should fail with invalid command
        assert!(!output.status.success());
    }

    #[test]
    fn test_cli_output_formats() {
        // Test JSON output format
        let output = Command::new("cargo")
            .args(["run", "--", "--output", "json", "session", "stats"])
            .output()
            .expect("Failed to execute command");

        // Should accept the format (even if command might fail due to no sessions)
        let stderr = str::from_utf8(&output.stderr).expect("Invalid UTF-8");
        assert!(!stderr.contains("invalid value 'json'"));
    }

    #[test]
    fn test_cli_verbose_flag() {
        let output = Command::new("cargo")
            .args(["run", "--", "-v", "--help"])
            .output()
            .expect("Failed to execute command");

        // Verbose flag should be accepted
        let stderr = str::from_utf8(&output.stderr).expect("Invalid UTF-8");
        assert!(!stderr.contains("unexpected argument"));
    }

    #[test]
    fn test_cli_quiet_flag() {
        let output = Command::new("cargo")
            .args(["run", "--", "-q", "--help"])
            .output()
            .expect("Failed to execute command");

        // Quiet flag should be accepted
        let stderr = str::from_utf8(&output.stderr).expect("Invalid UTF-8");
        assert!(!stderr.contains("unexpected argument"));
    }
}