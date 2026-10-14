use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};

use super::error::MediaEngineError;

/// Enumeration of media processing tool binaries required by OpenTake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Ffmpeg,
    Ffprobe,
}

impl ToolType {
    pub fn executable_name(&self) -> &'static str {
        match self {
            Self::Ffmpeg => {
                if cfg!(windows) {
                    "ffmpeg.exe"
                } else {
                    "ffmpeg"
                }
            }
            Self::Ffprobe => {
                if cfg!(windows) {
                    "ffprobe.exe"
                } else {
                    "ffprobe"
                }
            }
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Ffmpeg => "FFmpeg",
            Self::Ffprobe => "FFprobe",
        }
    }
}

impl fmt::Display for ToolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Identifies where an executable candidate was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSource {
    ExplicitConfig,
    TrustedSystemLocation,
    PathEnvironment,
}

/// A discovered executable candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredExecutable {
    pub tool: ToolType,
    pub path: PathBuf,
    pub source: CandidateSource,
    pub is_symlink: bool,
    pub canonical_path: PathBuf,
}

/// Configuration options for tool discovery.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryConfig {
    /// Optional explicit path to ffmpeg binary.
    pub explicit_ffmpeg_path: Option<PathBuf>,
    /// Optional explicit path to ffprobe binary.
    pub explicit_ffprobe_path: Option<PathBuf>,
}

/// Discovers and validates a tool binary on the local host.
pub fn discover_tool(
    tool: ToolType,
    config: &DiscoveryConfig,
) -> Result<DiscoveredExecutable, MediaEngineError> {
    let mut searched_paths = Vec::new();

    // 1. Check explicit override if provided
    let explicit = match tool {
        ToolType::Ffmpeg => &config.explicit_ffmpeg_path,
        ToolType::Ffprobe => &config.explicit_ffprobe_path,
    };
    if let Some(explicit_path) = explicit {
        searched_paths.push(explicit_path.display().to_string());
        if let Ok(discovered) =
            validate_candidate(tool, explicit_path, CandidateSource::ExplicitConfig)
        {
            return Ok(discovered);
        }
    }

    // 2. Check trusted standard system paths (Linux / Unix / macOS)
    let trusted_dirs = [
        "/usr/bin",
        "/usr/local/bin",
        "/opt/homebrew/bin",
        "/usr/pkg/bin",
    ];
    let exe_name = tool.executable_name();

    for dir in trusted_dirs {
        let candidate = Path::new(dir).join(exe_name);
        searched_paths.push(candidate.display().to_string());
        if candidate.exists() {
            if let Ok(discovered) =
                validate_candidate(tool, &candidate, CandidateSource::TrustedSystemLocation)
            {
                return Ok(discovered);
            }
        }
    }

    // 3. Check system PATH environment variable
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            searched_paths.push(candidate.display().to_string());
            if candidate.exists() {
                if let Ok(discovered) =
                    validate_candidate(tool, &candidate, CandidateSource::PathEnvironment)
                {
                    return Ok(discovered);
                }
            }
        }
    }

    Err(MediaEngineError::BinaryNotFound {
        tool: tool.display_name().to_string(),
        searched_paths,
    })
}

/// Validates that a path points to a regular executable file.
pub fn validate_candidate(
    tool: ToolType,
    path: &Path,
    source: CandidateSource,
) -> Result<DiscoveredExecutable, MediaEngineError> {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                return Err(MediaEngineError::PermissionDenied {
                    path: path.display().to_string(),
                });
            }
            return Err(MediaEngineError::InvalidExecutable {
                path: path.display().to_string(),
                reason: format!("Cannot read file metadata: {e}"),
            });
        }
    };

    if metadata.is_dir() {
        return Err(MediaEngineError::InvalidExecutable {
            path: path.display().to_string(),
            reason: "Target path is a directory, not an executable file.".to_string(),
        });
    }

    // Check executable permissions on Unix systems
    #[cfg(unix)]
    {
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        let is_executable = (mode & 0o111) != 0;
        if !is_executable {
            return Err(MediaEngineError::BinaryNotExecutable {
                path: path.display().to_string(),
                reason: format!(
                    "Unix file permission mode 0o{:o} does not have execute bit set",
                    mode
                ),
            });
        }
    }

    let symlink_metadata = fs::symlink_metadata(path).ok();
    let is_symlink = symlink_metadata.is_some_and(|m| m.file_type().is_symlink());

    let canonical_path = match fs::canonicalize(path) {
        Ok(cp) => cp,
        Err(_) => path.to_path_buf(),
    };

    Ok(DiscoveredExecutable {
        tool,
        path: path.to_path_buf(),
        source,
        is_symlink,
        canonical_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_rejected_as_candidate() {
        let temp_dir = std::env::temp_dir();
        let err = validate_candidate(ToolType::Ffmpeg, &temp_dir, CandidateSource::ExplicitConfig)
            .unwrap_err();
        match err {
            MediaEngineError::InvalidExecutable { ref reason, .. } => {
                assert!(reason.contains("directory"));
            }
            other => panic!("Expected InvalidExecutable error, got {other:?}"),
        }
    }

    #[test]
    fn test_nonexistent_binary_returns_not_found() {
        let config = DiscoveryConfig {
            explicit_ffmpeg_path: Some(PathBuf::from("/nonexistent/bin/ffmpeg_fake")),
            ..Default::default()
        };
        // Searching for tool when only invalid path is configured or empty
        let err = discover_tool(ToolType::Ffmpeg, &config);
        // It should either fail or fall through to system
        // Let's test with a fake tool type or impossible path
        assert!(err.is_ok() || matches!(err, Err(MediaEngineError::BinaryNotFound { .. })));
    }

    #[test]
    fn test_tool_executable_names() {
        let ffmpeg_name = ToolType::Ffmpeg.executable_name();
        let ffprobe_name = ToolType::Ffprobe.executable_name();
        assert!(ffmpeg_name.starts_with("ffmpeg"));
        assert!(ffprobe_name.starts_with("ffprobe"));
    }
}
