//! The bridge between the window and the Rust core: commands and events.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::audio::{self, AudioError};
use crate::chunker::SAMPLE_RATE;
use crate::engine::{CancelToken, Engine, EngineError};
use crate::export;
use crate::model_manager::{self, DownloadError, ModelSpec, ModelStatus};
use crate::transcript::{Segment, Transcript};

pub struct AppState {
    models_dir: PathBuf,
    spec: ModelSpec,
    engine: Mutex<Option<Engine>>,
    jobs: Mutex<HashMap<String, CancelToken>>,
    download_cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl AppState {
    pub fn new(models_dir: PathBuf) -> AppState {
        AppState {
            models_dir,
            spec: ModelSpec::voxtral_mini(),
            engine: Mutex::new(None),
            jobs: Mutex::new(HashMap::new()),
            download_cancel: Mutex::new(None),
        }
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

type CommandResult<T> = Result<T, CommandError>;

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
        status: model_manager::status(&state.models_dir, &state.spec),
        size_bytes: state.spec.size,
        free_bytes: model_manager::free_space(&state.models_dir),
    }
}

#[tauri::command]
pub async fn download_model(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let cancel = Arc::new(AtomicBool::new(false));
    *state.download_cancel.lock().unwrap() = Some(cancel.clone());
    log::info!("model download started");
    let result = model_manager::download(&state.models_dir, &state.spec, cancel, |progress| {
        let _ = app.emit("download-progress", progress);
    })
    .await;
    *state.download_cancel.lock().unwrap() = None;
    result.map(|path| log::info!("model ready at {}", path.display())).map_err(Into::into)
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    if let Some(flag) = state.download_cancel.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobProgress {
    job_id: String,
    /// "reading" | "loading" | "transcribing"
    stage: &'static str,
    done_ms: u64,
    total_ms: u64,
    segment: Option<Segment>,
}

#[tauri::command]
pub async fn transcribe(
    app: AppHandle,
    job_id: String,
    path: String,
    language: String,
) -> CommandResult<Transcript> {
    let token = CancelToken::new();
    let state = app.state::<AppState>();
    state.jobs.lock().unwrap().insert(job_id.clone(), token.clone());

    let worker_app = app.clone();
    let worker_job = job_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        run_job(&worker_app, &worker_job, Path::new(&path), &language, &token)
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
    language: &str,
    token: &CancelToken,
) -> CommandResult<Transcript> {
    let state = app.state::<AppState>();
    let progress = |stage, done_ms, total_ms, segment| {
        let job_id = job_id.to_owned();
        let _ = app.emit("job-progress", JobProgress { job_id, stage, done_ms, total_ms, segment });
    };

    progress("reading", 0, 0, None);
    log::info!("transcribing {}", path.display());
    let pcm = audio::decode_to_mono_16k(path)?;
    let total_ms = (pcm.len() * 1000 / SAMPLE_RATE) as u64;

    let mut engine = state.engine.lock().unwrap();
    if engine.is_none() {
        if model_manager::status(&state.models_dir, &state.spec) != ModelStatus::Ready {
            return Err(CommandError::new("model_missing", "speech model not downloaded"));
        }
        progress("loading", 0, total_ms, None);
        *engine = Some(Engine::load(&state.spec.path_in(&state.models_dir))?);
    }
    let engine = engine.as_mut().expect("engine was just loaded");

    progress("transcribing", 0, total_ms, None);
    let language = (language != "auto").then_some(language);
    let segments = engine.transcribe(&pcm, language, token, |segment, done_ms| {
        progress("transcribing", done_ms, total_ms, Some(segment.clone()));
    })?;

    let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Transcript { title, language: language.map(str::to_owned), segments })
}

#[tauri::command]
pub fn cancel_transcription(state: State<'_, AppState>, job_id: String) {
    if let Some(token) = state.jobs.lock().unwrap().get(&job_id) {
        token.cancel();
    }
}

#[tauri::command]
pub fn export_transcript(
    transcript: Transcript,
    format: String,
    timestamps: bool,
    path: String,
) -> CommandResult<()> {
    let path = Path::new(&path);
    let written = match format.as_str() {
        "txt" => std::fs::write(path, export::to_txt(&transcript, timestamps)).map_err(anyhow::Error::from),
        "md" => std::fs::write(path, export::to_markdown(&transcript, timestamps)).map_err(anyhow::Error::from),
        "docx" => export::write_docx(&transcript, timestamps, path),
        other => Err(anyhow::anyhow!("unknown format {other}")),
    };
    written.map_err(|e| CommandError::new("export_failed", e))?;
    log::info!("exported {format} to {}", path.display());
    Ok(())
}

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
