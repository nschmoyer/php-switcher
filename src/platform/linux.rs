// Linux-specific implementation

pub fn get_common_php_paths() -> Vec<&'static str> {
    vec![
        "/usr/bin/php",
        "/usr/local/bin/php",
        "/opt/php",
    ]
}

pub fn get_scan_patterns() -> Vec<&'static str> {
    vec![
        "/usr/bin/php*",
        "/usr/local/bin/php*",
        "/usr/lib/php*",
        "/opt/php*",
    ]
}
