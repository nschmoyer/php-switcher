// PHP installation detection module

use crate::version::PhpVersion;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct PhpInstallation {
    pub version: PhpVersion,
    pub path: PathBuf,
}

impl PhpInstallation {
    pub fn new(version: PhpVersion, path: PathBuf) -> Self {
        Self { version, path }
    }
}

/// Get the version from a PHP binary by running it with -v
pub fn get_version_from_binary<P: AsRef<Path>>(binary_path: P) -> Result<PhpVersion> {
    let output = Command::new(binary_path.as_ref())
        .arg("-v")
        .output()
        .map_err(|e| anyhow!("Failed to execute PHP binary: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!("PHP binary returned non-zero exit code"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_php_v_output(&stdout)
}

/// Parse the output of 'php -v' to extract version
pub fn parse_php_v_output(output: &str) -> Result<PhpVersion> {
    PhpVersion::from_php_output(output)
}

/// Check if a binary is a valid PHP executable
pub fn is_valid_php_binary<P: AsRef<Path>>(binary_path: P) -> Result<()> {
    let path = binary_path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(anyhow!("Binary does not exist: {}", path.display()));
    }

    // Try to run it with -v
    let output = Command::new(path)
        .arg("-v")
        .output()
        .map_err(|e| anyhow!("Failed to execute binary: {}", e))?;

    // Check if output contains "PHP"
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains("PHP") {
        return Err(anyhow!("Binary is not PHP"));
    }

    Ok(())
}

/// Detect the currently active PHP installation (from PATH)
pub fn detect_current_php() -> Result<PhpInstallation> {
    let version = get_version_from_binary("php")?;

    // Find the actual path to PHP
    let which_output = Command::new("which")
        .arg("php")
        .output()
        .map_err(|e| anyhow!("Failed to run 'which php': {}", e))?;

    if !which_output.status.success() {
        return Err(anyhow!("Could not find PHP in PATH"));
    }

    let path_str = String::from_utf8_lossy(&which_output.stdout);
    let path = PathBuf::from(path_str.trim());

    Ok(PhpInstallation::new(version, path))
}

/// Scan a directory for PHP binaries
pub fn scan_directory_for_php<P: AsRef<Path>>(dir_path: P) -> Result<Vec<PhpInstallation>> {
    let dir = dir_path.as_ref();
    let mut installations = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Ok(installations);
    }

    // Read directory entries
    let entries = std::fs::read_dir(dir)
        .map_err(|e| anyhow!("Failed to read directory {}: {}", dir.display(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();

        // Only check files (not directories)
        if !path.is_file() {
            continue;
        }

        // Check if filename starts with "php"
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy();
            if filename_str.starts_with("php") {
                // Try to get version from this binary
                if let Ok(version) = get_version_from_binary(&path) {
                    installations.push(PhpInstallation::new(version, path));
                }
            }
        }
    }

    Ok(installations)
}

/// Find all PHP installations on the system
pub fn find_all_php_installations() -> Result<Vec<PhpInstallation>> {
    use std::collections::HashSet;

    let mut installations = Vec::new();
    let mut seen_paths = HashSet::new();

    // Common directories to scan
    let scan_dirs = vec![
        "/usr/bin",
        "/usr/local/bin",
        "/opt/homebrew/bin",
        "/usr/lib",
        "/usr/local/lib",
    ];

    // Also check for Homebrew Cellar directories
    let homebrew_dirs = vec![
        "/usr/local/Cellar",
        "/opt/homebrew/Cellar",
    ];

    // Scan common binary directories
    for dir in scan_dirs {
        if let Ok(found) = scan_directory_for_php(dir) {
            for installation in found {
                // Deduplicate by canonical path
                if let Ok(canonical) = installation.path.canonicalize() {
                    if seen_paths.insert(canonical.clone()) {
                        installations.push(installation);
                    }
                }
            }
        }
    }

    // Scan Homebrew Cellar for php@ versioned formulas
    for homebrew_dir in homebrew_dirs {
        if let Ok(entries) = std::fs::read_dir(homebrew_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("php") {
                        // Look for bin/php in this Cellar directory
                        // Structure is usually: /path/to/Cellar/php@8.2/8.2.12/bin/php
                        if let Ok(version_dirs) = std::fs::read_dir(&path) {
                            for version_dir in version_dirs.flatten() {
                                let bin_dir = version_dir.path().join("bin");
                                if let Ok(found) = scan_directory_for_php(&bin_dir) {
                                    for installation in found {
                                        if let Ok(canonical) = installation.path.canonicalize() {
                                            if seen_paths.insert(canonical.clone()) {
                                                installations.push(installation);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check home directory paths for version managers
    if let Some(home) = dirs::home_dir() {
        // phpbrew
        let phpbrew_dir = home.join(".phpbrew/php");
        if let Ok(entries) = std::fs::read_dir(&phpbrew_dir) {
            for entry in entries.flatten() {
                let bin_dir = entry.path().join("bin");
                if let Ok(found) = scan_directory_for_php(&bin_dir) {
                    for installation in found {
                        if let Ok(canonical) = installation.path.canonicalize() {
                            if seen_paths.insert(canonical.clone()) {
                                installations.push(installation);
                            }
                        }
                    }
                }
            }
        }

        // phpenv
        let phpenv_dir = home.join(".phpenv/versions");
        if let Ok(entries) = std::fs::read_dir(&phpenv_dir) {
            for entry in entries.flatten() {
                let bin_dir = entry.path().join("bin");
                if let Ok(found) = scan_directory_for_php(&bin_dir) {
                    for installation in found {
                        if let Ok(canonical) = installation.path.canonicalize() {
                            if seen_paths.insert(canonical.clone()) {
                                installations.push(installation);
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by version (newest first)
    installations.sort_by(|a, b| b.version.cmp(&a.version));

    Ok(installations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version_from_binary() {
        // This test will run 'php -v' on the system if PHP is installed
        // We'll test the logic, but it may be skipped if PHP isn't available
        let result = get_version_from_binary("php");

        // We can't guarantee PHP is installed in the test environment,
        // so we just test that the function returns a Result
        match result {
            Ok(version) => {
                // If successful, verify it's a valid version
                assert!(version.major > 0);
            }
            Err(_) => {
                // It's okay if PHP isn't installed in test environment
                println!("PHP not found in test environment (this is okay)");
            }
        }
    }

    #[test]
    fn test_parse_php_v_output() {
        let output = "PHP 8.2.12 (cli) (built: Oct 24 2023 12:00:00) (NTS)";
        let result = parse_php_v_output(output);

        assert!(result.is_ok());
        let version = result.unwrap();
        assert_eq!(version.major, 8);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 12);
    }

    #[test]
    fn test_installation_from_path() {
        // Test creating a PhpInstallation
        let version = PhpVersion::new(8, 2, 12);
        let path = PathBuf::from("/usr/bin/php8.2");
        let installation = PhpInstallation {
            version: version.clone(),
            path: path.clone(),
        };

        assert_eq!(installation.version, version);
        assert_eq!(installation.path, path);
    }

    #[test]
    fn test_is_php_binary_valid() {
        // Test with a path that's definitely not PHP
        let result = is_valid_php_binary("/bin/echo");
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_directory_for_php() {
        // Test scanning /usr/bin for PHP binaries
        // This test is system-dependent but should work on most Linux/Mac systems
        let result = scan_directory_for_php("/usr/bin");

        // We can't guarantee what's in /usr/bin, but we can test the function runs
        match result {
            Ok(installations) => {
                // If we found any, verify they're valid
                for installation in installations {
                    assert!(installation.version.major > 0);
                    assert!(installation.path.exists());
                }
            }
            Err(_) => {
                // It's okay if scanning fails in test environment
                println!("Directory scan failed (this is okay in test environment)");
            }
        }
    }

    #[test]
    fn test_find_all_php_installations() {
        // Test finding all PHP installations on the system
        let result = find_all_php_installations();

        // This should always return Ok, even if empty
        assert!(result.is_ok());
        let installations = result.unwrap();

        // Verify no duplicates by path
        let mut paths = std::collections::HashSet::new();
        for installation in &installations {
            assert!(paths.insert(&installation.path));
        }
    }
}
