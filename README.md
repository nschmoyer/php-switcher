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
git clone https://github.com/yourusername/php-switcher
cd php-switcher
cargo build --release
sudo cp target/release/php-switcher /usr/local/bin/
```

### One-time Setup

Add the php-switcher bin directory to your PATH by adding this to your `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$HOME/.php-switcher/bin:$PATH"
```

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

Output:
```
Current PHP version: 8.4.13

Available PHP versions:
  ● 8.4.13  /usr/bin/php  [ACTIVE]
  ○ 8.2.12  /usr/bin/php8.2
  ○ 7.4.33  /usr/bin/php7.4

Use 'php-switcher use <version>' to switch versions
```

### Switch PHP Version

```bash
# Switch to PHP 8.2 (fuzzy matching)
php-switcher use 8.2
php-switcher 8.2  # Shorthand

# Switch to exact version
php-switcher use 8.2.12
```

Output:
```
Switching to PHP 8.2...
✓ Found PHP at: /usr/bin/php8.2
✓ Updated symlink: /home/user/.php-switcher/bin/php → /usr/bin/php8.2
✓ Verified: 8.2.12

PHP version switched successfully!

Add /home/user/.php-switcher/bin to your PATH to use the new version:
  export PATH="/home/user/.php-switcher/bin:$PATH"
```

### Scan for PHP Installations

```bash
# Scan system for PHP installations
php-switcher scan
```

Output:
```
Scanning for PHP installations...
✓ Found 3 PHP installation(s)

  8.4.13 at /usr/bin/php
  8.2.12 at /usr/bin/php8.2
  7.4.33 at /usr/bin/php7.4

Configuration updated.
```

### Show Information

```bash
# Show general info
php-switcher info

# Show info for specific version
php-switcher info 8.2
```

## How It Works

1. **Detection**: php-switcher scans common locations for PHP binaries:
   - `/usr/bin`, `/usr/local/bin` (system installations)
   - Homebrew Cellar directories (macOS)
   - phpbrew (`~/.phpbrew/php`)
   - phpenv (`~/.phpenv/versions`)

2. **Configuration**: Discovered versions are cached in `~/.php-switcher/config.toml`

3. **Switching**: Creates a symlink at `~/.php-switcher/bin/php` pointing to the selected version

4. **Activation**: You add `~/.php-switcher/bin` to your PATH once, then switching is instant

## Supported Platforms

- **Linux**: Tested on Fedora, Ubuntu, Debian
- **macOS**: Tested on Intel and Apple Silicon

## Development

This project was built using Test-Driven Development (TDD) with Rust.

### Run Tests

```bash
cargo test
```

### Build

```bash
cargo build --release
```

### Project Structure

```
src/
├── lib.rs          # Library entry point
├── main.rs         # CLI entry point
├── version.rs      # Version parsing and comparison
├── detector.rs     # PHP installation detection
├── config.rs       # Configuration management
├── switcher.rs     # Version switching logic
└── platform/       # Platform-specific code
    ├── mod.rs
    ├── linux.rs
    └── macos.rs
```

## Configuration

Configuration is stored in `~/.php-switcher/config.toml`:

```toml
[settings]
last_scan = "2025-10-23T12:00:00Z"
default_version = "8.2"

[[versions]]
version = "8.4.13"
path = "/usr/bin/php"
source = "auto"

[[versions]]
version = "8.2.12"
path = "/usr/bin/php8.2"
source = "auto"
```

## Future Enhancements

- [ ] Per-project PHP version files (`.php-version`)
- [ ] Auto-installation of PHP versions
- [ ] Shell prompt integration
- [ ] Extension management
- [ ] Windows support

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
