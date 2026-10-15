//! System Font Discovery and Fallback Management for Subtitle Styling.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::process::Command;

/// Information about an available font family on the host system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontFamilyInfo {
    pub family: String,
    pub is_installed: bool,
    pub supports_bengali: bool,
    pub is_generic_fallback: bool,
}

/// Discovers available system fonts and provides guaranteed safe fallbacks.
pub fn discover_system_fonts() -> Vec<FontFamilyInfo> {
    let mut installed_families = BTreeSet::new();

    // Query fontconfig via fc-list if available on Linux
    if let Ok(output) = Command::new("fc-list").arg(":").arg("family").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // fc-list often outputs comma-separated alias lists like "DejaVu Sans,DejaVu Sans Light"
                for fam in line.split(',') {
                    let trimmed = fam.trim();
                    if !trimmed.is_empty() {
                        installed_families.insert(trimmed.to_string());
                    }
                }
            }
        }
    }

    // Common curated font list for subtitling
    let priority_families = [
        ("DejaVu Sans", false),
        ("DejaVu Serif", false),
        ("Noto Sans", false),
        ("Noto Sans Bengali", true),
        ("Noto Serif Bengali", true),
        ("Liberation Sans", false),
        ("Liberation Serif", false),
        ("Nimbus Sans", false),
        ("Arial", false),
        ("Roboto", false),
        ("Inter", false),
    ];

    let mut result = Vec::new();

    // Add priority fonts first
    for (fam, bengali) in priority_families {
        let is_installed = installed_families.contains(fam);
        result.push(FontFamilyInfo {
            family: fam.to_string(),
            is_installed,
            supports_bengali: bengali || fam.to_lowercase().contains("bengali"),
            is_generic_fallback: false,
        });
    }

    // Add other discovered sans/serif fonts (up to 40 additional clean fonts)
    let mut additional_count = 0;
    for fam in installed_families.iter() {
        if result.iter().any(|r| r.family.eq_ignore_ascii_case(fam)) {
            continue;
        }
        let lower = fam.to_lowercase();
        // Filter out decorative or math fonts to keep subtitle menu clean
        if lower.contains("sans")
            || lower.contains("serif")
            || lower.contains("bengali")
            || lower.contains("mono")
        {
            result.push(FontFamilyInfo {
                family: fam.clone(),
                is_installed: true,
                supports_bengali: lower.contains("bengali"),
                is_generic_fallback: false,
            });
            additional_count += 1;
            if additional_count >= 30 {
                break;
            }
        }
    }

    result
}

/// Resolves a safe fallback font family for export when the requested font is missing.
pub fn resolve_safe_font_family(requested: &str, is_bengali_script: bool) -> String {
    if is_bengali_script {
        // Prefer installed Bengali fonts
        "Noto Sans Bengali".to_string()
    } else if requested.trim().is_empty() {
        "DejaVu Sans".to_string()
    } else {
        requested.trim().to_string()
    }
}
