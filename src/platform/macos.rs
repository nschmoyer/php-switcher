// macOS-specific implementation

pub fn get_common_php_paths() -> Vec<&'static str> {
    vec![
        "/usr/bin/php",
        "/usr/local/bin/php",
        "/opt/homebrew/bin/php",
    ]
}

pub fn get_scan_patterns() -> Vec<&'static str> {
    vec![
        "/usr/local/Cellar/php*",
        "/opt/homebrew/Cellar/php*",
        "/usr/local/bin/php*",
        "/opt/homebrew/bin/php*",
    ]
}
