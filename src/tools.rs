// PHP tool detection and shim management module

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Represents a detected PHP tool
#[derive(Debug, Clone, PartialEq)]
pub struct PhpTool {
    pub name: String,
    pub original_path: PathBuf,
    pub shebang: String,
}

/// Common PHP tools to detect
const COMMON_PHP_TOOLS: &[&str] = &[
    "composer",
    "phpunit",
    "psalm",
    "phpstan",
    "rector",
    "php-cs-fixer",
    "phpize",
    "php-config",
];

/// Read the shebang line from an executable file
pub fn read_shebang<P: AsRef<Path>>(path: P) -> Result<String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path.as_ref())
        .map_err(|e| anyhow!("Failed to open file: {}", e))?;

    let mut reader = BufReader::new(file);
    let mut first_line = String::new();

    reader
        .read_line(&mut first_line)
        .map_err(|e| anyhow!("Failed to read shebang: {}", e))?;

    let shebang = first_line.trim().to_string();

    // Validate it's actually a shebang
    if !shebang.starts_with("#!") {
        return Err(anyhow!("No valid shebang found"));
    }

    Ok(shebang)
}

/// Determine if a tool needs a shim based on its shebang
pub fn needs_shim(shebang: &str) -> bool {
    // Empty or invalid shebangs don't need shims
    if shebang.is_empty() || !shebang.starts_with("#!") {
        return false;
    }

    // If it uses /usr/bin/env, it already respects PATH
    if shebang.contains("/env ") || shebang.contains("/env\t") {
        return false;
    }

    // If it directly references a PHP binary path, it needs a shim
    shebang.contains("php")
}

/// Scan PATH for common PHP tools
pub fn scan_for_php_tools(
    custom_tools: &[String],
    custom_paths: &[PathBuf],
) -> Result<Vec<PhpTool>> {
    use std::env;

    let mut tools = Vec::new();

    // Combine common tools with custom tools
    let mut tool_names: Vec<String> = COMMON_PHP_TOOLS.iter().map(|s| s.to_string()).collect();
    tool_names.extend_from_slice(custom_tools);

    // Get search paths: custom paths + PATH environment variable
    let mut search_paths = custom_paths.to_vec();

    if let Ok(path_var) = env::var("PATH") {
        for path_str in path_var.split(':') {
            search_paths.push(PathBuf::from(path_str));
        }
    }

    // Search for each tool
    for tool_name in &tool_names {
        for search_path in &search_paths {
            let tool_path = search_path.join(tool_name);

            // Check if the tool exists and is executable
            if tool_path.exists() && tool_path.is_file() {
                // Try to read shebang
                if let Ok(shebang) = read_shebang(&tool_path) {
                    tools.push(PhpTool {
                        name: tool_name.clone(),
                        original_path: tool_path.clone(),
                        shebang,
                    });

                    // Found this tool, move to next
                    break;
                }
            }
        }
    }

    Ok(tools)
}

/// Create a shim script for a PHP tool
pub fn create_shim<P: AsRef<Path>>(tool: &PhpTool, bin_dir: P) -> Result<PathBuf> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let bin_dir = bin_dir.as_ref();

    // Create bin directory if it doesn't exist
    fs::create_dir_all(bin_dir)?;

    // Determine home directory for shim script
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let switcher_php = home.join(".php-switcher/bin/php");

    // Create shim content
    let shim_content = format!(
        r#"#!/bin/bash
# Auto-generated shim for {} by php-switcher
# Original: {}
exec {} {} "$@"
"#,
        tool.name,
        tool.original_path.display(),
        switcher_php.display(),
        tool.original_path.display()
    );

    // Write shim to bin directory
    let shim_path = bin_dir.join(&tool.name);
    fs::write(&shim_path, shim_content)?;

    // Make shim executable (755 permissions)
    fs::set_permissions(&shim_path, fs::Permissions::from_mode(0o755))?;

    Ok(shim_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    #[test]
    fn test_read_shebang_hardcoded() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("composer");

        // Create a script with hardcoded shebang
        fs::write(&script_path, "#!/usr/bin/php\n<?php\necho 'test';").unwrap();

        let shebang = read_shebang(&script_path).unwrap();
        assert_eq!(shebang, "#!/usr/bin/php");
    }

    #[test]
    fn test_read_shebang_env() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("phpunit");

        // Create a script with env shebang
        fs::write(&script_path, "#!/usr/bin/env php\n<?php\necho 'test';").unwrap();

        let shebang = read_shebang(&script_path).unwrap();
        assert_eq!(shebang, "#!/usr/bin/env php");
    }

    #[test]
    fn test_read_shebang_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("not-executable");

        // Create a file without shebang
        fs::write(&script_path, "<?php\necho 'test';").unwrap();

        let result = read_shebang(&script_path);
        // Should return an error or empty string for non-shebang files
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_read_shebang_nonexistent() {
        let result = read_shebang("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_needs_shim_hardcoded() {
        assert!(needs_shim("#!/usr/bin/php"));
        assert!(needs_shim("#!/bin/php"));
        assert!(needs_shim("#!/usr/local/bin/php"));
    }

    #[test]
    fn test_needs_shim_env() {
        assert!(!needs_shim("#!/usr/bin/env php"));
        assert!(!needs_shim("#!/bin/env php"));
    }

    #[test]
    fn test_needs_shim_edge_cases() {
        assert!(!needs_shim(""));
        assert!(!needs_shim("#!"));
        assert!(needs_shim("#!/opt/php/bin/php"));
    }

    #[test]
    fn test_scan_for_tools_in_path() {
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        // Create fake composer with hardcoded shebang
        let composer_path = bin_dir.join("composer");
        fs::write(&composer_path, "#!/usr/bin/php\n<?php\necho 'composer';").unwrap();
        fs::set_permissions(&composer_path, fs::Permissions::from_mode(0o755)).unwrap();

        // Create fake phpunit with env shebang
        let phpunit_path = bin_dir.join("phpunit");
        fs::write(&phpunit_path, "#!/usr/bin/env php\n<?php\necho 'phpunit';").unwrap();
        fs::set_permissions(&phpunit_path, fs::Permissions::from_mode(0o755)).unwrap();

        // Scan with custom path
        let tools = scan_for_php_tools(&[], &[bin_dir.clone()]).unwrap();

        // Should find both tools
        assert!(tools.len() >= 2);

        // Find composer
        let composer = tools.iter().find(|t| t.name == "composer");
        assert!(composer.is_some());
        assert_eq!(composer.unwrap().shebang, "#!/usr/bin/php");

        // Find phpunit
        let phpunit = tools.iter().find(|t| t.name == "phpunit");
        assert!(phpunit.is_some());
        assert_eq!(phpunit.unwrap().shebang, "#!/usr/bin/env php");
    }

    #[test]
    fn test_scan_ignores_missing_tools() {
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        // Scan only the empty custom path (don't use system PATH)
        // This test verifies we don't error when tools aren't found
        let custom_tools = vec!["nonexistent-tool-12345".to_string()];
        let tools = scan_for_php_tools(&custom_tools, &[bin_dir]).unwrap();

        // Should not contain the nonexistent tool
        assert!(!tools.iter().any(|t| t.name == "nonexistent-tool-12345"));
    }

    #[test]
    fn test_scan_with_custom_tools() {
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        // Create a custom tool
        let custom_path = bin_dir.join("my-custom-tool");
        fs::write(&custom_path, "#!/usr/bin/php\n<?php\necho 'custom';").unwrap();
        fs::set_permissions(&custom_path, fs::Permissions::from_mode(0o755)).unwrap();

        // Scan with custom tool name
        let custom_tools = vec!["my-custom-tool".to_string()];
        let tools = scan_for_php_tools(&custom_tools, &[bin_dir]).unwrap();

        // Should find the custom tool
        assert!(tools.iter().any(|t| t.name == "my-custom-tool"));
    }

    #[test]
    fn test_create_shim_content() {
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");

        let tool = PhpTool {
            name: "composer".to_string(),
            original_path: PathBuf::from("/usr/bin/composer"),
            shebang: "#!/usr/bin/php".to_string(),
        };

        let shim_path = create_shim(&tool, &bin_dir).unwrap();

        // Verify shim was created
        assert!(shim_path.exists());
        assert_eq!(shim_path.file_name().unwrap(), "composer");

        // Read shim content
        let content = fs::read_to_string(&shim_path).unwrap();

        // Should contain bash shebang
        assert!(content.starts_with("#!/bin/bash") || content.starts_with("#!/usr/bin/env bash"));

        // Should use the switcher's php
        assert!(content.contains(".php-switcher/bin/php"));

        // Should exec the original tool
        assert!(content.contains("/usr/bin/composer"));

        // Should pass through arguments
        assert!(content.contains("\"$@\""));
    }

    #[test]
    fn test_create_shim_preserves_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");

        let tool = PhpTool {
            name: "phpunit".to_string(),
            original_path: PathBuf::from("/usr/local/bin/phpunit"),
            shebang: "#!/usr/bin/php".to_string(),
        };

        let shim_path = create_shim(&tool, &bin_dir).unwrap();

        // Verify shim is executable
        let metadata = fs::metadata(&shim_path).unwrap();
        let permissions = metadata.permissions();

        // Check executable bit
        assert_ne!(permissions.mode() & 0o111, 0);
    }
}
