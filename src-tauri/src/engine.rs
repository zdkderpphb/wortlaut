//! Voxtral via transcribe.cpp: load once, transcribe piece by piece.

use std::path::Path;

pub use transcribe_cpp::CancelToken;
use transcribe_cpp::{Backend, Error as TcError, Model, ModelOptions, RunOptions, Session, SessionOptions, Task};

use crate::chunker::SAMPLE_RATE;
use crate::pieces::Piece;
use crate::transcript::Segment;

/// 30 s of audio is ~375 audio tokens plus a few hundred text tokens.
const CONTEXT_TOKENS: i32 = 2048;
/// Instructions carry text (e.g. a transcript block to summarize).
const INSTRUCT_CONTEXT_TOKENS: i32 = 8192;
/// Pieces quieter than this are skipped: models tend to invent text in silence.
const SILENCE_RMS: f32 = 0.002;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("model could not be loaded: {0}")]
    Load(String),
    #[error("cancelled")]
    Cancelled,
    #[error("transcription failed: {0}")]
    Failed(String),
}

pub struct Engine {
    model: Model,
    session: Session,
    /// Created on first use: summaries need a bigger context.
    instruct_session: Option<Session>,
    backend: String,
}

impl Engine {
    /// Loads on the best device (GPU when possible). If that fails – a broken
    /// graphics driver, too little video memory – it falls back to the CPU:
    /// slower, but it works.
    pub fn load(path: &Path) -> Result<Engine, EngineError> {
        init_backends()?;
        Self::load_on(path, Backend::Auto).or_else(|err| {
            log::warn!("loading on the best device failed ({err}), falling back to CPU");
            Self::load_on(path, Backend::Cpu)
        })
    }

    fn load_on(path: &Path, backend: Backend) -> Result<Engine, EngineError> {
        let load_error = |e: TcError| EngineError::Load(e.to_string());
        let model = Model::load_with(path, &ModelOptions { backend, ..Default::default() }).map_err(load_error)?;
        let session = model
            .session_with(&SessionOptions { n_ctx: CONTEXT_TOKENS, ..Default::default() })
            .map_err(load_error)?;
        let backend = model.backend();
        log::info!("model loaded on {backend}");
        Ok(Engine { model, session, instruct_session: None, backend })
    }

    /// Compute backend in use, e.g. "Metal", "Vulkan" or "CPU".
    pub fn backend_name(&self) -> &str {
        &self.backend
    }

    /// Transcribes the `pieces` of `pcm` (16 kHz mono) one by one. `language`
    /// is an ISO code or `None` for auto-detection. `on_segment` sees each
    /// piece as soon as it is done, with how many ms of audio are finished.
    pub fn transcribe(
        &mut self,
        pcm: &[f32],
        pieces: &[Piece],
        language: Option<&str>,
        cancel: &CancelToken,
        mut on_segment: impl FnMut(&Segment, u64),
    ) -> Result<Vec<Segment>, EngineError> {
        self.session.set_cancel_token(cancel);
        let options = RunOptions { language: language.map(str::to_owned), ..Default::default() };

        let mut segments = Vec::new();
        for piece in pieces {
            if cancel.is_cancelled() {
                return Err(EngineError::Cancelled);
            }
            let text = self.transcribe_chunk(&pcm[piece.start..piece.end], &options)?;
            let segment = Segment { start_ms: piece.start_ms(), end_ms: piece.end_ms(), text, speaker: piece.speaker };
            on_segment(&segment, piece.end_ms());
            segments.push(segment);
        }
        Ok(segments)
    }

    /// Transcribes up to the first minute with auto-detection and returns the
    /// language of that text, if it is one of Voxtral's. Pinning it for the
    /// rest keeps short, unclear pieces from drifting into another language.
    pub fn detect_language(&mut self, pcm: &[f32], cancel: &CancelToken) -> Result<Option<&'static str>, EngineError> {
        self.session.set_cancel_token(cancel);
        let mut text = String::new();
        for chunk in crate::chunker::split(pcm).into_iter().take(2) {
            text.push_str(&self.transcribe_chunk(&pcm[chunk.start..chunk.end], &RunOptions::default())?);
            text.push(' ');
        }
        Ok(crate::language::detect(&text))
    }

    /// Runs a free-text instruction (e.g. "summarize this: …") and returns
    /// the model's answer. Voxtral always takes audio, so it gets a second of
    /// silence alongside the text.
    pub fn instruct(&mut self, prompt: &str) -> Result<String, EngineError> {
        if self.instruct_session.is_none() {
            let session = self
                .model
                .session_with(&SessionOptions { n_ctx: INSTRUCT_CONTEXT_TOKENS, ..Default::default() })
                .map_err(|e| EngineError::Failed(e.to_string()))?;
            self.instruct_session = Some(session);
        }
        let session = self.instruct_session.as_mut().expect("just created");
        let silence = vec![0.0; SAMPLE_RATE];
        let options = RunOptions { task: Task::Instruct, prompt: Some(prompt.to_owned()), ..Default::default() };
        match session.run(&silence, &options) {
            Ok(result) => Ok(result.text.trim().to_owned()),
            Err(TcError::OutputTruncated { partial, .. } | TcError::OutputRepetition { partial, .. }) => {
                Ok(partial.map(|t| t.text.trim().to_owned()).unwrap_or_default())
            }
            Err(e) => Err(EngineError::Failed(e.to_string())),
        }
    }

    fn transcribe_chunk(&mut self, pcm: &[f32], options: &RunOptions) -> Result<String, EngineError> {
        if is_silent(pcm) {
            return Ok(String::new());
        }
        match self.session.run(pcm, options) {
            Ok(result) => Ok(result.text.trim().to_owned()),
            Err(TcError::Aborted { .. }) => Err(EngineError::Cancelled),
            // Incomplete but still useful: keep what the model produced.
            Err(TcError::OutputTruncated { partial, .. } | TcError::OutputRepetition { partial, .. }) => {
                Ok(partial.map(|t| t.text.trim().to_owned()).unwrap_or_default())
            }
            Err(TcError::InputTooShort(_)) => Ok(String::new()),
            Err(e) => Err(EngineError::Failed(e.to_string())),
        }
    }
}

/// Names of the compute devices the engine can use, e.g. "MTL0" or "Vulkan0".
pub fn compute_devices() -> Result<Vec<String>, EngineError> {
    init_backends()?;
    Ok(transcribe_cpp::devices()
        .into_iter()
        .map(|d| format!("{} ({})", d.name, d.description))
        .collect())
}

fn is_silent(pcm: &[f32]) -> bool {
    let mean_square = pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len().max(1) as f32;
    mean_square.sqrt() < SILENCE_RMS
}

/// Windows ships Vulkan as a loadable module next to the exe; elsewhere the
/// backends are compiled in and this is a no-op.
fn init_backends() -> Result<(), EngineError> {
    static INIT: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    INIT.get_or_init(|| transcribe_cpp::init_backends_default().map_err(|e| e.to_string()))
        .clone()
        .map_err(EngineError::Load)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_detected() {
        assert!(is_silent(&vec![0.0; 16_000]));
        assert!(is_silent(&[]));
        let tone: Vec<f32> = (0..16_000).map(|i| 0.1 * (i as f32 * 0.1).sin()).collect();
        assert!(!is_silent(&tone));
    }

    /// Runs only with a real model: `WORTLAUT_MODEL=/path/to/model.gguf cargo test`.
    #[test]
    fn transcribes_german_speech_with_the_real_model() {
        let Ok(model) = std::env::var("WORTLAUT_MODEL") else {
            eprintln!("skipped: set WORTLAUT_MODEL to run");
            return;
        };
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/speech-de.m4a");
        let pcm = crate::audio::decode_to_mono_16k(&fixture).unwrap();
        let pieces = crate::pieces::pieces_without_speakers(&pcm);

        let mut engine = Engine::load(Path::new(&model)).unwrap();
        let mut progress = Vec::new();
        let segments = engine
            .transcribe(&pcm, &pieces, Some("de"), &CancelToken::new(), |_, done| progress.push(done))
            .unwrap();

        let text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        eprintln!("[{}] {text}", engine.backend_name());
        let lower = text.to_lowercase();
        for word in ["guten tag", "transkription", "wien", "kaffee"] {
            assert!(lower.contains(word), "missing {word:?} in {text:?}");
        }
        assert_eq!(progress.last().copied(), segments.last().map(|s| s.end_ms));
        assert_eq!(engine.detect_language(&pcm, &CancelToken::new()).unwrap(), Some("de"));

        let cancelled = CancelToken::new();
        cancelled.cancel();
        let err = engine.transcribe(&pcm, &pieces, None, &cancelled, |_, _| {}).unwrap_err();
        assert!(matches!(err, EngineError::Cancelled));

        // The fallback for machines without a usable GPU.
        let mut cpu = Engine::load_on(Path::new(&model), Backend::Cpu).unwrap();
        assert_eq!(cpu.backend_name(), "CPU");
        let segments = cpu.transcribe(&pcm, &pieces, Some("de"), &CancelToken::new(), |_, _| {}).unwrap();
        assert!(segments[0].text.to_lowercase().contains("wien"), "{segments:?}");
    }
}
