// Installation hints module
//
// Provides helpful suggestions for installing PHP versions that aren't found on the system.
// Keeps hints deliberately generic to minimize maintenance burden.

use crate::platform::Platform;
use colored::Colorize;

/// Show installation hints for a missing PHP version
pub fn show_installation_hints(version: &str, platform: Platform) {
    println!("\n{}", format!("PHP {} not found on your system.", version).red().bold());
    println!("\n{}", "To install PHP:".bold());

    match platform {
        Platform::Linux => show_linux_hints(version),
        Platform::MacOS => show_macos_hints(version),
        Platform::BSD => show_bsd_hints(version),
        Platform::Other => show_generic_hints(version),
    }

    // Always show the generic PHP.net link
    println!("\n{}", "For detailed installation instructions:".dimmed());
    println!("  {}", "https://www.php.net/manual/en/install.php".cyan());
}

fn show_linux_hints(version: &str) {
    println!("  {} Search your package manager:", "•".green());
    println!("    dnf search php{} php{}", version, version.replace('.', ""));
    println!("    apt search php{}", version);
    println!("    zypper search php{}", version);
    println!();
    println!("  {} Popular third-party repositories:", "•".green());
    println!("    {} {}",
        "Remi (RHEL/Fedora/CentOS):".bold(),
        "https://rpms.remirepo.net/".cyan()
    );
    println!("    {} {}",
        "Ondrej PPA (Ubuntu/Debian):".bold(),
        "https://launchpad.net/~ondrej/+archive/ubuntu/php".cyan()
    );
}

fn show_macos_hints(version: &str) {
    println!("  {} Using Homebrew:", "•".green());
    println!("    brew install php@{}", version);
    println!();
    println!("  {} If formula not found, try:", "•".green());
    println!("    brew tap shivammathur/php");
    println!("    brew install shivammathur/php/php@{}", version);
}

fn show_bsd_hints(version: &str) {
    println!("  {} Using pkg:", "•".green());
    println!("    pkg search php{}", version.replace('.', ""));
    println!("    pkg install php{}", version.replace('.', ""));
    println!();
    println!("  {} Or check your BSD's ports collection", "•".green());
}

fn show_generic_hints(version: &str) {
    println!("  {} Check your system's package manager for PHP {}", "•".green(), version);
    println!("  {} Or download from PHP.net", "•".green());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_show_installation_hints_linux() {
        // This test just verifies the function runs without panicking
        // We can't easily test the output without capturing stdout
        show_installation_hints("8.1", Platform::Linux);
    }

    #[test]
    fn test_show_installation_hints_macos() {
        show_installation_hints("8.2", Platform::MacOS);
    }

    #[test]
    fn test_show_installation_hints_bsd() {
        show_installation_hints("8.3", Platform::BSD);
    }

    #[test]
    fn test_show_installation_hints_other() {
        show_installation_hints("7.4", Platform::Other);
    }

    #[test]
    fn test_hints_with_various_version_formats() {
        // Test with different version string formats
        show_installation_hints("8.1", Platform::Linux);
        show_installation_hints("8.1.0", Platform::Linux);
        show_installation_hints("8", Platform::MacOS);
    }
}
