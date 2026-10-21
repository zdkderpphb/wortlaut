//! Every finished transcript is kept as `<id>.json`, so nothing is lost when
//! the app closes. Edits overwrite the file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use crate::transcript::Transcript;

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("invalid transcript id")]
    InvalidId,
    #[error("transcript not found")]
    NotFound,
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("damaged file: {0}")]
    Damaged(#[from] serde_json::Error),
}

/// What the list on the left needs, without loading every segment.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub duration_ms: u64,
    pub speaker_count: usize,
    pub source_path: Option<String>,
}

/// `<created ms>-<4 hex digits>`: sortable, unique within a session, and
/// safe to use as a file name.
pub fn new_id(created_ms: u128) -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let salt = (COUNTER.fetch_add(1, Ordering::Relaxed) ^ std::process::id()) & 0xffff;
    format!("{created_ms}-{salt:04x}")
}

fn path_for(dir: &Path, id: &str) -> Result<PathBuf, HistoryError> {
    let valid = !id.is_empty() && id.len() <= 40 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if !valid {
        return Err(HistoryError::InvalidId);
    }
    Ok(dir.join(format!("{id}.json")))
}

pub fn save(dir: &Path, transcript: &Transcript) -> Result<(), HistoryError> {
    let path = path_for(dir, &transcript.id)?;
    std::fs::create_dir_all(dir)?;
    // Write next to it, then rename: a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(transcript)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn load(dir: &Path, id: &str) -> Result<Transcript, HistoryError> {
    let path = path_for(dir, id)?;
    let bytes = std::fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => HistoryError::NotFound,
        _ => HistoryError::Io(e),
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn delete(dir: &Path, id: &str) -> Result<(), HistoryError> {
    match std::fs::remove_file(path_for(dir, id)?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(HistoryError::Io(e)),
        _ => Ok(()),
    }
}

/// Newest first. Damaged files are skipped (and logged), not fatal.
pub fn list(dir: &Path) -> Vec<TranscriptMeta> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut metas: Vec<TranscriptMeta> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter_map(|path| {
            let parsed = std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|b| serde_json::from_slice::<Transcript>(&b).map_err(|e| e.to_string()));
            match parsed {
                Ok(t) => Some(TranscriptMeta {
                    speaker_count: t.speakers.len(),
                    id: t.id,
                    title: t.title,
                    created_at: t.created_at,
                    duration_ms: t.duration_ms,
                    source_path: t.source_path,
                }),
                Err(e) => {
                    log::warn!("skipping {}: {e}", path.display());
                    None
                }
            }
        })
        .collect();
    metas.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| b.id.cmp(&a.id)));
    metas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::Segment;

    fn transcript(id: &str, title: &str, created_at: u64) -> Transcript {
        Transcript {
            id: id.into(),
            title: title.into(),
            created_at,
            duration_ms: 5_000,
            speakers: vec!["Anna".into()],
            segments: vec![Segment { start_ms: 0, end_ms: 5_000, text: "Hallo".into(), speaker: Some(0) }],
            ..Default::default()
        }
    }

    #[test]
    fn ids_are_unique_and_file_safe() {
        let a = new_id(1_792_000_000_000);
        let b = new_id(1_792_000_000_000);
        assert_ne!(a, b);
        assert!(path_for(Path::new("/x"), &a).is_ok());
    }

    #[test]
    fn saves_loads_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = transcript("1-aa", "Interview", 10);
        save(dir.path(), &t).unwrap();
        assert_eq!(load(dir.path(), "1-aa").unwrap(), t);

        t.speakers[0] = "Interviewerin".into();
        save(dir.path(), &t).unwrap();
        assert_eq!(load(dir.path(), "1-aa").unwrap().speakers, vec!["Interviewerin"]);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(leftovers.len(), 1, "{leftovers:?}");
    }

    #[test]
    fn lists_newest_first_and_skips_damaged_files() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &transcript("1-aa", "Alt", 10)).unwrap();
        save(dir.path(), &transcript("2-bb", "Neu", 20)).unwrap();
        std::fs::write(dir.path().join("3-cc.json"), "{ kaputt").unwrap();

        let list = list(dir.path());
        let titles: Vec<_> = list.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, vec!["Neu", "Alt"]);
        assert_eq!(list[0].speaker_count, 1);
    }

    #[test]
    fn deletes_and_reports_missing() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &transcript("1-aa", "Weg", 10)).unwrap();
        delete(dir.path(), "1-aa").unwrap();
        delete(dir.path(), "1-aa").unwrap(); // second time is fine
        assert!(matches!(load(dir.path(), "1-aa"), Err(HistoryError::NotFound)));
    }

    #[test]
    fn refuses_ids_that_could_escape_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        for id in ["../secret", "a/b", "", "x.json", "..\\\\evil"] {
            assert!(matches!(load(dir.path(), id), Err(HistoryError::InvalidId)), "{id}");
            assert!(matches!(delete(dir.path(), id), Err(HistoryError::InvalidId)), "{id}");
        }
        assert!(list(Path::new("/does/not/exist")).is_empty());
    }
}
