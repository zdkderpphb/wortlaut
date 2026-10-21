//! The bridge between the window and the Rust core: commands and events.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::audio::{self, AudioError};
use crate::chunker::SAMPLE_RATE;
use crate::diarize::{self, DiarizeError};
use crate::engine::{CancelToken, Engine, EngineError};
use crate::export::{self, ExportLabels, Format};
use crate::history::{self, HistoryError, TranscriptMeta};
use crate::model_manager::{self, DownloadError, ModelSpec, ModelStatus};
use crate::pieces::{self, Piece};
use crate::summary;
use crate::transcript::{Segment, Transcript};

pub struct AppState {
    models_dir: PathBuf,
    history_dir: PathBuf,
    specs: Vec<ModelSpec>,
    engine: Mutex<Option<Engine>>,
    jobs: Mutex<HashMap<String, CancelToken>>,
    download_cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl AppState {
    pub fn new(data_dir: &Path) -> AppState {
        AppState {
            models_dir: data_dir.join("models"),
            history_dir: data_dir.join("transcripts"),
            specs: ModelSpec::all(),
            engine: Mutex::new(None),
            jobs: Mutex::new(HashMap::new()),
            download_cancel: Mutex::new(None),
        }
    }

    fn model_path(&self, spec: &ModelSpec) -> PathBuf {
        spec.path_in(&self.models_dir)
    }

    /// The engine, loaded on first use and kept for the rest of the session.
    fn engine(&self, on_load: impl FnOnce()) -> CommandResult<MutexGuard<'_, Option<Engine>>> {
        let mut engine = self.engine.lock().unwrap();
        if engine.is_none() {
            if model_manager::status_all(&self.models_dir, &self.specs) != ModelStatus::Ready {
                return Err(CommandError::new("model_missing", "speech models not downloaded"));
            }
            on_load();
            *engine = Some(Engine::load(&self.model_path(&ModelSpec::voxtral_mini()))?);
        }
        Ok(engine)
    }
}

/// What the window gets when something goes wrong. `code` picks the
/// friendly message; `detail` is shown under "Technische Details".
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    detail: String,
}

impl CommandError {
    fn new(code: &'static str, detail: impl ToString) -> CommandError {
        let detail = detail.to_string();
        if code != "cancelled" {
            log::error!("{code}: {detail}");
        }
        CommandError { code, detail }
    }
}

impl From<AudioError> for CommandError {
    fn from(e: AudioError) -> Self {
        let code = match e {
            AudioError::Unsupported(_) => "unsupported_format",
            AudioError::NoAudioTrack => "no_audio",
            AudioError::Io(_) => "file_read",
        };
        CommandError::new(code, e)
    }
}

impl From<EngineError> for CommandError {
    fn from(e: EngineError) -> Self {
        let code = match e {
            EngineError::Load(_) => "model_load",
            EngineError::Cancelled => "cancelled",
            EngineError::Failed(_) => "transcription_failed",
        };
        CommandError::new(code, e)
    }
}

impl From<DownloadError> for CommandError {
    fn from(e: DownloadError) -> Self {
        let code = match e {
            DownloadError::Network(_) => "network",
            DownloadError::Checksum => "checksum",
            DownloadError::NoSpace { .. } => "no_space",
            DownloadError::Cancelled => "cancelled",
            DownloadError::Io(_) => "file_write",
        };
        CommandError::new(code, e)
    }
}

impl From<DiarizeError> for CommandError {
    fn from(e: DiarizeError) -> Self {
        CommandError::new("diarization_failed", e)
    }
}

impl From<HistoryError> for CommandError {
    fn from(e: HistoryError) -> Self {
        CommandError::new("history_failed", e)
    }
}

type CommandResult<T> = Result<T, CommandError>;

fn now() -> std::time::Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
}

// ---------------------------------------------------------------- models

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    status: ModelStatus,
    size_bytes: u64,
    free_bytes: u64,
}

#[tauri::command]
pub fn model_info(state: State<'_, AppState>) -> ModelInfo {
    ModelInfo {
        status: model_manager::status_all(&state.models_dir, &state.specs),
        size_bytes: model_manager::total_size(&state.specs),
        free_bytes: model_manager::free_space(&state.models_dir),
    }
}

#[tauri::command]
pub async fn download_model(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let cancel = Arc::new(AtomicBool::new(false));
    *state.download_cancel.lock().unwrap() = Some(cancel.clone());
    log::info!("model download started");
    let result = model_manager::download_all(&state.models_dir, &state.specs, cancel, |progress| {
        let _ = app.emit("download-progress", progress);
    })
    .await;
    *state.download_cancel.lock().unwrap() = None;
    result.map(|()| log::info!("models ready in {}", state.models_dir.display())).map_err(Into::into)
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    if let Some(flag) = state.download_cancel.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

// --------------------------------------------------------- transcription

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeOptions {
    /// ISO code or "auto".
    language: String,
    detect_speakers: bool,
    /// Fixed number of speakers, or `None` to let the clustering decide.
    speaker_count: Option<u32>,
    /// "Sprecher" / "Speaker" – default names become "Sprecher 1", …
    speaker_label: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobProgress {
    job_id: String,
    /// "reading" | "diarizing" | "loading" | "transcribing"
    stage: &'static str,
    done_ms: u64,
    total_ms: u64,
    segment: Option<Segment>,
}

/// Transcribes a file, saves the result in the history and returns it.
#[tauri::command]
pub async fn transcribe(
    app: AppHandle,
    job_id: String,
    path: String,
    options: TranscribeOptions,
) -> CommandResult<Transcript> {
    let token = CancelToken::new();
    let state = app.state::<AppState>();
    state.jobs.lock().unwrap().insert(job_id.clone(), token.clone());

    let worker_app = app.clone();
    let worker_job = job_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        run_job(&worker_app, &worker_job, Path::new(&path), &options, &token)
    })
    .await
    .map_err(|e| CommandError::new("transcription_failed", e));

    state.jobs.lock().unwrap().remove(&job_id);
    result?
}

fn run_job(
    app: &AppHandle,
    job_id: &str,
    path: &Path,
    options: &TranscribeOptions,
    token: &CancelToken,
) -> CommandResult<Transcript> {
    let state = app.state::<AppState>();
    let progress = |stage, done_ms, total_ms, segment| {
        let job_id = job_id.to_owned();
        let _ = app.emit("job-progress", JobProgress { job_id, stage, done_ms, total_ms, segment });
    };

    progress("reading", 0, 0, None);
    log::info!("transcribing {} (speakers: {})", path.display(), options.detect_speakers);
    let pcm = audio::decode_to_mono_16k(path)?;
    let total_ms = (pcm.len() * 1000 / SAMPLE_RATE) as u64;

    let pieces = if options.detect_speakers {
        progress("diarizing", 0, total_ms, None);
        let turns = diarize::detect_speakers(
            &state.model_path(&ModelSpec::speaker_segmentation()),
            &state.model_path(&ModelSpec::speaker_embedding()),
            &pcm,
            options.speaker_count,
        )?;
        let pieces = pieces::pieces_from_turns(&turns, &pcm);
        // No speech found by diarization: still transcribe, just without speakers.
        if pieces.is_empty() { pieces::pieces_without_speakers(&pcm) } else { pieces }
    } else {
        pieces::pieces_without_speakers(&pcm)
    };
    if token.is_cancelled() {
        return Err(EngineError::Cancelled.into());
    }

    let mut engine = state.engine(|| progress("loading", 0, total_ms, None))?;
    let engine = engine.as_mut().expect("engine is loaded");
    progress("transcribing", 0, total_ms, None);
    let language = (options.language != "auto").then_some(options.language.as_str());
    let segments = engine.transcribe(&pcm, &pieces, language, token, |segment, done_ms| {
        progress("transcribing", done_ms, total_ms, Some(segment.clone()));
    })?;

    let transcript = Transcript {
        id: history::new_id(now().as_millis()),
        title: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        language: language.map(str::to_owned),
        source_path: Some(path.to_string_lossy().into_owned()),
        created_at: now().as_secs(),
        duration_ms: total_ms,
        speakers: speaker_names(&pieces, &options.speaker_label),
        summary: None,
        segments,
    };
    if let Err(e) = history::save(&state.history_dir, &transcript) {
        log::error!("could not save transcript {}: {e}", transcript.id);
    }
    Ok(transcript)
}

fn speaker_names(pieces: &[Piece], label: &str) -> Vec<String> {
    let count = pieces.iter().filter_map(|p| p.speaker).max().map_or(0, |max| max + 1);
    (1..=count).map(|n| format!("{label} {n}")).collect()
}

#[tauri::command]
pub fn cancel_transcription(state: State<'_, AppState>, job_id: String) {
    if let Some(token) = state.jobs.lock().unwrap().get(&job_id) {
        token.cancel();
    }
}

// --------------------------------------------------------------- history

#[tauri::command]
pub fn history_list(state: State<'_, AppState>) -> Vec<TranscriptMeta> {
    history::list(&state.history_dir)
}

#[tauri::command]
pub fn history_load(state: State<'_, AppState>, id: String) -> CommandResult<Transcript> {
    Ok(history::load(&state.history_dir, &id)?)
}

#[tauri::command]
pub fn history_save(state: State<'_, AppState>, transcript: Transcript) -> CommandResult<()> {
    Ok(history::save(&state.history_dir, &transcript)?)
}

#[tauri::command]
pub fn history_delete(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(history::delete(&state.history_dir, &id)?)
}

// --------------------------------------------------------------- summary

#[derive(Clone, Serialize)]
struct SummaryProgress {
    id: String,
    done: usize,
    total: usize,
}

#[tauri::command]
pub async fn summarize(app: AppHandle, transcript: Transcript, locale: String) -> CommandResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut engine = state.engine(|| {})?;
        let engine = engine.as_mut().expect("engine is loaded");
        summary::summarize(engine, &transcript, &locale, |done, total| {
            let _ = app.emit("summary-progress", SummaryProgress { id: transcript.id.clone(), done, total });
        })
        .map_err(|e| CommandError::new("summary_failed", e))
    })
    .await
    .map_err(|e| CommandError::new("summary_failed", e))?
}

// ---------------------------------------------------------------- export

#[tauri::command]
pub fn export_transcript(
    transcript: Transcript,
    format: Format,
    timestamps: bool,
    labels: ExportLabels,
    path: String,
) -> CommandResult<()> {
    let path = Path::new(&path);
    export::write(&transcript, format, timestamps, &labels, path).map_err(|e| CommandError::new("export_failed", e))?;
    log::info!("exported {format:?} to {}", path.display());
    Ok(())
}

/// Writes every format into `dir`; returns the file names.
#[tauri::command]
pub fn export_all(transcript: Transcript, timestamps: bool, labels: ExportLabels, dir: String) -> CommandResult<Vec<String>> {
    let safe_title: String = transcript
        .title
        .chars()
        .map(|c| if r#"/\:*?"<>|"#.contains(c) { '_' } else { c })
        .collect();
    let mut written = Vec::new();
    for format in Format::ALL {
        let name = format!("{safe_title}{}.{}", format.file_suffix(), format.extension());
        export::write(&transcript, format, timestamps, &labels, &Path::new(&dir).join(&name))
            .map_err(|e| CommandError::new("export_failed", e))?;
        written.push(name);
    }
    log::info!("exported all formats to {dir}");
    Ok(written)
}

// -------------------------------------------------------------- playback

/// Lets the window play `path` through the asset protocol. Only files the
/// user transcribed are ever allowed.
#[tauri::command]
pub fn allow_playback(app: AppHandle, path: String) -> bool {
    let path = Path::new(&path);
    path.is_file() && app.asset_protocol_scope().allow_file(path).is_ok()
}

// ------------------------------------------------------------------ misc

#[tauri::command]
pub fn open_log_dir(app: AppHandle) -> CommandResult<()> {
    let dir = app.path().app_log_dir().map_err(|e| CommandError::new("file_read", e))?;
    std::fs::create_dir_all(&dir).map_err(|e| CommandError::new("file_read", e))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| CommandError::new("file_read", e))
}

#[tauri::command]
pub fn reveal_file(app: AppHandle, path: String) -> CommandResult<()> {
    app.opener().reveal_item_in_dir(path).map_err(|e| CommandError::new("file_read", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speakers_get_numbered_default_names() {
        let piece = |speaker| Piece { start: 0, end: 1, speaker };
        assert_eq!(speaker_names(&[piece(Some(0)), piece(Some(2)), piece(Some(1))], "Sprecher"), [
            "Sprecher 1",
            "Sprecher 2",
            "Sprecher 3"
        ]);
        assert!(speaker_names(&[piece(None)], "Sprecher").is_empty());
    }
}
