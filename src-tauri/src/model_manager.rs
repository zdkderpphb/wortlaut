//! Finds, downloads (resumable) and verifies the speech model.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

/// Keep this much disk free beyond the model itself.
const SPACE_MARGIN: u64 = 200 * 1024 * 1024;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, Debug)]
pub struct ModelSpec {
    pub url: String,
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
}

impl ModelSpec {
    pub fn voxtral_mini() -> ModelSpec {
        ModelSpec {
            url: "https://huggingface.co/handy-computer/Voxtral-Mini-3B-2507-GGUF/resolve/main/Voxtral-Mini-3B-2507-Q4_K_M.gguf".into(),
            file_name: "Voxtral-Mini-3B-2507-Q4_K_M.gguf".into(),
            size: 2_984_721_056,
            sha256: "3a6717aa8f8989108d260cbd237584289eec43cc987e10133c33643515936205".into(),
        }
    }

    pub fn path_in(&self, dir: &Path) -> PathBuf {
        dir.join(&self.file_name)
    }

    fn part_path_in(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.part", self.file_name))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ModelStatus {
    Missing,
    Partial { bytes: u64 },
    Ready,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    pub bytes_per_sec: f64,
    /// True while the finished file is being checked.
    pub verifying: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("network error: {0}")]
    Network(String),
    #[error("checksum mismatch")]
    Checksum,
    #[error("not enough disk space: need {needed} bytes, {free} free")]
    NoSpace { needed: u64, free: u64 },
    #[error("cancelled")]
    Cancelled,
    #[error("file error: {0}")]
    Io(String),
}

impl From<std::io::Error> for DownloadError {
    fn from(e: std::io::Error) -> Self {
        DownloadError::Io(e.to_string())
    }
}

/// Cheap check by size only; the checksum was verified when the file was
/// moved into place.
pub fn status(dir: &Path, spec: &ModelSpec) -> ModelStatus {
    let len = |p: PathBuf| std::fs::metadata(p).map(|m| m.len()).ok();
    if len(spec.path_in(dir)) == Some(spec.size) {
        return ModelStatus::Ready;
    }
    match len(spec.part_path_in(dir)) {
        Some(bytes) if bytes > 0 => ModelStatus::Partial { bytes },
        _ => ModelStatus::Missing,
    }
}

/// Free bytes on the disk that holds `dir` (or its nearest existing parent).
pub fn free_space(dir: &Path) -> u64 {
    dir.ancestors()
        .find(|p| p.exists())
        .and_then(|p| fs4::available_space(p).ok())
        .unwrap_or(0)
}

pub async fn download(
    dir: &Path,
    spec: &ModelSpec,
    cancel: Arc<AtomicBool>,
    mut on_progress: impl FnMut(Progress),
) -> Result<PathBuf, DownloadError> {
    let part = spec.part_path_in(dir);
    let mut have = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if have > spec.size {
        std::fs::remove_file(&part)?;
        have = 0;
    }

    let needed = spec.size.saturating_sub(have).saturating_add(SPACE_MARGIN);
    let free = free_space(dir);
    if free < needed {
        return Err(DownloadError::NoSpace { needed, free });
    }
    std::fs::create_dir_all(dir)?;

    if have < spec.size {
        have = fetch(spec, &part, have, &cancel, &mut on_progress).await?;
    }
    if have != spec.size {
        return Err(DownloadError::Network(format!("incomplete: {have} of {} bytes", spec.size)));
    }

    on_progress(Progress { downloaded: have, total: spec.size, bytes_per_sec: 0.0, verifying: true });
    let part_for_hash = part.clone();
    let digest = tokio::task::spawn_blocking(move || sha256_file(&part_for_hash))
        .await
        .map_err(|e| DownloadError::Io(e.to_string()))??;
    if !digest.eq_ignore_ascii_case(&spec.sha256) {
        std::fs::remove_file(&part)?;
        return Err(DownloadError::Checksum);
    }

    let target = spec.path_in(dir);
    std::fs::rename(&part, &target)?;
    Ok(target)
}

/// Streams the rest of the file into `part`, starting at byte `have`.
/// Returns how many bytes `part` holds afterwards.
async fn fetch(
    spec: &ModelSpec,
    part: &Path,
    mut have: u64,
    cancel: &AtomicBool,
    on_progress: &mut impl FnMut(Progress),
) -> Result<u64, DownloadError> {
    let network = |e: reqwest::Error| DownloadError::Network(e.to_string());
    let client = reqwest::Client::builder()
        .user_agent(concat!("Wortlaut/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        // The default 64 KB HTTP/2 window caps a big download at a few
        // hundred KB/s for the first seconds.
        .http2_adaptive_window(true)
        .build()
        .map_err(network)?;

    let mut request = client.get(&spec.url);
    if have > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let response = request.send().await.map_err(network)?;
    let resumed = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    if !response.status().is_success() {
        return Err(DownloadError::Network(format!("HTTP {}", response.status())));
    }
    if !resumed {
        // Server ignored the range: start over.
        have = 0;
    }

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(part)
        .await?;

    let started = Instant::now();
    let start_bytes = have;
    let mut last_report: Option<Instant> = None;
    let mut stream = response.bytes_stream();
    while let Some(piece) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            file.flush().await?;
            return Err(DownloadError::Cancelled);
        }
        let piece = piece.map_err(network)?;
        file.write_all(&piece).await?;
        have += piece.len() as u64;

        if last_report.is_none_or(|t| t.elapsed() >= PROGRESS_INTERVAL) {
            last_report = Some(Instant::now());
            let secs = started.elapsed().as_secs_f64().max(0.001);
            on_progress(Progress {
                downloaded: have,
                total: spec.size,
                bytes_per_sec: (have - start_bytes) as f64 / secs,
                verifying: false,
            });
        }
    }
    file.flush().await?;
    Ok(have)
}

fn sha256_file(path: &Path) -> Result<String, DownloadError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;

    /// Minimal HTTP server: serves `body`, honours `Range: bytes=N-`,
    /// records every Range header it saw.
    struct TestServer {
        url: String,
        ranges: Arc<Mutex<Vec<Option<String>>>>,
    }

    fn serve(body: Vec<u8>) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model.gguf", listener.local_addr().unwrap());
        let ranges = Arc::new(Mutex::new(Vec::new()));
        let seen = ranges.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut range = None;
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("range:") {
                        range = Some(v.trim().to_owned());
                    }
                }
                seen.lock().unwrap().push(range.clone());
                let start: usize = range
                    .as_deref()
                    .and_then(|r| r.strip_prefix("bytes="))
                    .and_then(|r| r.trim_end_matches('-').parse().ok())
                    .unwrap_or(0);
                let head = if range.is_some() {
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{}/{}\r\n",
                        body.len() - 1,
                        body.len()
                    )
                } else {
                    "HTTP/1.1 200 OK\r\n".to_owned()
                };
                let part = &body[start..];
                let _ = write!(stream, "{head}Content-Length: {}\r\nConnection: close\r\n\r\n", part.len());
                for piece in part.chunks(16 * 1024) {
                    if stream.write_all(piece).is_err() {
                        break;
                    }
                }
            }
        });
        TestServer { url, ranges }
    }

    fn body() -> Vec<u8> {
        (0..2_000_000u32).map(|i| (i % 251) as u8).collect()
    }

    fn spec_for(server: &TestServer, body: &[u8]) -> ModelSpec {
        ModelSpec {
            url: server.url.clone(),
            file_name: "model.gguf".into(),
            size: body.len() as u64,
            sha256: hex(&Sha256::digest(body)),
        }
    }

    #[tokio::test]
    async fn downloads_verifies_and_reports_ready() {
        let body = body();
        let server = serve(body.clone());
        let spec = spec_for(&server, &body);
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(status(dir.path(), &spec), ModelStatus::Missing);

        let mut last = None;
        let path = download(dir.path(), &spec, Arc::default(), |p| last = Some(p)).await.unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert_eq!(status(dir.path(), &spec), ModelStatus::Ready);
        assert_eq!(last.unwrap().downloaded, body.len() as u64);
        assert!(!dir.path().join("model.gguf.part").exists());
    }

    #[tokio::test]
    async fn resumes_a_partial_download() {
        let body = body();
        let server = serve(body.clone());
        let spec = spec_for(&server, &body);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("model.gguf.part"), &body[..700_000]).unwrap();
        assert_eq!(status(dir.path(), &spec), ModelStatus::Partial { bytes: 700_000 });

        let path = download(dir.path(), &spec, Arc::default(), |_| {}).await.unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert_eq!(*server.ranges.lock().unwrap(), vec![Some("bytes=700000-".to_owned())]);
    }

    #[tokio::test]
    async fn rejects_a_corrupt_download_and_removes_it() {
        let body = body();
        let server = serve(body.clone());
        let mut spec = spec_for(&server, &body);
        spec.sha256 = "0".repeat(64);
        let dir = tempfile::tempdir().unwrap();

        let err = download(dir.path(), &spec, Arc::default(), |_| {}).await.unwrap_err();

        assert!(matches!(err, DownloadError::Checksum), "{err:?}");
        assert_eq!(status(dir.path(), &spec), ModelStatus::Missing);
    }

    #[tokio::test]
    async fn cancelling_keeps_the_partial_file() {
        let body = body();
        let server = serve(body.clone());
        let spec = spec_for(&server, &body);
        let dir = tempfile::tempdir().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();

        let err = download(dir.path(), &spec, cancel, |_| flag.store(true, Ordering::SeqCst))
            .await
            .unwrap_err();

        assert!(matches!(err, DownloadError::Cancelled), "{err:?}");
        assert!(matches!(status(dir.path(), &spec), ModelStatus::Partial { .. } | ModelStatus::Missing));
    }

    #[test]
    fn refuses_to_start_without_enough_disk_space() {
        let dir = tempfile::tempdir().unwrap();
        let mut spec = ModelSpec::voxtral_mini();
        spec.size = u64::MAX / 2;
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let err = runtime.block_on(download(dir.path(), &spec, Arc::default(), |_| {})).unwrap_err();
        assert!(matches!(err, DownloadError::NoSpace { .. }), "{err:?}");
    }

    #[test]
    fn free_space_is_known_even_for_folders_that_do_not_exist_yet() {
        let dir = tempfile::tempdir().unwrap();
        assert!(free_space(&dir.path().join("a/b/c")) > 0);
    }
}
