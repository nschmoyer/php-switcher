use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use php_switcher::{config, detector};

#[derive(Parser)]
#[command(name = "php-switcher")]
#[command(about = "Easy PHP version switching", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Version to switch to (shorthand for 'use')
    #[arg(value_name = "VERSION")]
    php_version: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available PHP versions
    List,

    /// Switch to a specific PHP version
    Use { version: String },

    /// Scan for PHP installations
    Scan,

    /// Show information about PHP installations
    Info { version: Option<String> },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle shorthand: php-switcher 8.2 -> php-switcher use 8.2
    if let Some(version) = cli.php_version {
        return switch_version(&version);
    }

    match cli.command {
        Some(Commands::List) | None => list_versions()?,
        Some(Commands::Use { version }) => switch_version(&version)?,
        Some(Commands::Scan) => scan_installations()?,
        Some(Commands::Info { version }) => show_info(version.as_deref())?,
    }

    Ok(())
}

fn list_versions() -> Result<()> {
    // Try to detect current PHP
    let current = detector::detect_current_php().ok();

    if let Some(ref current_php) = current {
        println!(
            "{} {}\n",
            "Current PHP version:".bold(),
            current_php.version.to_string().green()
        );
    }

    // Load config to get cached installations
    let mut config = config::load_config()?;

    // If config is empty, scan for installations
    if config.versions.is_empty() {
        println!("{}", "Scanning for PHP installations...".yellow());
        let installations = detector::find_all_php_installations()?;
        config.update_from_installations(&installations);
        config::save_config(&config)?;
    }

    if config.versions.is_empty() {
        println!("{}", "No PHP installations found.".red());
        println!("\nYou can:");
        println!("  - Install PHP using your package manager");
        println!("  - Run 'php-switcher scan' to re-scan");
        return Ok(());
    }

    println!("{}", "Available PHP versions:".bold());

    for entry in &config.versions {
        let is_current = current
            .as_ref()
            .map(|c| c.version.to_string() == entry.version)
            .unwrap_or(false);

        // Get the primary path (prefer 'php' binary)
        let primary_path = entry
            .paths
            .iter()
            .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("php"))
            .or_else(|| entry.paths.first());

        if is_current {
            println!(
                "  {} {}  {}  {}",
                "●".green(),
                entry.version.green().bold(),
                primary_path
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
                    .dimmed(),
                "[ACTIVE]".green().bold()
            );
        } else {
            println!(
                "  {} {}  {}",
                "○".dimmed(),
                entry.version,
                primary_path
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
                    .dimmed()
            );
        }

        // Show related binaries if more than just 'php'
        if entry.paths.len() > 1 {
            let related: Vec<String> = entry
                .paths
                .iter()
                .filter(|p| p.file_name().and_then(|n| n.to_str()) != Some("php"))
                .filter_map(|p| p.file_name()?.to_str().map(String::from))
                .collect();

            if !related.is_empty() {
                println!(
                    "      {} {}",
                    "Related:".dimmed(),
                    related.join(", ").dimmed()
                );
            }
        }
    }

    println!("\n{}", "Use 'php-switcher use <version>' to switch versions".dimmed());

    Ok(())
}

fn switch_version(version_pattern: &str) -> Result<()> {
    println!("Switching to PHP {}...", version_pattern.bold());

    // Load config
    let config = config::load_config()?;

    // Find matching version (returns all paths)
    let paths = config
        .get_installation_by_version(version_pattern)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No PHP installation found matching '{}'\nRun 'php-switcher list' to see available versions",
                version_pattern
            )
        })?;

    if paths.is_empty() {
        return Err(anyhow::anyhow!("No PHP binaries found for version {}", version_pattern));
    }

    // Get primary path for verification
    let primary_path = paths
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("php"))
        .or_else(|| paths.first())
        .ok_or_else(|| anyhow::anyhow!("No primary PHP binary found"))?;

    println!("{} Found PHP at: {}", "✓".green(), primary_path.display());
    println!("  {} related binaries to symlink", paths.len());

    // Create symlink directory
    let switcher_dir = config::get_config_dir()?;
    let bin_dir = switcher_dir.join("bin");
    std::fs::create_dir_all(&bin_dir)?;

    // Create symlinks for all related binaries
    let mut symlink_count = 0;
    for path in &paths {
        if let Some(filename) = path.file_name() {
            let symlink_path = bin_dir.join(filename);

            // Remove existing symlink if it exists
            if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
                std::fs::remove_file(&symlink_path).ok(); // Ignore errors
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
                filename.to_string_lossy().dimmed(),
                path.display().to_string().dimmed()
            );
        }
    }

    // Verify the switch using the primary binary
    let primary_symlink = bin_dir.join("php");
    if primary_symlink.exists() {
        if let Ok(version) = detector::get_version_from_binary(&primary_symlink) {
            println!("\n{} Verified: {}", "✓".green(), version.to_string().bold());
        }
    }

    println!("\n{}", "PHP version switched successfully!".green().bold());
    println!("  {} symlinks created", symlink_count);

    println!(
        "\n{}",
        format!(
            "Add {} to your PATH to use the new version:",
            bin_dir.display()
        )
        .yellow()
    );
    println!("  export PATH=\"{}:$PATH\"", bin_dir.display());
    println!("\nOr add this to your ~/.bashrc or ~/.zshrc");

    Ok(())
}

fn scan_installations() -> Result<()> {
    println!("{}", "Scanning for PHP installations...".yellow());

    let installations = detector::find_all_php_installations()?;

    if installations.is_empty() {
        println!("{}", "No PHP installations found.".red());
        return Ok(());
    }

    println!(
        "{} Found {} PHP installation(s)\n",
        "✓".green(),
        installations.len()
    );

    for installation in &installations {
        // Get the primary path
        let primary_path = installation.primary_path();

        println!(
            "  {} at {}",
            installation.version.to_string().bold(),
            primary_path.map(|p| p.display().to_string()).unwrap_or_default()
        );

        // Show related binaries
        if installation.paths.len() > 1 {
            let related: Vec<String> = installation
                .paths
                .iter()
                .filter(|p| Some(*p) != primary_path)
                .filter_map(|p| p.file_name()?.to_str().map(String::from))
                .collect();

            if !related.is_empty() {
                println!(
                    "      {} {}",
                    "Related:".dimmed(),
                    related.join(", ").dimmed()
                );
            }
        }
    }

    // Save to config
    let mut config = config::load_config()?;
    config.update_from_installations(&installations);
    config::save_config(&config)?;

    println!("\n{}", "Configuration updated.".green());

    Ok(())
}

fn show_info(version: Option<&str>) -> Result<()> {
    if let Some(version_pattern) = version {
        // Show info for specific version
        let config = config::load_config()?;
        let paths = config
            .get_installation_by_version(version_pattern)
            .ok_or_else(|| anyhow::anyhow!("No PHP installation found matching '{}'", version_pattern))?;

        let primary_path = config
            .get_primary_path_by_version(version_pattern)
            .ok_or_else(|| anyhow::anyhow!("No primary PHP binary found"))?;

        if let Ok(version) = detector::get_version_from_binary(&primary_path) {
            println!("{}", "PHP Installation Info".bold());
            println!("  Version: {}", version.to_string().bold());
            println!("  Short version: {}", version.short_version());
            println!("  Primary path: {}", primary_path.display());

            // Show all binaries
            println!("\n  {} binaries:", paths.len());
            for path in &paths {
                if let Some(filename) = path.file_name() {
                    println!("    - {} ({})", filename.to_string_lossy(), path.display());
                }
            }
        }
    } else {
        // Show general info
        println!("{}", "php-switcher".bold());
        println!("Version: {}", env!("CARGO_PKG_VERSION"));

        let config_path = config::get_config_path()?;
        println!("\nConfiguration:");
        println!("  Config file: {}", config_path.display());

        let config = config::load_config()?;
        println!("  Tracked versions: {}", config.versions.len());

        if let Some(last_scan) = config.settings.last_scan {
            println!("  Last scan: {}", last_scan);
        }
    }

    Ok(())
}

