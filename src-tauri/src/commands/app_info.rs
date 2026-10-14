use serde::{Deserialize, Serialize};

/// Structured application metadata returned over the typed Tauri IPC bridge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_name: String,
    pub app_version: String,
    pub core_version: String,
    pub os_name: String,
    pub backend_ready: bool,
}

/// Typed Tauri IPC command returning application and core engine metadata.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        app_name: "Open Media Workbench".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        core_version: opentake_core::CORE_VERSION.to_string(),
        os_name: std::env::consts::OS.to_string(),
        backend_ready: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_app_info_returns_valid_data() {
        let info = get_app_info();
        assert_eq!(info.app_name, "Open Media Workbench");
        assert_eq!(info.core_version, opentake_core::CORE_VERSION);
        assert!(!info.app_version.is_empty());
        assert!(!info.os_name.is_empty());
        assert!(info.backend_ready);
    }

    #[test]
    fn test_app_info_serialization_camel_case() {
        let info = get_app_info();
        let json = serde_json::to_string(&info).expect("Failed to serialize AppInfo");
        assert!(json.contains("\"appName\":\"Open Media Workbench\""));
        assert!(json.contains("\"coreVersion\":"));
        assert!(json.contains("\"backendReady\":true"));
    }
}
