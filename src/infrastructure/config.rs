use crate::domain::{config::TermComConfig, error::{TermComError, TermComResult}};
use std::path::{Path, PathBuf};
use std::fs;

/// Configuration manager
pub struct ConfigManager {
    global_config_path: PathBuf,
    project_config_path: Option<PathBuf>,
}

impl ConfigManager {
    /// Create new configuration manager
    pub fn new() -> TermComResult<Self> {
        let global_config_path = Self::get_global_config_path()?;
        let project_config_path = Self::find_project_config_path();
        
        Ok(Self {
            global_config_path,
            project_config_path,
        })
    }

    /// Load configuration from files
    pub fn load_config(&self) -> TermComResult<TermComConfig> {
        // Start with default configuration
        let mut config = TermComConfig::default();
        
        // Load global configuration if exists
        if self.global_config_path.exists() {
            let global_config = self.load_config_from_path(&self.global_config_path)?;
            config.global = global_config.global;
        }
        
        // Load and merge project configuration if exists
        if let Some(project_path) = &self.project_config_path {
            if project_path.exists() {
                let project_config = self.load_config_from_path(project_path)?;
                // Merge project devices with existing devices
                config.devices.extend(project_config.devices);
            }
        }
        
        Ok(config)
    }

    /// Save configuration to files
    pub fn save_config(&self, config: &TermComConfig) -> TermComResult<()> {
        // Ensure global config directory exists
        if let Some(parent) = self.global_config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| TermComError::Config {
                message: format!("Failed to create config directory: {}", e),
            })?;
        }
        
        // Save global configuration
        let global_config = TermComConfig {
            global: config.global.clone(),
            devices: Vec::new(), // Global config doesn't contain devices
        };
        self.save_config_to_path(&self.global_config_path, &global_config)?;
        
        // Save project configuration if path is available
        if let Some(project_path) = &self.project_config_path {
            let project_config = TermComConfig {
                global: crate::domain::config::GlobalConfig::default(), // Use default for project
                devices: config.devices.clone(),
            };
            
            // Ensure project config directory exists
            if let Some(parent) = project_path.parent() {
                fs::create_dir_all(parent).map_err(|e| TermComError::Config {
                    message: format!("Failed to create project config directory: {}", e),
                })?;
            }
            
            self.save_config_to_path(project_path, &project_config)?;
        }
        
        Ok(())
    }

    /// Get global configuration path
    fn get_global_config_path() -> TermComResult<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| TermComError::Config {
            message: "Could not determine home directory".to_string(),
        })?;
        
        Ok(home.join(".config").join("termcom").join("config.toml"))
    }

    /// Find project configuration path by walking up directory tree
    fn find_project_config_path() -> Option<PathBuf> {
        let current_dir = std::env::current_dir().ok()?;
        let mut path = current_dir.as_path();
        
        loop {
            let config_path = path.join(".termcom").join("config.toml");
            if config_path.exists() {
                return Some(config_path);
            }
            
            path = path.parent()?;
        }
    }

    /// Load configuration from specific path
    pub fn load_config_from_path(&self, path: &Path) -> TermComResult<TermComConfig> {
        let content = fs::read_to_string(path).map_err(|e| TermComError::Config {
            message: format!("Failed to read config file {}: {}", path.display(), e),
        })?;
        
        toml::from_str(&content).map_err(|e| TermComError::Config {
            message: format!("Failed to parse config file {}: {}", path.display(), e),
        })
    }

    /// Save configuration to specific path
    pub fn save_config_to_path(&self, path: &Path, config: &TermComConfig) -> TermComResult<()> {
        let content = toml::to_string_pretty(config).map_err(|e| TermComError::Config {
            message: format!("Failed to serialize config: {}", e),
        })?;
        
        fs::write(path, content).map_err(|e| TermComError::Config {
            message: format!("Failed to write config file {}: {}", path.display(), e),
        })
    }

    /// Create default project configuration
    pub fn init_project_config(&self, path: &Path) -> TermComResult<()> {
        let config_dir = path.join(".termcom");
        let config_file = config_dir.join("config.toml");
        
        if config_file.exists() {
            return Err(TermComError::Config {
                message: "Project configuration already exists".to_string(),
            });
        }
        
        fs::create_dir_all(&config_dir).map_err(|e| TermComError::Config {
            message: format!("Failed to create .termcom directory: {}", e),
        })?;
        
        let default_config = TermComConfig {
            global: crate::domain::config::GlobalConfig::default(),
            devices: vec![
                crate::domain::config::DeviceConfig {
                    name: "example_serial".to_string(),
                    description: "Example serial device".to_string(),
                    connection: crate::domain::config::ConnectionConfig::Serial {
                        port: "/dev/ttyUSB0".to_string(),
                        baud_rate: 9600,
                        data_bits: 8,
                        stop_bits: 1,
                        parity: crate::domain::config::ParityConfig::None,
                        flow_control: crate::domain::config::FlowControlConfig::None,
                    },
                    commands: vec![
                        crate::domain::config::CustomCommand {
                            name: "status".to_string(),
                            description: "Get device status".to_string(),
                            template: "STATUS\\r\\n".to_string(),
                            response_pattern: Some("OK.*".to_string()),
                            timeout_ms: 1000,
                        },
                    ],
                },
                crate::domain::config::DeviceConfig {
                    name: "example_tcp".to_string(),
                    description: "Example TCP device".to_string(),
                    connection: crate::domain::config::ConnectionConfig::Tcp {
                        host: "192.168.1.100".to_string(),
                        port: 8080,
                        timeout_ms: 3000,
                        keep_alive: true,
                    },
                    commands: Vec::new(),
                },
            ],
        };
        
        self.save_config_to_path(&config_file, &default_config)?;
        
        Ok(())
    }

    /// Get the current project config path (if any)
    pub fn get_project_config_path(&self) -> Option<&PathBuf> {
        self.project_config_path.as_ref()
    }

    /// Get the global config path
    pub fn get_global_config_path_ref(&self) -> &PathBuf {
        &self.global_config_path
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ConfigManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_manager_creation() {
        let _manager = ConfigManager::new().unwrap();
    }

    #[test]
    fn test_load_default_config() {
        let manager = ConfigManager::new().unwrap();
        let config = manager.load_config().unwrap();
        
        assert_eq!(config.global.log_level, "info");
        assert_eq!(config.global.max_sessions, 10);
        assert!(config.devices.is_empty());
    }

    #[test]
    fn test_init_project_config() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ConfigManager::new().unwrap();
        
        manager.init_project_config(temp_dir.path()).unwrap();
        
        let config_file = temp_dir.path().join(".termcom").join("config.toml");
        assert!(config_file.exists());
        
        let content = fs::read_to_string(&config_file).unwrap();
        let config: TermComConfig = toml::from_str(&content).unwrap();
        assert_eq!(config.devices.len(), 2);
    }
}