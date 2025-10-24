// Version parsing and comparison module

use anyhow::{anyhow, Result};
use regex::Regex;
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhpVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PhpVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn from_php_output(output: &str) -> Result<Self> {
        // Regex to match PHP version like "PHP 8.2.12" or "PHP 8.4.0-dev"
        let re = Regex::new(r"PHP\s+(\d+)\.(\d+)\.(\d+)").unwrap();

        if let Some(captures) = re.captures(output) {
            let major = captures[1].parse::<u32>()
                .map_err(|_| anyhow!("Invalid major version"))?;
            let minor = captures[2].parse::<u32>()
                .map_err(|_| anyhow!("Invalid minor version"))?;
            let patch = captures[3].parse::<u32>()
                .map_err(|_| anyhow!("Invalid patch version"))?;

            Ok(Self::new(major, minor, patch))
        } else {
            Err(anyhow!("Could not parse PHP version from output"))
        }
    }

    pub fn matches(&self, pattern: &str) -> bool {
        let parts: Vec<&str> = pattern.split('.').collect();

        match parts.len() {
            1 => {
                // Match major version only (e.g., "8")
                if let Ok(major) = parts[0].parse::<u32>() {
                    self.major == major
                } else {
                    false
                }
            }
            2 => {
                // Match major.minor (e.g., "8.2")
                if let (Ok(major), Ok(minor)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<u32>(),
                ) {
                    self.major == major && self.minor == minor
                } else {
                    false
                }
            }
            3 => {
                // Match major.minor.patch (e.g., "8.2.12")
                if let (Ok(major), Ok(minor), Ok(patch)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<u32>(),
                ) {
                    self.major == major && self.minor == minor && self.patch == patch
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn short_version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

impl fmt::Display for PhpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for PhpVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PhpVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                other => other,
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_php_version_standard() {
        let output = "PHP 8.2.12 (cli) (built: Oct 24 2023 12:00:00) (NTS)";
        let version = PhpVersion::from_php_output(output);

        assert!(version.is_ok());
        let version = version.unwrap();
        assert_eq!(version.major, 8);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 12);
        assert_eq!(version.to_string(), "8.2.12");
    }

    #[test]
    fn test_parse_php_version_simple() {
        let output = "PHP 7.4.33";
        let version = PhpVersion::from_php_output(output);

        assert!(version.is_ok());
        let version = version.unwrap();
        assert_eq!(version.major, 7);
        assert_eq!(version.minor, 4);
        assert_eq!(version.patch, 33);
    }

    #[test]
    fn test_parse_php_version_with_suffix() {
        let output = "PHP 8.4.0-dev (cli) (built: Oct 23 2025 10:00:00) (NTS)";
        let version = PhpVersion::from_php_output(output);

        assert!(version.is_ok());
        let version = version.unwrap();
        assert_eq!(version.major, 8);
        assert_eq!(version.minor, 4);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_parse_invalid_version() {
        let output = "Not a PHP version";
        let version = PhpVersion::from_php_output(output);
        assert!(version.is_err());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = PhpVersion::new(8, 2, 12);
        let v2 = PhpVersion::new(8, 2, 11);
        let v3 = PhpVersion::new(8, 3, 0);

        assert!(v1 > v2);
        assert!(v3 > v1);
        assert_eq!(v1, PhpVersion::new(8, 2, 12));
    }

    #[test]
    fn test_partial_match() {
        let version = PhpVersion::new(8, 2, 12);

        assert!(version.matches("8.2.12"));
        assert!(version.matches("8.2"));
        assert!(version.matches("8"));
        assert!(!version.matches("8.3"));
        assert!(!version.matches("7"));
    }

    #[test]
    fn test_short_version_string() {
        let version = PhpVersion::new(8, 2, 12);
        assert_eq!(version.short_version(), "8.2");
    }
}
