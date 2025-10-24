// Version switching module

use crate::{config, detector, hints, platform};
use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

/// Switch to a specified PHP version
///
/// This function:
/// 1. Looks for the version in the config cache
/// 2. If not found, automatically scans the system
/// 3. If still not found, shows installation hints
/// 4. Creates symlinks for all related binaries (php, php-cgi, etc.)
pub fn switch_version(version_pattern: &str) -> Result<()> {
    println!("Switching to PHP {}...", version_pattern.bold());

    // Load config
    let mut config = config::load_config()?;

    // Try to find matching version in cache
    let mut paths = config.get_installation_by_version(version_pattern);

    // If not found, auto-scan the system
    if paths.is_none() {
        println!(
            "{}",
            format!("PHP {} not found in cache, scanning system...", version_pattern)
                .yellow()
        );

        let installations = detector::find_all_php_installations()?;

        if installations.is_empty() {
            println!("{}", "No PHP installations found on system.".red());
            let detected_platform = platform::Platform::detect();
            hints::show_installation_hints(version_pattern, detected_platform);
            return Err(anyhow::anyhow!("No PHP installations found"));
        }

        // Update config with newly found installations
        config.update_from_installations(&installations);
        config::save_config(&config)?;

        println!(
            "{} Scan complete, found {} installation(s)",
            "✓".green(),
            installations.len()
        );

        // Try to find the version again
        paths = config.get_installation_by_version(version_pattern);
    }

    // If still not found after scanning, show installation hints
    let paths = match paths {
        Some(p) if !p.is_empty() => p,
        _ => {
            let detected_platform = platform::Platform::detect();
            hints::show_installation_hints(version_pattern, detected_platform);
            return Err(anyhow::anyhow!(
                "PHP {} not found. Please install it and try again.",
                version_pattern
            ));
        }
    };

    // Get primary path for verification
    let primary_path = paths
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("php"))
        .or_else(|| paths.first())
        .ok_or_else(|| anyhow::anyhow!("No primary PHP binary found"))?;

    println!("{} Found PHP at: {}", "✓".green(), primary_path.display());
    println!("  {} related binaries to symlink", paths.len());

    // Create symlinks for all related binaries
    let bin_dir = get_bin_dir()?;
    let symlink_count = create_symlinks(&paths, &bin_dir)?;

    // Verify the switch using the primary binary
    verify_switch(&bin_dir)?;

    // Create shims for PHP tools if scanning is enabled
    let shim_count = if config.tools.scan_for_tools && !config.tools.managed.is_empty() {
        println!("\n{}", "Creating tool shims...".dimmed());

        let tools: Vec<crate::tools::PhpTool> = config.tools.managed.iter().map(|entry| {
            crate::tools::PhpTool {
                name: entry.name.clone(),
                original_path: entry.original_path.clone(),
                shebang: entry.shebang.clone(),
            }
        }).collect();

        let count = create_shims_for_tools(&tools, &bin_dir)?;

        if count > 0 {
            for tool in &tools {
                if crate::tools::needs_shim(&tool.shebang) {
                    println!("  {} {} → uses switched PHP", "✓".green(), tool.name.dimmed());
                }
            }
        }

        // Update config to mark shims as created
        for entry in &mut config.tools.managed {
            entry.shim_created = crate::tools::needs_shim(&entry.shebang);
        }
        config::save_config(&config)?;

        count
    } else {
        0
    };

    // Show success message
    println!("\n{}", "PHP version switched successfully!".green().bold());
    println!("  {} PHP symlinks created", symlink_count);
    if shim_count > 0 {
        println!("  {} tool shims created", shim_count);
    } else if !config.tools.scan_for_tools {
        let tip = "💡 Tip: Enable tool scanning to auto-shim composer, phpunit, etc.";
        let cmd = "   Run: php-switcher tools enable && php-switcher tools scan";
        println!("\n{}", tip.dimmed());
        println!("{}", cmd.dimmed());
    }

    show_path_instructions(&bin_dir);

    Ok(())
}

/// Create symlinks for all PHP binaries in the target directory
fn create_symlinks(source_paths: &[PathBuf], bin_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(bin_dir)?;

    let mut symlink_count = 0;

    // Find the primary PHP binary (the one named "php" or the first one)
    let primary_path = source_paths
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("php"))
        .or_else(|| source_paths.first())
        .ok_or_else(|| anyhow::anyhow!("No PHP binary found"))?;

    // Always create a standard "php" symlink to the primary binary
    let php_symlink = bin_dir.join("php");
    if php_symlink.exists() || php_symlink.symlink_metadata().is_ok() {
        std::fs::remove_file(&php_symlink).ok();
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(primary_path, &php_symlink)?;
    }

    symlink_count += 1;
    println!(
        "  {} {} → {}",
        "✓".green(),
        "php".dimmed(),
        primary_path.display().to_string().dimmed()
    );

    // Create symlinks for related binaries (php-cgi, php-fpm, etc.)
    for path in source_paths {
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy();

            // Skip the primary binary if it's already named "php"
            if filename_str == "php" {
                continue;
            }

            // For versioned binaries like "php81", "php81-cgi", create symlinks with standard names
            // e.g., php81 -> skip (primary already handled), php81-cgi -> php-cgi
            let standardized_name = if filename_str.starts_with("php") {
                // Remove version numbers from the name (e.g., php81-cgi -> php-cgi)
                let without_prefix = &filename_str[3..]; // Skip "php"
                let rest = without_prefix.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');

                // If only a version number (like "php81"), skip it since we already handled primary
                if rest.is_empty() || rest == "php" {
                    continue;
                }

                // Reconstruct: php + rest (e.g., "-cgi" -> "php-cgi")
                format!("php{}", rest)
            } else {
                filename_str.to_string()
            };

            let symlink_path = bin_dir.join(&standardized_name);

            // Remove existing symlink if it exists
            if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
                std::fs::remove_file(&symlink_path).ok();
            }

            // Create symlink
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(path, &symlink_path)?;
            }

            symlink_count += 1;
            println!(
                "  {} {} → {}",
                "✓".green(),
                standardized_name.dimmed(),
                path.display().to_string().dimmed()
            );
        }
    }

    Ok(symlink_count)
}

/// Verify that the switch was successful by checking the primary PHP binary
fn verify_switch(bin_dir: &Path) -> Result<()> {
    let primary_symlink = bin_dir.join("php");
    if primary_symlink.exists() {
        if let Ok(version) = detector::get_version_from_binary(&primary_symlink) {
            println!("\n{} Verified: {}", "✓".green(), version.to_string().bold());
        }
    }
    Ok(())
}

/// Get the bin directory where symlinks will be created
fn get_bin_dir() -> Result<PathBuf> {
    let switcher_dir = config::get_config_dir()?;
    Ok(switcher_dir.join("bin"))
}

/// Show instructions for adding the bin directory to PATH
fn show_path_instructions(bin_dir: &Path) {
    println!(
        "\n{}",
        "IMPORTANT: Ensure the switcher bin directory is first in your PATH:".yellow()
    );
    println!("  export PATH=\"{}:$PATH\"", bin_dir.display());
    println!("\nAdd this to your ~/.bashrc or ~/.zshrc and run: source ~/.bashrc");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bin_dir() {
        let bin_dir = get_bin_dir();
        assert!(bin_dir.is_ok());

        let path = bin_dir.unwrap();
        assert!(path.to_string_lossy().contains(".php-switcher"));
        assert!(path.to_string_lossy().ends_with("bin"));
    }

    #[test]
    fn test_create_symlinks_with_empty_paths() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");

        // Empty paths should return an error (no PHP binary found)
        let result = create_symlinks(&[], &bin_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_symlinks_with_versioned_binary() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let bin_dir = temp_dir.path().join("bin");
        std::fs::create_dir_all(&source_dir).unwrap();

        // Create a fake php81 binary
        let php81_path = source_dir.join("php81");
        std::fs::write(&php81_path, "#!/bin/bash\necho fake php").unwrap();

        // Create symlinks
        let paths = vec![php81_path.clone()];
        let result = create_symlinks(&paths, &bin_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Should create 1 symlink (php -> php81)

        // Verify the "php" symlink was created and points to php81
        let php_symlink = bin_dir.join("php");
        assert!(php_symlink.exists());
        assert!(php_symlink.symlink_metadata().unwrap().is_symlink());
    }

    #[test]
    fn test_create_symlinks_with_related_binaries() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let bin_dir = temp_dir.path().join("bin");
        std::fs::create_dir_all(&source_dir).unwrap();

        // Create fake binaries
        let php81_path = source_dir.join("php81");
        let php81_cgi_path = source_dir.join("php81-cgi");
        std::fs::write(&php81_path, "#!/bin/bash\necho fake php").unwrap();
        std::fs::write(&php81_cgi_path, "#!/bin/bash\necho fake php-cgi").unwrap();

        // Create symlinks
        let paths = vec![php81_path.clone(), php81_cgi_path.clone()];
        let result = create_symlinks(&paths, &bin_dir);
        assert!(result.is_ok());
        // Should create 2 symlinks: php -> php81, php-cgi -> php81-cgi
        assert_eq!(result.unwrap(), 2);

        // Verify symlinks
        let php_symlink = bin_dir.join("php");
        let php_cgi_symlink = bin_dir.join("php-cgi");
        assert!(php_symlink.exists());
        assert!(php_cgi_symlink.exists());
    }

    #[test]
    fn test_verify_switch_with_nonexistent_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("nonexistent");

        // Should not error even if directory doesn't exist
        let result = verify_switch(&bin_dir);
        assert!(result.is_ok());
    }

    // Tool shim creation tests
    #[test]
    fn test_create_shims_for_tools() {
        use crate::tools::PhpTool;
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");

        let tools = vec![
            PhpTool {
                name: "composer".to_string(),
                original_path: PathBuf::from("/usr/bin/composer"),
                shebang: "#!/usr/bin/php".to_string(),
            },
            PhpTool {
                name: "phpunit".to_string(),
                original_path: PathBuf::from("/usr/bin/phpunit"),
                shebang: "#!/usr/bin/php".to_string(),
            },
        ];

        let result = create_shims_for_tools(&tools, &bin_dir);

        assert!(result.is_ok());
        let created = result.unwrap();

        // Should have created 2 shims
        assert_eq!(created, 2);

        // Verify shims exist
        assert!(bin_dir.join("composer").exists());
        assert!(bin_dir.join("phpunit").exists());
    }

    #[test]
    fn test_skip_shim_for_env_tools() {
        use crate::tools::PhpTool;
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");

        let tools = vec![
            PhpTool {
                name: "phpunit".to_string(),
                original_path: PathBuf::from("/usr/bin/phpunit"),
                shebang: "#!/usr/bin/env php".to_string(), // Uses env - no shim needed
            },
        ];

        let result = create_shims_for_tools(&tools, &bin_dir);

        assert!(result.is_ok());
        let created = result.unwrap();

        // Should not create shim for tools with env shebang
        assert_eq!(created, 0);
        assert!(!bin_dir.join("phpunit").exists());
    }

    #[test]
    fn test_update_shims_on_rescan() {
        use crate::tools::PhpTool;
        use std::fs;
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        // Create an old shim
        fs::write(bin_dir.join("composer"), "#!/bin/bash\necho 'old shim'").unwrap();

        let tools = vec![
            PhpTool {
                name: "composer".to_string(),
                original_path: PathBuf::from("/usr/bin/composer"),
                shebang: "#!/usr/bin/php".to_string(),
            },
        ];

        let result = create_shims_for_tools(&tools, &bin_dir);

        assert!(result.is_ok());

        // Verify shim was updated (should contain new content)
        let content = fs::read_to_string(bin_dir.join("composer")).unwrap();
        assert!(content.contains(".php-switcher/bin/php"));
        assert!(!content.contains("old shim"));
    }
}

/// Create shims for PHP tools that need them
pub fn create_shims_for_tools<P: AsRef<Path>>(tools: &[crate::tools::PhpTool], bin_dir: P) -> Result<usize> {
    use crate::tools;

    let mut created = 0;

    for tool in tools {
        // Only create shims for tools with hardcoded PHP paths
        if tools::needs_shim(&tool.shebang) {
            tools::create_shim(tool, bin_dir.as_ref())?;
            created += 1;
        }
    }

    Ok(created)
}
