use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use php_switcher::{config, detector, switcher};

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

    /// Manage PHP tools (composer, phpunit, etc.)
    Tools {
        #[command(subcommand)]
        tools_command: ToolsCommands,
    },
}

#[derive(Subcommand)]
enum ToolsCommands {
    /// List detected PHP tools and their shim status
    List,

    /// Scan for PHP tools
    Scan,

    /// Enable automatic tool scanning
    Enable,

    /// Disable automatic tool scanning
    Disable,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle shorthand: php-switcher 8.2 -> php-switcher use 8.2
    if let Some(version) = cli.php_version {
        return switcher::switch_version(&version);
    }

    match cli.command {
        Some(Commands::List) | None => list_versions()?,
        Some(Commands::Use { version }) => switcher::switch_version(&version)?,
        Some(Commands::Scan) => scan_installations()?,
        Some(Commands::Info { version }) => show_info(version.as_deref())?,
        Some(Commands::Tools { tools_command }) => match tools_command {
            ToolsCommands::List => tools_list()?,
            ToolsCommands::Scan => tools_scan()?,
            ToolsCommands::Enable => tools_enable()?,
            ToolsCommands::Disable => tools_disable()?,
        },
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

fn tools_list() -> Result<()> {
    let config = config::load_config()?;

    println!("{}", "PHP Tools".bold());
    println!("Scanning: {}\n", if config.tools.scan_for_tools { "enabled".green() } else { "disabled".red() });

    if config.tools.managed.is_empty() {
        println!("{}", "No tools detected yet.".yellow());
        println!("\nTo scan for tools:");
        println!("  1. Enable scanning: php-switcher tools enable");
        println!("  2. Run a scan: php-switcher tools scan");
        return Ok(());
    }

    println!("Detected tools:");
    for tool in &config.tools.managed {
        let shim_status = if tool.shim_created { "✓".green() } else { "○".dimmed() };
        let needs_shim = if tool.shebang.contains("/env") { "(uses env)".dimmed().to_string() } else { "".to_string() };

        println!("  {} {} - {} {}",
            shim_status,
            tool.name.bold(),
            tool.original_path.display().to_string().dimmed(),
            needs_shim
        );
        println!("      Shebang: {}", tool.shebang.dimmed());
    }

    Ok(())
}

fn tools_scan() -> Result<()> {
    let mut config = config::load_config()?;

    if !config.tools.scan_for_tools {
        println!("{}", "Tool scanning is disabled.".yellow());
        println!("Enable it with: php-switcher tools enable");
        return Ok(());
    }

    println!("{}", "Scanning for PHP tools...".bold());

    let tools = detector::find_all_php_tools(&config.tools)?;

    if tools.is_empty() {
        println!("{}", "No PHP tools found.".yellow());
        return Ok(());
    }

    println!("Found {} tool(s)\n", tools.len());

    // Update config with detected tools
    config.tools.managed.clear();
    for tool in &tools {
        config.tools.managed.push(config::ToolEntry {
            name: tool.name.clone(),
            original_path: tool.original_path.clone(),
            shebang: tool.shebang.clone(),
            shim_created: false, // Will be created during next switch
        });

        println!("  {} {}", "✓".green(), tool.name.bold());
        println!("      Path: {}", tool.original_path.display().to_string().dimmed());
        println!("      Shebang: {}", tool.shebang.dimmed());
    }

    config::save_config(&config)?;

    println!("\n{}", "Scan complete!".green());
    println!("Shims will be created automatically on next 'php-switcher use'");

    Ok(())
}

fn tools_enable() -> Result<()> {
    let mut config = config::load_config()?;

    config.tools.scan_for_tools = true;
    config::save_config(&config)?;

    println!("{}", "✓ Tool scanning enabled".green());
    println!("\nNext steps:");
    println!("  1. Run: php-switcher tools scan");
    println!("  2. Switch PHP version to create shims");

    Ok(())
}

fn tools_disable() -> Result<()> {
    let mut config = config::load_config()?;

    config.tools.scan_for_tools = false;
    config::save_config(&config)?;

    println!("{}", "✓ Tool scanning disabled".green());

    Ok(())
}

