//! Version and build information for gldf-rs
//!
//! This module provides version information and hash computation utilities
//! for tracking builds and deployments.
//!
//! # Version Check API
//!
//! The version module provides a mechanism for local applications to verify
//! their build matches the deployed version:
//!
//! ```rust,ignore
//! use gldf_rs::version::{BuildVersion, VersionStatus};
//!
//! // In WASM, fetch version.json from server
//! let server_version = BuildVersion::from_json(&fetched_json)?;
//! let local_version = BuildVersion::embedded();
//!
//! match local_version.compare(&server_version) {
//!     VersionStatus::Current => println!("Up to date"),
//!     VersionStatus::Outdated { .. } => println!("Update available"),
//!     VersionStatus::Newer { .. } => println!("Local build is newer"),
//!     VersionStatus::Unknown => println!("Cannot determine"),
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Version information for a build
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildVersion {
    /// UTC timestamp of the build
    pub build_time: String,
    /// Crate version from Cargo.toml
    pub version: String,
    /// Git commit hash (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_hash: Option<String>,
    /// Component hashes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<ComponentHashes>,
}

/// Hash information for build components
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentHashes {
    /// Leptos editor hashes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leptos: Option<FileHashes>,
    /// Bevy 3D viewer hashes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bevy: Option<FileHashes>,
}

/// Hash information for a component's files
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileHashes {
    /// Hash of the JavaScript file
    pub js_hash: String,
    /// Hash of the WASM file
    pub wasm_hash: String,
}

impl BuildVersion {
    /// Create a new BuildVersion with current timestamp
    pub fn new() -> Self {
        Self {
            build_time: chrono_lite_now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_hash: None,
            components: None,
        }
    }

    /// Create from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Set component hashes
    pub fn with_components(mut self, components: ComponentHashes) -> Self {
        self.components = Some(components);
        self
    }

    /// Set git hash
    pub fn with_git_hash(mut self, hash: impl Into<String>) -> Self {
        self.git_hash = Some(hash.into());
        self
    }
}

impl Default for BuildVersion {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentHashes {
    /// Create new ComponentHashes
    pub fn new() -> Self {
        Self {
            leptos: None,
            bevy: None,
        }
    }

    /// Set Leptos hashes
    pub fn with_leptos(mut self, js_hash: impl Into<String>, wasm_hash: impl Into<String>) -> Self {
        self.leptos = Some(FileHashes {
            js_hash: js_hash.into(),
            wasm_hash: wasm_hash.into(),
        });
        self
    }

    /// Set Bevy hashes
    pub fn with_bevy(mut self, js_hash: impl Into<String>, wasm_hash: impl Into<String>) -> Self {
        self.bevy = Some(FileHashes {
            js_hash: js_hash.into(),
            wasm_hash: wasm_hash.into(),
        });
        self
    }
}

impl Default for ComponentHashes {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of comparing two versions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionStatus {
    /// Local version matches server version exactly
    Current,
    /// Local version is older than server version
    Outdated {
        local_time: String,
        server_time: String,
    },
    /// Local version is newer than server version (dev build)
    Newer {
        local_time: String,
        server_time: String,
    },
    /// Cannot determine (missing timestamps or hashes)
    Unknown,
}

impl VersionStatus {
    /// Check if version is current
    pub fn is_current(&self) -> bool {
        matches!(self, VersionStatus::Current)
    }

    /// Check if an update is available
    pub fn needs_update(&self) -> bool {
        matches!(self, VersionStatus::Outdated { .. })
    }

    /// Human-readable status message
    pub fn message(&self) -> String {
        match self {
            VersionStatus::Current => "Up to date".to_string(),
            VersionStatus::Outdated {
                local_time,
                server_time,
            } => {
                format!("Update available (local: {}, server: {})", local_time, server_time)
            }
            VersionStatus::Newer {
                local_time,
                server_time,
            } => {
                format!("Development build (local: {}, server: {})", local_time, server_time)
            }
            VersionStatus::Unknown => "Version unknown".to_string(),
        }
    }
}

impl BuildVersion {
    /// Compare this version against another (typically server version)
    ///
    /// Returns the status indicating if this version is current, outdated, or newer.
    pub fn compare(&self, server: &BuildVersion) -> VersionStatus {
        // First check component hashes if available
        if let (Some(local_comp), Some(server_comp)) = (&self.components, &server.components) {
            // Check Leptos hashes
            if let (Some(local_leptos), Some(server_leptos)) = (&local_comp.leptos, &server_comp.leptos)
            {
                if local_leptos.wasm_hash == server_leptos.wasm_hash {
                    return VersionStatus::Current;
                }
            }
        }

        // Fall back to build time comparison
        if self.build_time.is_empty() || server.build_time.is_empty() {
            return VersionStatus::Unknown;
        }

        // ISO 8601 timestamps can be compared lexicographically
        match self.build_time.cmp(&server.build_time) {
            std::cmp::Ordering::Equal => VersionStatus::Current,
            std::cmp::Ordering::Less => VersionStatus::Outdated {
                local_time: self.build_time.clone(),
                server_time: server.build_time.clone(),
            },
            std::cmp::Ordering::Greater => VersionStatus::Newer {
                local_time: self.build_time.clone(),
                server_time: server.build_time.clone(),
            },
        }
    }

    /// Check if this version matches another exactly (same hashes)
    pub fn matches(&self, other: &BuildVersion) -> bool {
        if let (Some(self_comp), Some(other_comp)) = (&self.components, &other.components) {
            // Check all available component hashes
            let leptos_match = match (&self_comp.leptos, &other_comp.leptos) {
                (Some(a), Some(b)) => a.wasm_hash == b.wasm_hash && a.js_hash == b.js_hash,
                (None, None) => true,
                _ => false,
            };
            let bevy_match = match (&self_comp.bevy, &other_comp.bevy) {
                (Some(a), Some(b)) => a.wasm_hash == b.wasm_hash && a.js_hash == b.js_hash,
                (None, None) => true,
                _ => false,
            };
            leptos_match && bevy_match
        } else {
            // Fall back to build time comparison
            self.build_time == other.build_time
        }
    }
}

/// Compute MD5 hash of data, returning first 16 hex chars
pub fn compute_hash(data: &[u8]) -> String {
    let digest = md5::compute(data);
    // Return first 16 hex chars (8 bytes)
    hex::encode(&digest[..8])
}

/// Get current library version
pub fn library_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Simple ISO 8601 timestamp without external dependencies
fn chrono_lite_now() -> String {
    // Use std::time for a simple timestamp
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Simple conversion to UTC datetime components
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Algorithm from Howard Hinnant
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_version() {
        let version = BuildVersion::new();
        assert!(!version.build_time.is_empty());
        assert!(!version.version.is_empty());

        let json = version.to_json().unwrap();
        let parsed = BuildVersion::from_json(&json).unwrap();
        assert_eq!(version.version, parsed.version);
    }

    #[test]
    fn test_compute_hash() {
        let data = b"hello world";
        let hash = compute_hash(data);
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_with_components() {
        let components = ComponentHashes::new()
            .with_leptos("abc123", "def456")
            .with_bevy("ghi789", "jkl012");

        let version = BuildVersion::new()
            .with_components(components)
            .with_git_hash("abcd1234");

        let json = version.to_json().unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("ghi789"));
        assert!(json.contains("abcd1234"));
    }

    #[test]
    fn test_version_compare_current() {
        let components = ComponentHashes::new().with_leptos("hash1", "hash2");

        let local = BuildVersion {
            build_time: "2025-01-01T00:00:00Z".to_string(),
            version: "0.3.0".to_string(),
            git_hash: None,
            components: Some(components.clone()),
        };

        let server = BuildVersion {
            build_time: "2025-01-01T00:00:00Z".to_string(),
            version: "0.3.0".to_string(),
            git_hash: None,
            components: Some(components),
        };

        let status = local.compare(&server);
        assert!(matches!(status, VersionStatus::Current));
        assert!(status.is_current());
        assert!(!status.needs_update());
    }

    #[test]
    fn test_version_compare_outdated() {
        let local = BuildVersion {
            build_time: "2025-01-01T00:00:00Z".to_string(),
            version: "0.3.0".to_string(),
            git_hash: None,
            components: None,
        };

        let server = BuildVersion {
            build_time: "2025-01-02T00:00:00Z".to_string(),
            version: "0.3.1".to_string(),
            git_hash: None,
            components: None,
        };

        let status = local.compare(&server);
        assert!(matches!(status, VersionStatus::Outdated { .. }));
        assert!(status.needs_update());
        assert!(status.message().contains("Update available"));
    }

    #[test]
    fn test_version_compare_newer() {
        let local = BuildVersion {
            build_time: "2025-01-02T00:00:00Z".to_string(),
            version: "0.3.1".to_string(),
            git_hash: None,
            components: None,
        };

        let server = BuildVersion {
            build_time: "2025-01-01T00:00:00Z".to_string(),
            version: "0.3.0".to_string(),
            git_hash: None,
            components: None,
        };

        let status = local.compare(&server);
        assert!(matches!(status, VersionStatus::Newer { .. }));
        assert!(!status.needs_update());
        assert!(status.message().contains("Development build"));
    }

    #[test]
    fn test_version_matches() {
        let comp1 = ComponentHashes::new()
            .with_leptos("js1", "wasm1")
            .with_bevy("js2", "wasm2");

        let comp2 = ComponentHashes::new()
            .with_leptos("js1", "wasm1")
            .with_bevy("js2", "wasm2");

        let v1 = BuildVersion {
            build_time: "2025-01-01T00:00:00Z".to_string(),
            version: "0.3.0".to_string(),
            git_hash: None,
            components: Some(comp1),
        };

        let v2 = BuildVersion {
            build_time: "2025-01-01T00:00:00Z".to_string(),
            version: "0.3.0".to_string(),
            git_hash: None,
            components: Some(comp2),
        };

        assert!(v1.matches(&v2));
    }

    #[test]
    fn test_parse_real_version_json() {
        let json = r#"{
          "build_time": "2025-12-24T16:28:25Z",
          "version": "0.3.0",
          "git_hash": "abc123",
          "components": {
            "leptos": {
              "js_hash": "cc65d34ecd7f227b",
              "wasm_hash": "d4c59a70ccc13ee3"
            },
            "bevy": {
              "js_hash": "868d5b24370b0f88",
              "wasm_hash": "8ab07588f74a8653"
            }
          }
        }"#;

        let version = BuildVersion::from_json(json).unwrap();
        assert_eq!(version.build_time, "2025-12-24T16:28:25Z");
        assert_eq!(version.version, "0.3.0");
        assert_eq!(version.git_hash, Some("abc123".to_string()));

        let components = version.components.unwrap();
        let leptos = components.leptos.unwrap();
        assert_eq!(leptos.js_hash, "cc65d34ecd7f227b");
        assert_eq!(leptos.wasm_hash, "d4c59a70ccc13ee3");
    }
}
