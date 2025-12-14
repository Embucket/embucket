//! Build-time information for Embucket binaries.
//!
//! This crate provides access to version and git metadata captured at build time.
//! All information is embedded at compile time via environment variables set by build.rs.

/// Build information for Embucket binaries.
pub struct BuildInfo;

impl BuildInfo {
    /// Version from Cargo.toml (e.g., "0.1.0")
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    /// Full git commit hash (e.g., "7b92aa2347...")
    pub const GIT_SHA: &'static str = env!("GIT_SHA");

    /// Short git commit hash (e.g., "7b92aa23")
    pub const GIT_SHA_SHORT: &'static str = env!("GIT_SHA_SHORT");

    /// Git branch name (e.g., "main")
    pub const GIT_BRANCH: &'static str = env!("GIT_BRANCH");

    /// Git describe output - semantic version from tags
    /// Format examples:
    /// - "v0.1.0" - on a tag
    /// - "v0.1.0-5-g7b92aa23" - 5 commits after tag v0.1.0
    /// - "v0.1.0-dirty" - on a tag with uncommitted changes
    /// - "7b92aa23" - no tags exist, just the commit hash
    pub const GIT_DESCRIBE: &'static str = env!("GIT_DESCRIBE");

    /// Whether the repository had uncommitted changes ("true" or "false")
    pub const GIT_DIRTY: &'static str = env!("GIT_DIRTY");

    /// Build timestamp in RFC 3339 format
    pub const BUILD_TIMESTAMP: &'static str = env!("BUILD_TIMESTAMP");

    /// Returns a formatted version string with git metadata.
    ///
    /// Format: "0.1.0 (7b92aa23) on main built 2025-12-13"
    /// If dirty: "0.1.0 (7b92aa23-dirty) on main built 2025-12-13"
    #[must_use]
    pub fn full_version() -> String {
        let dirty_suffix = if Self::GIT_DIRTY == "true" {
            "-dirty"
        } else {
            ""
        };
        format!(
            "{} ({}{}) on {} built {}",
            Self::VERSION,
            Self::GIT_SHA_SHORT,
            dirty_suffix,
            Self::GIT_BRANCH,
            Self::BUILD_TIMESTAMP
        )
    }

    /// Returns true if the repository had uncommitted changes at build time.
    #[must_use]
    pub fn is_dirty() -> bool {
        Self::GIT_DIRTY == "true"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_info_constants() {
        // These should all be non-empty (either real values or "unknown")
        assert!(!BuildInfo::VERSION.is_empty());
        assert!(!BuildInfo::GIT_SHA.is_empty());
        assert!(!BuildInfo::GIT_SHA_SHORT.is_empty());
        assert!(!BuildInfo::GIT_BRANCH.is_empty());
        assert!(!BuildInfo::GIT_DIRTY.is_empty());
        assert!(!BuildInfo::BUILD_TIMESTAMP.is_empty());
    }

    #[test]
    fn test_full_version() {
        let version = BuildInfo::full_version();
        // Should contain at least the version number
        assert!(version.contains(BuildInfo::VERSION));
    }

    #[test]
    fn test_is_dirty() {
        // Should return a boolean without panicking
        let _ = BuildInfo::is_dirty();
    }
}
