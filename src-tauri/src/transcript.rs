use serde::{Deserialize, Serialize};

/// One transcribed stretch of audio, times in milliseconds.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    /// Index into `Transcript::speakers`, when speakers were detected.
    #[serde(default)]
    pub speaker: Option<usize>,
}

/// A finished transcript as shown, saved in the history and exported.
/// Fields added after 0.1 default, so older files still load.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    #[serde(default)]
    pub id: String,
    /// Usually the source file name without extension.
    pub title: String,
    pub language: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    /// Unix seconds.
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub duration_ms: u64,
    /// Display names, e.g. "Sprecher 1" – renamed by the user.
    #[serde(default)]
    pub speakers: Vec<String>,
    #[serde(default)]
    pub summary: Option<String>,
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

    pub fn speaker_name(&self, segment: &Segment) -> Option<&str> {
        segment.speaker.and_then(|i| self.speakers.get(i)).map(String::as_str)
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }
}
