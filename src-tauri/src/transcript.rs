use serde::{Deserialize, Serialize};

/// One transcribed stretch of audio (one chunk), times in milliseconds.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    /// Usually the source file name without extension.
    pub title: String,
    pub language: Option<String>,
    pub segments: Vec<Segment>,
}

impl Transcript {
    /// Segments with non-empty text, trimmed.
    pub fn paragraphs(&self) -> impl Iterator<Item = (&Segment, &str)> {
        self.segments
            .iter()
            .map(|s| (s, s.text.trim()))
            .filter(|(_, text)| !text.is_empty())
    }
}
