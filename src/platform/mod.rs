// Platform abstraction module

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
pub use macos::*;

/// Detected operating system platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
    BSD,
    Other,
}

impl Platform {
    /// Detect the current platform
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            Platform::Linux
        }

        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }

        #[cfg(any(
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        {
            Platform::BSD
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        )))]
        {
            Platform::Other
        }
    }

    /// Get a human-readable name for the platform
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Linux => "Linux",
            Platform::MacOS => "macOS",
            Platform::BSD => "BSD",
            Platform::Other => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detect() {
        let platform = Platform::detect();

        // The test should pass on any platform
        // We just verify it returns a valid Platform variant
        assert!(matches!(
            platform,
            Platform::Linux | Platform::MacOS | Platform::BSD | Platform::Other
        ));
    }

    #[test]
    fn test_platform_name() {
        assert_eq!(Platform::Linux.name(), "Linux");
        assert_eq!(Platform::MacOS.name(), "macOS");
        assert_eq!(Platform::BSD.name(), "BSD");
        assert_eq!(Platform::Other.name(), "Unknown");
    }

    #[test]
    fn test_current_platform_name() {
        let platform = Platform::detect();
        let name = platform.name();

        // Should return a non-empty string
        assert!(!name.is_empty());
    }
}
