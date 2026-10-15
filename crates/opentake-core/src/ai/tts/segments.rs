//! Segment text splitting and subtitle cue extraction for Text-to-Speech.

use crate::localization::subtitles::model::SubtitleTrack;

/// Chunks raw script text into natural sentence/paragraph units suitable for synthesis.
/// Preserves Unicode grapheme clusters and supports Bengali punctuation (Dari '।').
pub fn chunk_script_text(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    // Punctuation characters that indicate a natural pause/sentence boundary
    let sentence_terminators = ['.', '!', '?', '।', '\n'];

    for ch in trimmed.chars() {
        current.push(ch);

        if sentence_terminators.contains(&ch) {
            let chunk_trimmed = current.trim().to_string();
            if !chunk_trimmed.is_empty() {
                chunks.push(chunk_trimmed);
            }
            current.clear();
        }
    }

    let leftover = current.trim().to_string();
    if !leftover.is_empty() {
        chunks.push(leftover);
    }

    // If no punctuation was present or only one chunk resulted, return whatever was non-empty
    if chunks.is_empty() && !trimmed.is_empty() {
        chunks.push(trimmed.to_string());
    }

    chunks
}

/// Extracted subtitle item ready for synthesis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleCueForTts {
    pub cue_id: String,
    pub text: String,
    pub start_us: u64,
    pub end_us: u64,
}

/// Extracts cues from a SubtitleTrack, optionally filtering by selected cue IDs.
pub fn extract_cues_for_tts(
    track: &SubtitleTrack,
    selected_cue_ids: Option<&[String]>,
) -> Vec<SubtitleCueForTts> {
    track
        .cues
        .iter()
        .filter(|cue| {
            if let Some(ids) = selected_cue_ids {
                ids.contains(&cue.id)
            } else {
                true
            }
        })
        .filter(|cue| !cue.text.trim().is_empty())
        .map(|cue| SubtitleCueForTts {
            cue_id: cue.id.clone(),
            text: cue.text.clone(),
            start_us: cue.start_us.0 as u64,
            end_us: cue.end_us.0 as u64,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_script_text_english() {
        let input = "Hello world! This is Open Media Workbench. Welcome aboard.";
        let chunks = chunk_script_text(input);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "Hello world!");
        assert_eq!(chunks[1], "This is Open Media Workbench.");
        assert_eq!(chunks[2], "Welcome aboard.");
    }

    #[test]
    fn test_chunk_script_text_bengali() {
        let input = "ওপেন মিডিয়া ওয়ার্কবেঞ্চে স্বাগতম। এটি একটি অফলাইন টুল।";
        let chunks = chunk_script_text(input);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], "ওপেন মিডিয়া ওয়ার্কবেঞ্চে স্বাগতম।");
        assert_eq!(chunks[1], "এটি একটি অফলাইন টুল।");
    }

    #[test]
    fn test_chunk_script_empty() {
        assert!(chunk_script_text("").is_empty());
        assert!(chunk_script_text("   \n\n  ").is_empty());
    }
}
