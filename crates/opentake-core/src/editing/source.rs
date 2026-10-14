use serde::{Deserialize, Serialize};

use super::error::EditError;
use super::time::MediaTimeUs;
use crate::media::identity::MediaAsset;

/// Authoritative reference to an external media source in the non-destructive editing model.
///
/// The EDL refers to media assets exclusively through stable identifiers and fingerprints,
/// never through raw volatile absolute paths or temporary proxy cache locations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReference {
    /// Canonical asset UUID or identifier registered with the media library.
    pub source_id: String,
    /// Immutable total duration of the original media file in microseconds.
    pub duration_us: MediaTimeUs,
    /// Content identity fingerprint or hash (e.g. SHA-256 or Blake3) to detect source tampering/replacement.
    pub fingerprint: Option<String>,
}

impl SourceReference {
    /// Constructs a validated `SourceReference`.
    ///
    /// # Errors
    /// Returns `EditError::InvalidSourceDuration` if `duration_us <= 0`.
    pub fn new(
        source_id: impl Into<String>,
        duration_us: MediaTimeUs,
        fingerprint: Option<String>,
    ) -> Result<Self, EditError> {
        let id_str = source_id.into();
        if id_str.trim().is_empty() {
            return Err(EditError::UnsupportedOperation(
                "Source ID cannot be empty".to_string(),
            ));
        }
        if !duration_us.is_positive() {
            return Err(EditError::InvalidSourceDuration {
                duration_us: duration_us.as_micros(),
            });
        }

        Ok(Self {
            source_id: id_str,
            duration_us,
            fingerprint,
        })
    }

    /// Adapts an authorized `MediaAsset` into a canonical `SourceReference`.
    pub fn from_media_asset(asset: &MediaAsset) -> Result<Self, EditError> {
        let duration_raw = asset
            .metadata
            .duration_micros()
            .ok_or(EditError::InvalidSourceDuration { duration_us: 0 })?;

        let duration_us = MediaTimeUs::from_micros(
            i64::try_from(duration_raw).map_err(|_| EditError::ArithmeticOverflow)?,
        );

        Self::new(
            asset.id.clone(),
            duration_us,
            Some(asset.source.identity_hint.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_source_reference() {
        let src = SourceReference::new(
            "asset_123",
            MediaTimeUs::from_micros(60_000_000),
            Some("hash".into()),
        )
        .unwrap();
        assert_eq!(src.source_id, "asset_123");
        assert_eq!(src.duration_us, MediaTimeUs(60_000_000));
        assert_eq!(src.fingerprint.as_deref(), Some("hash"));
    }

    #[test]
    fn test_invalid_source_reference() {
        // Zero duration
        assert!(SourceReference::new("asset_1", MediaTimeUs(0), None).is_err());
        // Negative duration
        assert!(SourceReference::new("asset_1", MediaTimeUs(-100), None).is_err());
        // Empty ID
        assert!(SourceReference::new("", MediaTimeUs(1_000_000), None).is_err());
    }
}
