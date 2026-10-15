//! SHA-256 and Model Artifact Integrity Verification.

use super::errors::AiError;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Buffer size for streaming SHA-256 calculation (64 KiB).
pub const STREAM_BUFFER_SIZE: usize = 65_536;

/// Computes the hex-encoded SHA-256 digest of a file using streaming reads with bounded memory.
pub fn compute_file_sha256(path: &Path) -> Result<String, AiError> {
    if !path.exists() {
        return Err(AiError::StorageError(format!(
            "Artifact file does not exist: {}",
            path.display()
        )));
    }

    let mut file = File::open(path)
        .map_err(|e| AiError::StorageError(format!("Failed to open file for hashing: {e}")))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; STREAM_BUFFER_SIZE];

    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(bytes_read) => hasher.update(&buffer[..bytes_read]),
            Err(e) => {
                return Err(AiError::StorageError(format!(
                    "Failed reading file during SHA-256 computation: {e}"
                )))
            }
        }
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Verifies that the file at `path` matches `expected_sha256` and `expected_size_bytes`.
pub fn verify_file_integrity(
    path: &Path,
    expected_sha256: &str,
    expected_size_bytes: u64,
) -> Result<(), AiError> {
    let metadata = std::fs::metadata(path).map_err(|e| {
        AiError::StorageError(format!(
            "Failed to read metadata for {}: {e}",
            path.display()
        ))
    })?;

    let actual_size = metadata.len();
    if actual_size != expected_size_bytes {
        return Err(AiError::SizeMismatch {
            expected: expected_size_bytes,
            actual: actual_size,
        });
    }

    let computed_sha256 = compute_file_sha256(path)?;
    if !constant_time_eq_str(&computed_sha256, expected_sha256) {
        return Err(AiError::Sha256Mismatch {
            expected: expected_sha256.to_string(),
            actual: computed_sha256,
        });
    }

    Ok(())
}

/// Constant-time string equality check to prevent timing leaks.
pub fn constant_time_eq_str(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    if a_bytes.len() != b_bytes.len() {
        return false;
    }

    let mut result: u8 = 0;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_sha256_known_string() {
        let mut file = NamedTempFile::new().expect("create temp");
        writeln!(file, "hello world").expect("write");
        let hash = compute_file_sha256(file.path()).expect("hash");
        // echo "hello world" | sha256sum -> d2a84f4b8b650937ec8f73cd8be2c74add5a911ba64df27458ed8229da804a26
        assert_eq!(
            hash,
            "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447"
        );
    }

    #[test]
    fn test_constant_time_equality() {
        assert!(constant_time_eq_str("abcdef", "abcdef"));
        assert!(!constant_time_eq_str("abcdef", "abcdeg"));
        assert!(!constant_time_eq_str("abcdef", "abcde"));
    }
}
