use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::SubtitleError;
use super::model::{SubtitleCue, SubtitleDocument, SubtitleTrack};
use crate::editing::time::MediaTimeUs;

/// Atomic subtitle editing operations supported by Open Media Workbench.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum SubtitleOperation {
    /// Appends or inserts a new cue into a track.
    AddCue { track_id: String, cue: SubtitleCue },
    /// Removes a single cue by ID.
    DeleteCue { track_id: String, cue_id: String },
    /// Atomically removes multiple cues by ID.
    DeleteCues {
        track_id: String,
        cue_ids: Vec<String>,
    },
    /// Updates the text content of a cue.
    UpdateCueText {
        track_id: String,
        cue_id: String,
        text: String,
    },
    /// Updates speaker metadata for a cue.
    UpdateCueSpeaker {
        track_id: String,
        cue_id: String,
        speaker: Option<String>,
    },
    /// Explicitly updates start and end boundaries in microseconds.
    SetCueTiming {
        track_id: String,
        cue_id: String,
        start_us: MediaTimeUs,
        end_us: MediaTimeUs,
    },
    /// Translates a cue along the timeline by `delta_us` (positive = later, negative = earlier).
    MoveCue {
        track_id: String,
        cue_id: String,
        delta_us: i64,
    },
    /// Resizes the left boundary (start_us) of a cue without moving end_us.
    ResizeCueStart {
        track_id: String,
        cue_id: String,
        new_start_us: MediaTimeUs,
    },
    /// Resizes the right boundary (end_us) of a cue without moving start_us.
    ResizeCueEnd {
        track_id: String,
        cue_id: String,
        new_end_us: MediaTimeUs,
    },
    /// Splits an existing cue at `split_time_us` into two adjacent cues.
    SplitCue {
        track_id: String,
        cue_id: String,
        split_time_us: MediaTimeUs,
    },
    /// Merges two adjacent cues into a single combined cue.
    MergeCues {
        track_id: String,
        first_cue_id: String,
        second_cue_id: String,
    },
    /// Atomically shifts a batch of cues by `delta_us`.
    ShiftSelectedCues {
        track_id: String,
        cue_ids: Vec<String>,
        delta_us: i64,
    },
    /// Clones an existing cue and places it immediately following the source cue.
    DuplicateCue { track_id: String, cue_id: String },
    /// Updates track metadata (e.g. language tag or display name).
    UpdateTrackMetadata {
        track_id: String,
        language: Option<String>,
        display_name: Option<String>,
    },
}

impl SubtitleTrack {
    /// Applies an atomic edit operation to this track.
    ///
    /// Validates all invariants:
    /// - $start\_us \ge 0$
    /// - $end\_us > start\_us$
    /// - Chronological cue ordering maintained after changes
    /// - Safe Unicode handling
    pub fn apply_operation(&mut self, op: &SubtitleOperation) -> Result<(), SubtitleError> {
        match op {
            SubtitleOperation::AddCue { cue, .. } => {
                if cue.start_us.as_micros() < 0 {
                    return Err(SubtitleError::NegativeTime {
                        time_us: cue.start_us.as_micros(),
                    });
                }
                if cue.end_us <= cue.start_us {
                    return Err(SubtitleError::InvalidTimeRange {
                        start_us: cue.start_us.as_micros(),
                        end_us: cue.end_us.as_micros(),
                    });
                }
                self.cues.push(cue.clone());
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::DeleteCue { cue_id, .. } => {
                let initial_len = self.cues.len();
                self.cues.retain(|c| &c.id != cue_id);
                if self.cues.len() == initial_len {
                    return Err(SubtitleError::CueNotFound(cue_id.clone()));
                }
                Ok(())
            }

            SubtitleOperation::DeleteCues { cue_ids, .. } => {
                self.cues.retain(|c| !cue_ids.contains(&c.id));
                Ok(())
            }

            SubtitleOperation::UpdateCueText { cue_id, text, .. } => {
                let cue = self
                    .cues
                    .iter_mut()
                    .find(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?;
                cue.text = text.clone();
                Ok(())
            }

            SubtitleOperation::UpdateCueSpeaker {
                cue_id, speaker, ..
            } => {
                let cue = self
                    .cues
                    .iter_mut()
                    .find(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?;
                cue.speaker = speaker.clone();
                Ok(())
            }

            SubtitleOperation::SetCueTiming {
                cue_id,
                start_us,
                end_us,
                ..
            } => {
                if start_us.as_micros() < 0 {
                    return Err(SubtitleError::NegativeTime {
                        time_us: start_us.as_micros(),
                    });
                }
                if end_us <= start_us {
                    return Err(SubtitleError::InvalidTimeRange {
                        start_us: start_us.as_micros(),
                        end_us: end_us.as_micros(),
                    });
                }

                let cue = self
                    .cues
                    .iter_mut()
                    .find(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?;

                cue.start_us = *start_us;
                cue.end_us = *end_us;
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::MoveCue {
                cue_id, delta_us, ..
            } => {
                let cue = self
                    .cues
                    .iter()
                    .find(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?;

                let new_start =
                    cue.start_us
                        .as_micros()
                        .checked_add(*delta_us)
                        .ok_or_else(|| {
                            SubtitleError::OperationError(
                                "Timestamp arithmetic overflow during move".to_string(),
                            )
                        })?;

                let new_end = cue
                    .end_us
                    .as_micros()
                    .checked_add(*delta_us)
                    .ok_or_else(|| {
                        SubtitleError::OperationError(
                            "Timestamp arithmetic overflow during move".to_string(),
                        )
                    })?;

                if new_start < 0 {
                    return Err(SubtitleError::NegativeTime { time_us: new_start });
                }

                let cue_mut = self.cues.iter_mut().find(|c| &c.id == cue_id).unwrap();
                cue_mut.start_us = MediaTimeUs(new_start);
                cue_mut.end_us = MediaTimeUs(new_end);
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::ResizeCueStart {
                cue_id,
                new_start_us,
                ..
            } => {
                if new_start_us.as_micros() < 0 {
                    return Err(SubtitleError::NegativeTime {
                        time_us: new_start_us.as_micros(),
                    });
                }

                let cue = self
                    .cues
                    .iter_mut()
                    .find(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?;

                if *new_start_us >= cue.end_us {
                    return Err(SubtitleError::InvalidTimeRange {
                        start_us: new_start_us.as_micros(),
                        end_us: cue.end_us.as_micros(),
                    });
                }

                cue.start_us = *new_start_us;
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::ResizeCueEnd {
                cue_id, new_end_us, ..
            } => {
                let cue = self
                    .cues
                    .iter_mut()
                    .find(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?;

                if *new_end_us <= cue.start_us {
                    return Err(SubtitleError::InvalidTimeRange {
                        start_us: cue.start_us.as_micros(),
                        end_us: new_end_us.as_micros(),
                    });
                }

                cue.end_us = *new_end_us;
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::SplitCue {
                cue_id,
                split_time_us,
                ..
            } => {
                let idx = self
                    .cues
                    .iter()
                    .position(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?;

                let original = &self.cues[idx];
                if *split_time_us <= original.start_us || *split_time_us >= original.end_us {
                    return Err(SubtitleError::InvalidTimeRange {
                        start_us: split_time_us.as_micros(),
                        end_us: original.end_us.as_micros(),
                    });
                }

                // Deterministically split text at word or newline boundaries if feasible
                let (first_text, second_text) = split_subtitle_text(&original.text);

                let original_end = original.end_us;
                let speaker = original.speaker.clone();
                let settings = original.settings.clone();

                // Mutate first half
                let first_cue = &mut self.cues[idx];
                first_cue.end_us = *split_time_us;
                first_cue.text = first_text;
                first_cue.words = None;

                // Create second half
                let second_cue = SubtitleCue {
                    id: Uuid::new_v4().to_string(),
                    start_us: *split_time_us,
                    end_us: original_end,
                    text: second_text,
                    speaker,
                    settings,
                    raw_identifier: None,
                    custom_metadata: std::collections::HashMap::new(),
                    words: None,
                };

                self.cues.push(second_cue);
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::MergeCues {
                first_cue_id,
                second_cue_id,
                ..
            } => {
                let first_pos = self
                    .cues
                    .iter()
                    .position(|c| &c.id == first_cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(first_cue_id.clone()))?;

                let second_pos = self
                    .cues
                    .iter()
                    .position(|c| &c.id == second_cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(second_cue_id.clone()))?;

                if first_pos == second_pos {
                    return Err(SubtitleError::OperationError(
                        "Cannot merge a cue with itself".to_string(),
                    ));
                }

                let (earlier_idx, later_idx) =
                    if self.cues[first_pos].start_us <= self.cues[second_pos].start_us {
                        (first_pos, second_pos)
                    } else {
                        (second_pos, first_pos)
                    };

                let later = self.cues[later_idx].clone();
                let earlier = &mut self.cues[earlier_idx];

                // Extend end time to cover both intervals
                if later.end_us > earlier.end_us {
                    earlier.end_us = later.end_us;
                }

                // Deterministically join text
                if earlier.text.trim().is_empty() {
                    earlier.text = later.text;
                } else if !later.text.trim().is_empty() {
                    earlier.text = format!("{}\n{}", earlier.text.trim(), later.text.trim());
                }

                // Remove the merged later cue
                self.cues.remove(later_idx);
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::ShiftSelectedCues {
                cue_ids, delta_us, ..
            } => {
                if cue_ids.is_empty() || *delta_us == 0 {
                    return Ok(());
                }

                // First pass: validate that ALL cues can shift without violating start >= 0
                for cue in self.cues.iter().filter(|c| cue_ids.contains(&c.id)) {
                    let new_start =
                        cue.start_us
                            .as_micros()
                            .checked_add(*delta_us)
                            .ok_or_else(|| {
                                SubtitleError::OperationError(
                                    "Timestamp arithmetic overflow during batch shift".to_string(),
                                )
                            })?;

                    if new_start < 0 {
                        return Err(SubtitleError::NegativeTime { time_us: new_start });
                    }
                }

                // Second pass: apply shifts atomically
                for cue in self.cues.iter_mut().filter(|c| cue_ids.contains(&c.id)) {
                    let new_start = cue.start_us.as_micros() + delta_us;
                    let new_end = cue.end_us.as_micros() + delta_us;
                    cue.start_us = MediaTimeUs(new_start);
                    cue.end_us = MediaTimeUs(new_end);
                }

                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::DuplicateCue { cue_id, .. } => {
                let cue = self
                    .cues
                    .iter()
                    .find(|c| &c.id == cue_id)
                    .ok_or_else(|| SubtitleError::CueNotFound(cue_id.clone()))?
                    .clone();

                let duration = cue.duration_us();
                let new_start = cue.end_us;
                let new_end = MediaTimeUs(cue.end_us.as_micros() + duration.as_micros());

                let duplicated = SubtitleCue {
                    id: Uuid::new_v4().to_string(),
                    start_us: new_start,
                    end_us: new_end,
                    text: cue.text,
                    speaker: cue.speaker,
                    settings: cue.settings,
                    raw_identifier: None,
                    custom_metadata: cue.custom_metadata,
                    words: cue.words,
                };

                self.cues.push(duplicated);
                self.sort_cues();
                Ok(())
            }

            SubtitleOperation::UpdateTrackMetadata {
                language,
                display_name,
                ..
            } => {
                if let Some(ref lang) = language {
                    self.language = lang.clone();
                }
                if let Some(ref name) = display_name {
                    self.display_name = name.clone();
                }
                Ok(())
            }
        }
    }
}

impl SubtitleDocument {
    /// Applies an operation to the targeted track in this document, updating `updated_at_epoch_ms`.
    pub fn apply_operation(&mut self, op: SubtitleOperation) -> Result<(), SubtitleError> {
        let track_id = match &op {
            SubtitleOperation::AddCue { track_id, .. } => track_id,
            SubtitleOperation::DeleteCue { track_id, .. } => track_id,
            SubtitleOperation::DeleteCues { track_id, .. } => track_id,
            SubtitleOperation::UpdateCueText { track_id, .. } => track_id,
            SubtitleOperation::UpdateCueSpeaker { track_id, .. } => track_id,
            SubtitleOperation::SetCueTiming { track_id, .. } => track_id,
            SubtitleOperation::MoveCue { track_id, .. } => track_id,
            SubtitleOperation::ResizeCueStart { track_id, .. } => track_id,
            SubtitleOperation::ResizeCueEnd { track_id, .. } => track_id,
            SubtitleOperation::SplitCue { track_id, .. } => track_id,
            SubtitleOperation::MergeCues { track_id, .. } => track_id,
            SubtitleOperation::ShiftSelectedCues { track_id, .. } => track_id,
            SubtitleOperation::DuplicateCue { track_id, .. } => track_id,
            SubtitleOperation::UpdateTrackMetadata { track_id, .. } => track_id,
        };

        let track = self
            .tracks
            .iter_mut()
            .find(|t| &t.id == track_id)
            .ok_or_else(|| SubtitleError::TrackNotFound(track_id.clone()))?;

        track.apply_operation(&op)?;
        self.touch();
        Ok(())
    }
}

/// Splits subtitle text cleanly at a newline or midpoint word boundary, safely preserving Unicode and Bengali.
fn split_subtitle_text(text: &str) -> (String, String) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    // If text has multiple lines, split by lines
    if let Some(idx) = trimmed.find('\n') {
        let first = trimmed[..idx].trim().to_string();
        let second = trimmed[idx + 1..].trim().to_string();
        if !first.is_empty() && !second.is_empty() {
            return (first, second);
        }
    }

    // Split by words near midpoint
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= 2 {
        let mid = words.len() / 2;
        let first = words[..mid].join(" ");
        let second = words[mid..].join(" ");
        return (first, second);
    }

    // Fallback: single word or character-based split
    let char_count = trimmed.chars().count();
    if char_count >= 2 {
        let mid_char = char_count / 2;
        let first: String = trimmed.chars().take(mid_char).collect();
        let second: String = trimmed.chars().skip(mid_char).collect();
        return (first, second);
    }

    (trimmed.to_string(), String::new())
}
