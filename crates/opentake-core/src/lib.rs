//! # OpenTake Core Library
//!
//! `opentake-core` is the pure, headless systems engine for OpenTake.
//! It encapsulates all media analysis, non-destructive editing interval math,
//! export filtergraph compilation, and project persistence without any
//! dependency on Tauri, WebViews, or UI frameworks.

pub mod ai;
pub mod editing;
pub mod localization;
pub mod media;
pub mod repair;

use serde::{Deserialize, Serialize};

/// Current semantic version of the core domain engine.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Core domain engine descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreDescriptor {
    pub name: String,
    pub version: String,
    pub description: String,
    pub is_ready: bool,
}

impl Default for CoreDescriptor {
    fn default() -> Self {
        Self {
            name: "opentake-core".to_string(),
            version: CORE_VERSION.to_string(),
            description: "OpenTake Headless Media & Editing Domain Engine".to_string(),
            is_ready: true,
        }
    }
}

/// Returns a default descriptor for the core engine.
pub fn get_core_descriptor() -> CoreDescriptor {
    CoreDescriptor::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_version_is_valid() {
        assert!(!CORE_VERSION.is_empty());
        assert_eq!(CORE_VERSION, "0.1.0-alpha.1");
    }

    #[test]
    fn test_core_descriptor_default() {
        let desc = get_core_descriptor();
        assert_eq!(desc.name, "opentake-core");
        assert_eq!(desc.version, CORE_VERSION);
        assert!(desc.is_ready);
    }

    #[test]
    fn test_core_descriptor_serialization_roundtrip() {
        let desc = get_core_descriptor();
        let serialized = serde_json::to_string(&desc).expect("Failed to serialize");
        let deserialized: CoreDescriptor =
            serde_json::from_str(&serialized).expect("Failed to deserialize");
        assert_eq!(desc, deserialized);
    }
}
