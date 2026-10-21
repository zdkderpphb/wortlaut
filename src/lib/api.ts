// Typed wrappers around the Rust commands and events (src-tauri/src/commands.rs).

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ModelStatus = { state: "missing" } | { state: "partial"; bytes: number } | { state: "ready" };

export interface ModelInfo {
  status: ModelStatus;
  sizeBytes: number;
  freeBytes: number;
}

export interface DownloadProgress {
  downloaded: number;
  total: number;
  bytesPerSec: number;
  verifying: boolean;
}

export interface Segment {
  startMs: number;
  endMs: number;
  text: string;
}

export interface Transcript {
  title: string;
  language: string | null;
  segments: Segment[];
}

export type Stage = "reading" | "loading" | "transcribing";

export interface JobProgress {
  jobId: string;
  stage: Stage;
  doneMs: number;
  totalMs: number;
  segment: Segment | null;
}

export interface CommandError {
  code: string;
  detail: string;
}

export type ExportFormat = "docx" | "txt" | "md";

/** Normalises anything a failed invoke throws into a CommandError. */
export function asCommandError(e: unknown): CommandError {
  if (e && typeof e === "object" && "code" in e) return e as CommandError;
  return { code: "unknown", detail: String(e) };
}

export const api = {
  modelInfo: () => invoke<ModelInfo>("model_info"),
  downloadModel: () => invoke<void>("download_model"),
  cancelDownload: () => invoke<void>("cancel_download"),
  transcribe: (jobId: string, path: string, language: string) =>
    invoke<Transcript>("transcribe", { jobId, path, language }),
  cancelTranscription: (jobId: string) => invoke<void>("cancel_transcription", { jobId }),
  exportTranscript: (transcript: Transcript, format: ExportFormat, timestamps: boolean, path: string) =>
    invoke<void>("export_transcript", { transcript, format, timestamps, path }),
  openLogDir: () => invoke<void>("open_log_dir"),
  revealFile: (path: string) => invoke<void>("reveal_file", { path }),

  onDownloadProgress: (cb: (p: DownloadProgress) => void): Promise<UnlistenFn> =>
    listen<DownloadProgress>("download-progress", (e) => cb(e.payload)),
  onJobProgress: (cb: (p: JobProgress) => void): Promise<UnlistenFn> =>
    listen<JobProgress>("job-progress", (e) => cb(e.payload)),
};
