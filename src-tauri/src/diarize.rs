//! Who spoke when: pyannote segmentation + CAM++ speaker embeddings,
//! clustered by sherpa-onnx. Works for any number of speakers.

use std::path::Path;

use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig,
};

use crate::chunker::SAMPLE_RATE;
use crate::pieces::Turn;

/// Clustering distance used when the number of speakers is not given.
const THRESHOLD: f32 = 0.5;

#[derive(Debug, thiserror::Error)]
pub enum DiarizeError {
    #[error("speaker detection could not start (models missing or damaged)")]
    Init,
    #[error("speaker detection failed")]
    Failed,
}

/// Finds speaker turns in 16 kHz mono audio. `speakers` fixes the number
/// of speakers; `None` lets the clustering decide.
pub fn detect_speakers(
    segmentation_model: &Path,
    embedding_model: &Path,
    pcm: &[f32],
    speakers: Option<u32>,
) -> Result<Vec<Turn>, DiarizeError> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8)) as i32;
    let config = OfflineSpeakerDiarizationConfig {
        segmentation: OfflineSpeakerSegmentationModelConfig {
            pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                model: Some(segmentation_model.to_string_lossy().into_owned()),
                window_shift_ratio: 0.1,
            },
            num_threads: threads,
            ..Default::default()
        },
        embedding: SpeakerEmbeddingExtractorConfig {
            model: Some(embedding_model.to_string_lossy().into_owned()),
            num_threads: threads,
            ..Default::default()
        },
        clustering: FastClusteringConfig {
            num_clusters: speakers.map_or(-1, |n| n as i32),
            threshold: THRESHOLD,
            ..Default::default()
        },
        ..Default::default()
    };
    let diarizer = OfflineSpeakerDiarization::create(&config).ok_or(DiarizeError::Init)?;
    let result = diarizer.process(pcm).ok_or(DiarizeError::Failed)?;

    let to_sample = |secs: f32| (secs.max(0.0) * SAMPLE_RATE as f32) as usize;
    Ok(result
        .sort_by_start_time()
        .into_iter()
        .map(|s| Turn { start: to_sample(s.start), end: to_sample(s.end), speaker: s.speaker as usize })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pieces::pieces_from_turns;

    /// Runs only with the models: `WORTLAUT_DIARIZE_MODELS=<folder with both .onnx files>`.
    #[test]
    fn tells_the_three_speakers_of_the_dialog_apart() {
        let Ok(dir) = std::env::var("WORTLAUT_DIARIZE_MODELS") else {
            eprintln!("skipped: set WORTLAUT_DIARIZE_MODELS to run");
            return;
        };
        let dir = Path::new(&dir);
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dialog-de.m4a");
        let pcm = crate::audio::decode_to_mono_16k(&fixture).unwrap();

        let turns = detect_speakers(
            &dir.join("pyannote-segmentation-3.0.onnx"),
            &dir.join("campplus-speaker-embedding.onnx"),
            &pcm,
            None,
        )
        .unwrap();
        let pieces = pieces_from_turns(&turns, &pcm);
        let speakers: std::collections::BTreeSet<_> = pieces.iter().filter_map(|p| p.speaker).collect();
        assert_eq!(speakers.len(), 3, "{pieces:?}");
        // The first two contributions are Anna, then Flo.
        assert_eq!((pieces[0].speaker, pieces[1].speaker), (Some(0), Some(1)));

        let two = detect_speakers(
            &dir.join("pyannote-segmentation-3.0.onnx"),
            &dir.join("campplus-speaker-embedding.onnx"),
            &pcm,
            Some(2),
        )
        .unwrap();
        let fixed: std::collections::BTreeSet<_> = two.iter().map(|t| t.speaker).collect();
        assert!(fixed.len() <= 2, "{two:?}");
    }

    #[test]
    fn missing_models_are_reported_not_panicked() {
        let missing = Path::new("/nonexistent/model.onnx");
        let err = detect_speakers(missing, missing, &[0.0; 16_000], None).unwrap_err();
        assert!(matches!(err, DiarizeError::Init));
    }
}
