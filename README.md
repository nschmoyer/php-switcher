# php-switcher

A fast, simple CLI tool for switching between PHP versions on Linux and macOS.

## Features

- **Automatic Detection**: Scans your system for installed PHP versions
- **Easy Switching**: Switch between PHP versions with a simple command
- **Version Matching**: Supports fuzzy version matching (e.g., `8.2` matches `8.2.12`)
- **Configuration Cache**: Stores discovered PHP installations for quick access
- **Multiple Installation Methods**: Supports system packages, Homebrew, phpbrew, phpenv, and more

## Installation

### From Source

```bash
git clone https://github.com/nschmoyer/php-switcher
cd php-switcher
cargo build --release
sudo cp target/release/php-switcher /usr/local/bin/
```

### One-time Setup

**IMPORTANT:** Add the php-switcher bin directory to the **beginning** of your PATH by adding this to your `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$HOME/.php-switcher/bin:$PATH"
```

This ensures that `~/.php-switcher/bin/php` is found before system PHP installations.

Then reload your shell:

```bash
source ~/.bashrc  # or source ~/.zshrc
```

## Usage

### List Available PHP Versions

```bash
# List all detected PHP versions
php-switcher
php-switcher list
```

### Switch PHP Version

```bash
# Switch to PHP 8.2 (fuzzy matching)
php-switcher use 8.2
php-switcher 8.2  # Shorthand

# Switch to exact version
php-switcher use 8.2.12
```

### Scan for PHP Installations

```bash
# Scan system for PHP installations
php-switcher scan
```

### Show Information

```bash
# Show general info
php-switcher info

# Show info for specific version
php-switcher info 8.2
```

### PHP Tool Management

**Problem**: Tools like `composer` often have hardcoded shebangs like `#!/usr/bin/php`, which bypass your PATH and ignore php-switcher.

**Solution**: php-switcher can automatically detect these tools and create shims that use the switched PHP version.

```bash
# Enable tool scanning (opt-in)
php-switcher tools enable

# Scan for PHP tools (composer, phpunit, etc.)
php-switcher tools scan

# List detected tools and their status
php-switcher tools list

# Disable tool scanning
php-switcher tools disable
```

**How it works**:
1. Scans your PATH for common PHP tools (composer, phpunit, psalm, phpstan, rector, etc.)
2. Checks each tool's shebang line
3. For tools with `#!/usr/bin/php` (hardcoded), creates a shim in `~/.php-switcher/bin/`
4. Skips tools with `#!/usr/bin/env php` (already PATH-aware)
5. Shims automatically use the currently switched PHP version

## Behind the Scenes

1. **Detection**: php-switcher scans common locations for PHP binaries:
   - `/usr/bin`, `/usr/local/bin` (system installations)
   - Homebrew Cellar directories (macOS)
   - phpbrew (`~/.phpbrew/php`)
   - phpenv (`~/.phpenv/versions`)

2. **Configuration**: Discovered versions are cached in `~/.php-switcher/config.toml`

3. **Switching**: Creates a symlink at `~/.php-switcher/bin/php` pointing to the selected version

4. **Activation**: You add `~/.php-switcher/bin` to your PATH once, then switching is instant

## Supported Platforms

- **Linux**: Tested on Fedora 42