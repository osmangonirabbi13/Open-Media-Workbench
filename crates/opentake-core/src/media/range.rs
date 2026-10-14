use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::error::MediaEngineError;

/// Maximum single byte-range payload delivered in one HTTP response (2 MiB).
/// Prevents large memory spikes and ensures bounded response buffering.
pub const MAX_RANGE_CHUNK_BYTES: usize = 2 * 1024 * 1024;

/// A resolved, validated byte range with inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedByteRange {
    /// Zero-based inclusive start byte offset.
    pub start: u64,
    /// Zero-based inclusive end byte offset.
    pub end: u64,
    /// Total file size in bytes.
    pub total_size: u64,
}

impl ResolvedByteRange {
    /// Number of bytes represented in this range.
    pub fn length(&self) -> u64 {
        self.end.saturating_sub(self.start).saturating_add(1)
    }

    /// Formats the standard HTTP Content-Range header value.
    /// E.g. "bytes 0-1023/1048576"
    pub fn content_range_header(&self) -> String {
        format!("bytes {}-{}/{}", self.start, self.end, self.total_size)
    }
}

/// Parses an optional HTTP `Range` header (e.g. `bytes=0-1024`, `bytes=1024-`, `bytes=-500`).
///
/// # Returns
/// - `Ok(None)`: No Range header provided (indicates full file GET, HTTP 200).
/// - `Ok(Some(ResolvedByteRange))`: Valid range resolved against `file_size` (HTTP 206).
/// - `Err(MediaEngineError::RangeNotSatisfiable)`: Out of bounds (HTTP 416).
/// - `Err(MediaEngineError::InvalidRange)`: Syntactically malformed header (HTTP 400).
pub fn parse_byte_range(
    range_header: Option<&str>,
    file_size: u64,
) -> Result<Option<ResolvedByteRange>, MediaEngineError> {
    let Some(header) = range_header else {
        return Ok(None);
    };

    let trimmed = header.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    // Must start with "bytes="
    if !trimmed.starts_with("bytes=") {
        return Err(MediaEngineError::InvalidRange {
            details: "Range header must start with 'bytes='".to_string(),
        });
    }

    let spec = &trimmed["bytes=".len()..].trim();

    // Reject multi-range requests (comma separated) for strict predictable security
    if spec.contains(',') {
        return Err(MediaEngineError::InvalidRange {
            details: "Multi-range requests are not supported".to_string(),
        });
    }

    let parts: Vec<&str> = spec.split('-').collect();
    if parts.len() != 2 {
        return Err(MediaEngineError::InvalidRange {
            details: format!("Malformed range specifier: '{spec}'"),
        });
    }

    let (start_str, end_str) = (parts[0].trim(), parts[1].trim());

    if start_str.is_empty() && end_str.is_empty() {
        return Err(MediaEngineError::InvalidRange {
            details: "Both start and end range boundaries are empty".to_string(),
        });
    }

    if file_size == 0 {
        return Err(MediaEngineError::RangeNotSatisfiable {
            start: 0,
            end: 0,
            total: 0,
        });
    }

    // Case 1: Suffix range `bytes=-500` (last N bytes of the file)
    if start_str.is_empty() {
        let suffix_len = end_str
            .parse::<u64>()
            .map_err(|e| MediaEngineError::InvalidRange {
                details: format!("Invalid suffix length: {e}"),
            })?;

        if suffix_len == 0 {
            return Err(MediaEngineError::InvalidRange {
                details: "Suffix length cannot be 0".to_string(),
            });
        }

        let start = file_size.saturating_sub(suffix_len);
        let end = file_size.saturating_sub(1);

        return Ok(Some(ResolvedByteRange {
            start,
            end,
            total_size: file_size,
        }));
    }

    let start = start_str
        .parse::<u64>()
        .map_err(|e| MediaEngineError::InvalidRange {
            details: format!("Invalid start byte offset: {e}"),
        })?;

    // If start is at or beyond file size, range is unsatisfiable
    if start >= file_size {
        return Err(MediaEngineError::RangeNotSatisfiable {
            start,
            end: start,
            total: file_size,
        });
    }

    // Case 2: Open-ended range `bytes=1024-`
    if end_str.is_empty() {
        let end = file_size.saturating_sub(1);
        return Ok(Some(ResolvedByteRange {
            start,
            end,
            total_size: file_size,
        }));
    }

    // Case 3: Explicit range `bytes=0-1024`
    let end = end_str
        .parse::<u64>()
        .map_err(|e| MediaEngineError::InvalidRange {
            details: format!("Invalid end byte offset: {e}"),
        })?;

    if start > end {
        return Err(MediaEngineError::InvalidRange {
            details: format!("Start byte ({start}) cannot exceed end byte ({end})"),
        });
    }

    // Clamp end to file_size - 1 per HTTP RFC 7233
    let clamped_end = end.min(file_size.saturating_sub(1));

    Ok(Some(ResolvedByteRange {
        start,
        end: clamped_end,
        total_size: file_size,
    }))
}

/// Reads a bounded byte slice from disk for a resolved byte range.
/// Enforces bounded allocation (`max_chunk_bytes`).
pub fn read_range_slice(
    path: &Path,
    range: &ResolvedByteRange,
    max_chunk_bytes: usize,
) -> Result<Vec<u8>, MediaEngineError> {
    let mut file = File::open(path).map_err(|e| MediaEngineError::PermissionDenied {
        path: format!("{}: {e}", path.display()),
    })?;

    file.seek(SeekFrom::Start(range.start))
        .map_err(|e| MediaEngineError::Internal(format!("Seek failed: {e}")))?;

    let requested_len = range.length();
    let read_len = requested_len.min(max_chunk_bytes as u64) as usize;

    let mut buffer = vec![0u8; read_len];
    let bytes_read = file
        .read(&mut buffer)
        .map_err(|e| MediaEngineError::Internal(format!("Read failed: {e}")))?;

    buffer.truncate(bytes_read);
    Ok(buffer)
}

/// Determines the proper video MIME type based on file extension and format hint.
pub fn determine_mime_type(path: &Path, format_hint: Option<&str>) -> &'static str {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "aac" => "audio/aac",
        "ogg" => "audio/ogg",
        _ => {
            if let Some(hint) = format_hint {
                let h = hint.to_lowercase();
                if h.contains("mp4") || h.contains("mov") {
                    "video/mp4"
                } else if h.contains("webm") {
                    "video/webm"
                } else if h.contains("matroska") {
                    "video/x-matroska"
                } else {
                    "video/mp4"
                }
            } else {
                "video/mp4"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_range_none() {
        let res = parse_byte_range(None, 1000).unwrap();
        assert_eq!(res, None);

        let res_empty = parse_byte_range(Some(""), 1000).unwrap();
        assert_eq!(res_empty, None);
    }

    #[test]
    fn test_parse_range_single_bounds() {
        let res = parse_byte_range(Some("bytes=0-499"), 1000)
            .unwrap()
            .unwrap();
        assert_eq!(res.start, 0);
        assert_eq!(res.end, 499);
        assert_eq!(res.total_size, 1000);
        assert_eq!(res.length(), 500);
        assert_eq!(res.content_range_header(), "bytes 0-499/1000");
    }

    #[test]
    fn test_parse_range_open_ended() {
        let res = parse_byte_range(Some("bytes=500-"), 1000).unwrap().unwrap();
        assert_eq!(res.start, 500);
        assert_eq!(res.end, 999);
        assert_eq!(res.length(), 500);
    }

    #[test]
    fn test_parse_range_suffix() {
        let res = parse_byte_range(Some("bytes=-300"), 1000).unwrap().unwrap();
        assert_eq!(res.start, 700);
        assert_eq!(res.end, 999);
        assert_eq!(res.length(), 300);
    }

    #[test]
    fn test_parse_range_clamped_to_eof() {
        let res = parse_byte_range(Some("bytes=0-2000"), 1000)
            .unwrap()
            .unwrap();
        assert_eq!(res.start, 0);
        assert_eq!(res.end, 999); // Clamped to file_size - 1
    }

    #[test]
    fn test_parse_range_unsatisfiable() {
        let res = parse_byte_range(Some("bytes=1000-1500"), 1000);
        assert!(matches!(
            res,
            Err(MediaEngineError::RangeNotSatisfiable { .. })
        ));

        let res_zero = parse_byte_range(Some("bytes=0-100"), 0);
        assert!(matches!(
            res_zero,
            Err(MediaEngineError::RangeNotSatisfiable { .. })
        ));
    }

    #[test]
    fn test_parse_range_invalid_syntax() {
        assert!(matches!(
            parse_byte_range(Some("invalid"), 1000),
            Err(MediaEngineError::InvalidRange { .. })
        ));
        assert!(matches!(
            parse_byte_range(Some("bytes=500-200"), 1000),
            Err(MediaEngineError::InvalidRange { .. })
        ));
        assert!(matches!(
            parse_byte_range(Some("bytes=0-100,200-300"), 1000),
            Err(MediaEngineError::InvalidRange { .. })
        ));
    }

    #[test]
    fn test_read_range_slice_bounded() {
        let mut temp = NamedTempFile::new().unwrap();
        let sample_data = b"0123456789abcdefghijklmnopqrstuvwxyz";
        temp.write_all(sample_data).unwrap();

        let range = ResolvedByteRange {
            start: 10,
            end: 15,
            total_size: sample_data.len() as u64,
        };

        let slice = read_range_slice(temp.path(), &range, 1024).unwrap();
        assert_eq!(slice, b"abcdef");
    }

    #[test]
    fn test_determine_mime_type() {
        assert_eq!(
            determine_mime_type(Path::new("video.mp4"), None),
            "video/mp4"
        );
        assert_eq!(
            determine_mime_type(Path::new("movie.webm"), None),
            "video/webm"
        );
        assert_eq!(
            determine_mime_type(Path::new("clip.mov"), None),
            "video/quicktime"
        );
    }
}
