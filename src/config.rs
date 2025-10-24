// Configuration management module

use crate::detector::PhpInstallation;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub settings: Settings,
    pub versions: Vec<VersionEntry>,
    #[serde(default)]
    pub tools: ToolsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub last_scan: Option<String>,
    pub default_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionEntry {
    pub version: String,
    pub paths: Vec<PathBuf>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolsConfig {
    #[serde(default)]
    pub scan_for_tools: bool,
    #[serde(default)]
    pub custom_tool_names: Vec<String>,
    #[serde(default)]
    pub custom_search_paths: Vec<PathBuf>,
    #[serde(default)]
    pub managed: Vec<ToolEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolEntry {
    pub name: String,
    pub original_path: PathBuf,
    pub shebang: String,
    pub shim_created: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            versions: Vec::new(),
            tools: ToolsConfig::default(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_scan: None,
            default_version: None,
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            scan_for_tools: false, // Opt-in by default
            custom_tool_names: Vec::new(),
            custom_search_paths: Vec::new(),
            managed: Vec::new(),
        }
    }
}

impl Config {
    pub fn update_from_installations(&mut self, installations: &[PhpInstallation]) {
        self.versions.clear();

        for installation in installations {
            self.versions.push(VersionEntry {
                version: installation.version.to_string(),
                paths: installation.paths.clone(),
                source: "auto".to_string(),
            });
        }

        // Update last scan timestamp
        self.settings.last_scan = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Get all paths for a version matching the pattern
    pub fn get_installation_by_version(&self, version_pattern: &str) -> Option<Vec<PathBuf>> {
        use crate::version::PhpVersion;

        for entry in &self.versions {
            if let Ok(version) = PhpVersion::from_php_output(&format!("PHP {}", entry.version)) {
                if version.matches(version_pattern) {
                    return Some(entry.paths.clone());
                }
            }
        }

        None
    }

    /// Get the primary PHP binary path for a version matching the pattern
    pub fn get_primary_path_by_version(&self, version_pattern: &str) -> Option<PathBuf> {
        self.get_installation_by_version(version_pattern)
            .and_then(|paths| {
                // Prefer the binary named exactly "php"
                paths
                    .iter()
                    .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("php"))
                    .or_else(|| paths.first())
                    .cloned()
            })
    }
}

/// Get the path to the config file
pub fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let config_dir = home.join(".php-switcher");
    Ok(config_dir.join("config.toml"))
}

/// Get the config directory
pub fn get_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".php-switcher"))
}

/// Save config to a file
pub fn save_config_to_file<P: AsRef<Path>>(config: &Config, path: P) -> Result<()> {
    let path = path.as_ref();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("Failed to create config directory: {}", e))?;
    }

    let toml_str =
        toml::to_string_pretty(config).map_err(|e| anyhow!("Failed to serialize config: {}", e))?;

    std::fs::write(path, toml_str)
        .map_err(|e| anyhow!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Load config from a file
pub fn load_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
    let path = path.as_ref();

    if !path.exists() {
        // Return default config if file doesn't exist
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("Failed to read config file: {}", e))?;

    let config: Config =
        toml::from_str(&contents).map_err(|e| anyhow!("Failed to parse config: {}", e))?;

    Ok(config)
}

/// Load config from the default location
pub fn load_config() -> Result<Config> {
    let path = get_config_path()?;
    load_config_from_file(path)
}

/// Save config to the default location
pub fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    save_config_to_file(config, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert!(config.settings.last_scan.is_none());
        assert!(config.settings.default_version.is_none());
        assert!(config.versions.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.settings.default_version = Some("8.2".to_string());
        config.versions.push(VersionEntry {
            version: "8.2.12".to_string(),
            paths: vec![PathBuf::from("/usr/bin/php8.2"), PathBuf::from("/usr/bin/php-cgi")],
            source: "auto".to_string(),
        });

        // Serialize to TOML
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("8.2.12"));
        assert!(toml_str.contains("/usr/bin/php8.2"));

        // Deserialize back
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_get_config_path() {
        let path = get_config_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(".php-switcher"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.settings.default_version = Some("8.2".to_string());
        config.versions.push(VersionEntry {
            version: "8.2.12".to_string(),
            paths: vec![PathBuf::from("/usr/bin/php8.2")],
            source: "auto".to_string(),
        });

        // Save config
        let save_result = save_config_to_file(&config, &config_file);
        assert!(save_result.is_ok());
        assert!(config_file.exists());

        // Load config
        let loaded = load_config_from_file(&config_file);
        assert!(loaded.is_ok());
        let loaded_config = loaded.unwrap();
        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_load_nonexistent_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("nonexistent.toml");

        let result = load_config_from_file(&config_file);
        // Should return default config when file doesn't exist
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_update_versions_from_installations() {
        use crate::version::PhpVersion;

        let mut config = Config::default();
        let installations = vec![
            PhpInstallation::new(
                PhpVersion::new(8, 2, 12),
                PathBuf::from("/usr/bin/php8.2"),
            ),
            PhpInstallation::new(
                PhpVersion::new(7, 4, 33),
                PathBuf::from("/usr/bin/php7.4"),
            ),
        ];

        config.update_from_installations(&installations);

        assert_eq!(config.versions.len(), 2);
        assert_eq!(config.versions[0].version, "8.2.12");
        assert_eq!(config.versions[1].version, "7.4.33");
    }

    #[test]
    fn test_tools_config_default() {
        let tools_config = ToolsConfig::default();

        assert!(!tools_config.scan_for_tools); // Should be false (opt-in)
        assert!(tools_config.custom_tool_names.is_empty());
        assert!(tools_config.custom_search_paths.is_empty());
        assert!(tools_config.managed.is_empty());
    }

    #[test]
    fn test_tools_config_serialization() {
        let mut config = Config::default();
        config.tools.scan_for_tools = true;
        config.tools.custom_tool_names = vec!["my-tool".to_string()];
        config.tools.custom_search_paths = vec![PathBuf::from("/opt/bin")];
        config.tools.managed.push(ToolEntry {
            name: "composer".to_string(),
            original_path: PathBuf::from("/usr/bin/composer"),
            shebang: "#!/usr/bin/php".to_string(),
            shim_created: true,
        });

        // Serialize to TOML
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("scan_for_tools = true"));
        assert!(toml_str.contains("composer"));
        assert!(toml_str.contains("#!/usr/bin/php"));

        // Deserialize back
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_tool_entry_serialization() {
        let entry = ToolEntry {
            name: "phpunit".to_string(),
            original_path: PathBuf::from("/usr/local/bin/phpunit"),
            shebang: "#!/usr/bin/env php".to_string(),
            shim_created: false,
        };

        // Serialize
        let toml_str = toml::to_string(&entry).unwrap();
        assert!(toml_str.contains("phpunit"));
        assert!(toml_str.contains("/usr/local/bin/phpunit"));
        assert!(toml_str.contains("#!/usr/bin/env php"));
        assert!(toml_str.contains("shim_created = false"));

        // Deserialize
        let deserialized: ToolEntry = toml::from_str(&toml_str).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_config_with_tools() {
        let mut config = Config::default();
        config.tools.scan_for_tools = true;
        config.tools.managed.push(ToolEntry {
            name: "composer".to_string(),
            original_path: PathBuf::from("/usr/bin/composer"),
            shebang: "#!/usr/bin/php".to_string(),
            shim_created: true,
        });

        // Tools config should be part of the main config
        assert!(config.tools.scan_for_tools);
        assert_eq!(config.tools.managed.len(), 1);
        assert_eq!(config.tools.managed[0].name, "composer");
    }
}
