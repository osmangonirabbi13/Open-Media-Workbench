use serde::{Deserialize, Serialize};

use super::discovery::ToolType;
use super::error::MediaEngineError;

/// Structured parsed version and build configuration information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolVersionInfo {
    pub tool: ToolType,
    pub raw_version_string: String,
    pub major: Option<u32>,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub vendor_suffix: Option<String>,
    pub compiler_info: Option<String>,
    pub configuration_flags: Vec<String>,
    pub is_gpl: bool,
    pub has_libx264: bool,
    pub has_libopus: bool,
}

impl ToolVersionInfo {
    /// Formats a clean version display string (e.g. "8.0.1 (Ubuntu)").
    pub fn display_version(&self) -> String {
        match (self.major, self.minor, self.patch) {
            (Some(maj), Some(min), Some(pat)) => {
                if let Some(ref suffix) = self.vendor_suffix {
                    format!("{maj}.{min}.{pat} ({suffix})")
                } else {
                    format!("{maj}.{min}.{pat}")
                }
            }
            _ => self.raw_version_string.clone(),
        }
    }
}

/// Parses the standard `-version` stdout from FFmpeg or FFprobe.
pub fn parse_version_output(
    expected_tool: ToolType,
    stdout: &str,
) -> Result<ToolVersionInfo, MediaEngineError> {
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.is_empty() {
        return Err(MediaEngineError::OutputParseFailed {
            field: "version".to_string(),
            raw: "Empty output received from binary".to_string(),
        });
    }

    let first_line = lines[0].trim();

    // 1. Validate tool identity
    let expected_prefix = match expected_tool {
        ToolType::Ffmpeg => "ffmpeg version ",
        ToolType::Ffprobe => "ffprobe version ",
    };

    if !first_line.starts_with(expected_prefix) {
        // Check if the other tool is found instead
        let other_prefix = match expected_tool {
            ToolType::Ffmpeg => "ffprobe version",
            ToolType::Ffprobe => "ffmpeg version",
        };
        if first_line.starts_with(other_prefix) {
            return Err(MediaEngineError::WrongToolIdentity {
                expected: expected_tool.display_name().to_string(),
                found: if expected_tool == ToolType::Ffmpeg {
                    "FFprobe".to_string()
                } else {
                    "FFmpeg".to_string()
                },
            });
        }

        return Err(MediaEngineError::WrongToolIdentity {
            expected: expected_tool.display_name().to_string(),
            found: first_line.chars().take(40).collect(),
        });
    }

    // Extract version part: e.g. "8.0.1-3ubuntu2+esm5 Copyright..."
    let after_prefix = &first_line[expected_prefix.len()..];
    let version_token = after_prefix.split_whitespace().next().unwrap_or("").trim();

    let (major, minor, patch, vendor_suffix) = parse_semver_and_suffix(version_token);

    // 2. Parse compiler info and configuration flags
    let mut compiler_info = None;
    let mut configuration_flags = Vec::new();
    let mut is_gpl = false;
    let mut has_libx264 = false;
    let mut has_libopus = false;

    for line in &lines[1..] {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("built with ") {
            compiler_info = Some(rest.trim().to_string());
        } else if let Some(config_str) = trimmed.strip_prefix("configuration: ") {
            for flag in config_str.split_whitespace() {
                let flag_string = flag.trim().to_string();
                if flag_string == "--enable-gpl" {
                    is_gpl = true;
                } else if flag_string == "--enable-libx264" {
                    has_libx264 = true;
                } else if flag_string == "--enable-libopus" {
                    has_libopus = true;
                }
                configuration_flags.push(flag_string);
            }
        }
    }

    Ok(ToolVersionInfo {
        tool: expected_tool,
        raw_version_string: version_token.to_string(),
        major,
        minor,
        patch,
        vendor_suffix,
        compiler_info,
        configuration_flags,
        is_gpl,
        has_libx264,
        has_libopus,
    })
}

/// Helper to parse major.minor.patch and vendor suffixes like "8.0.1-3ubuntu2+esm5".
fn parse_semver_and_suffix(
    version_str: &str,
) -> (Option<u32>, Option<u32>, Option<u32>, Option<String>) {
    if version_str.is_empty() {
        return (None, None, None, None);
    }

    // Split at '-' or '+' to separate numeric semver from package suffix
    let parts: Vec<&str> = version_str.splitn(2, ['-', '+']).collect();
    let num_part = parts[0];
    let suffix_part = if parts.len() > 1 {
        Some(version_str[num_part.len() + 1..].to_string())
    } else {
        None
    };

    let segments: Vec<&str> = num_part.split('.').collect();
    let major = segments.first().and_then(|s| s.parse::<u32>().ok());
    let minor = segments.get(1).and_then(|s| s.parse::<u32>().ok());
    let patch = segments.get(2).and_then(|s| s.parse::<u32>().ok());

    (major, minor, patch, suffix_part)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_UBUNTU_FFMPEG_OUTPUT: &str = r#"ffmpeg version 8.0.1-3ubuntu2+esm5 Copyright (c) 2000-2025 the FFmpeg developers
built with gcc 15 (Ubuntu 15.2.0-16ubuntu1)
configuration: --prefix=/usr --enable-gpl --enable-libx264 --enable-libopus --enable-shared
libavutil      60.  8.100 / 60.  8.100
libavcodec     62. 11.100 / 62. 11.100"#;

    const SAMPLE_FFPROBE_OUTPUT: &str = r#"ffprobe version 8.0.1-3ubuntu2+esm5 Copyright (c) 2007-2025 the FFmpeg developers
built with gcc 15 (Ubuntu 15.2.0-16ubuntu1)
configuration: --prefix=/usr --enable-gpl --enable-libx264
libavutil      60.  8.100 / 60.  8.100"#;

    #[test]
    fn test_parse_ubuntu_ffmpeg_version() {
        let parsed = parse_version_output(ToolType::Ffmpeg, SAMPLE_UBUNTU_FFMPEG_OUTPUT)
            .expect("Should parse");
        assert_eq!(parsed.tool, ToolType::Ffmpeg);
        assert_eq!(parsed.major, Some(8));
        assert_eq!(parsed.minor, Some(0));
        assert_eq!(parsed.patch, Some(1));
        assert_eq!(parsed.vendor_suffix.as_deref(), Some("3ubuntu2+esm5"));
        assert!(parsed.is_gpl);
        assert!(parsed.has_libx264);
        assert!(parsed.has_libopus);
        assert_eq!(parsed.display_version(), "8.0.1 (3ubuntu2+esm5)");
    }

    #[test]
    fn test_parse_ffprobe_version() {
        let parsed =
            parse_version_output(ToolType::Ffprobe, SAMPLE_FFPROBE_OUTPUT).expect("Should parse");
        assert_eq!(parsed.tool, ToolType::Ffprobe);
        assert_eq!(parsed.major, Some(8));
        assert_eq!(parsed.minor, Some(0));
        assert_eq!(parsed.patch, Some(1));
        assert!(parsed.is_gpl);
    }

    #[test]
    fn test_wrong_tool_identity_rejected() {
        let err = parse_version_output(ToolType::Ffmpeg, SAMPLE_FFPROBE_OUTPUT).unwrap_err();
        match err {
            MediaEngineError::WrongToolIdentity { expected, found } => {
                assert_eq!(expected, "FFmpeg");
                assert_eq!(found, "FFprobe");
            }
            other => panic!("Unexpected error type: {other:?}"),
        }
    }

    #[test]
    fn test_empty_output_rejected() {
        let err = parse_version_output(ToolType::Ffmpeg, "").unwrap_err();
        assert_eq!(err.code(), "OUTPUT_PARSE_FAILED");
    }

    #[test]
    fn test_simple_semver_without_suffix() {
        let simple_out = "ffmpeg version 7.1 Copyright (c) 2000-2024\nconfiguration: --enable-gpl";
        let parsed = parse_version_output(ToolType::Ffmpeg, simple_out).expect("Should parse");
        assert_eq!(parsed.major, Some(7));
        assert_eq!(parsed.minor, Some(1));
        assert_eq!(parsed.patch, None);
        assert_eq!(parsed.vendor_suffix, None);
        assert_eq!(parsed.display_version(), "7.1");
    }
}
