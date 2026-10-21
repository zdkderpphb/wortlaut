// Typed wrappers around the Rust commands and events (src-tauri/src/commands.rs).

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
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
  /** Index into `Transcript.speakers`. */
  speaker: number | null;
}

export interface Transcript {
  id: string;
  title: string;
  language: string | null;
  sourcePath: string | null;
  /** Unix seconds. */
  createdAt: number;
  durationMs: number;
  speakers: string[];
  summary: string | null;
  segments: Segment[];
}

export interface TranscriptMeta {
  id: string;
  title: string;
  createdAt: number;
  durationMs: number;
  speakerCount: number;
  sourcePath: string | null;
}

export interface TranscribeOptions {
  language: string;
  detectSpeakers: boolean;
  speakerCount: number | null;
  speakerLabel: string;
}

export type Stage = "reading" | "diarizing" | "loading" | "detecting" | "transcribing";

export interface JobProgress {
  jobId: string;
  stage: Stage;
  doneMs: number;
  totalMs: number;
  segment: Segment | null;
}

export interface SummaryProgress {
  id: string;
  done: number;
  total: number;
}

export interface CommandError {
  code: string;
  detail: string;
}

export type ExportFormat = "docx" | "txt" | "md" | "srt" | "maxqda" | "atlasti" | "nvivo" | "json";

export interface ExportLabels {
  summary: string;
  transcript: string;
}

/** File extension and name suffix per format – mirrors export::Format. */
export const exportFiles: Record<ExportFormat, { ext: string; suffix: string }> = {
  docx: { ext: "docx", suffix: "" },
  txt: { ext: "txt", suffix: "" },
  md: { ext: "md", suffix: "" },
  srt: { ext: "srt", suffix: "" },
  maxqda: { ext: "txt", suffix: " (MAXQDA)" },
  atlasti: { ext: "docx", suffix: " (ATLAS.ti)" },
  nvivo: { ext: "txt", suffix: " (NVivo)" },
  json: { ext: "json", suffix: "" },
};

/** Normalises anything a failed invoke throws into a CommandError. */
export function asCommandError(e: unknown): CommandError {
  if (e && typeof e === "object" && "code" in e) return e as CommandError;
  return { code: "unknown", detail: String(e) };
}

export const api = {
  modelInfo: () => invoke<ModelInfo>("model_info"),
  downloadModel: () => invoke<void>("download_model"),
  cancelDownload: () => invoke<void>("cancel_download"),

  transcribe: (jobId: string, path: string, options: TranscribeOptions) =>
    invoke<Transcript>("transcribe", { jobId, path, options }),
  cancelTranscription: (jobId: string) => invoke<void>("cancel_transcription", { jobId }),

  historyList: () => invoke<TranscriptMeta[]>("history_list"),
  historyLoad: (id: string) => invoke<Transcript>("history_load", { id }),
  historySave: (transcript: Transcript) => invoke<void>("history_save", { transcript }),
  historyDelete: (id: string) => invoke<void>("history_delete", { id }),

  summarize: (transcript: Transcript, locale: string) => invoke<string>("summarize", { transcript, locale }),

  exportTranscript: (
    transcript: Transcript,
    format: ExportFormat,
    timestamps: boolean,
    labels: ExportLabels,
    path: string,
  ) => invoke<void>("export_transcript", { transcript, format, timestamps, labels, path }),
  exportAll: (transcript: Transcript, timestamps: boolean, labels: ExportLabels, dir: string) =>
    invoke<string[]>("export_all", { transcript, timestamps, labels, dir }),

  /** Allows the file for playback and returns its URL, or null if missing. */
  playbackUrl: async (path: string): Promise<string | null> =>
    (await invoke<boolean>("allow_playback", { path })) ? convertFileSrc(path) : null,

  takeStartupFiles: () => invoke<string[]>("take_startup_files"),
  openLogDir: () => invoke<void>("open_log_dir"),
  revealFile: (path: string) => invoke<void>("reveal_file", { path }),

  onDownloadProgress: (cb: (p: DownloadProgress) => void): Promise<UnlistenFn> =>
    listen<DownloadProgress>("download-progress", (e) => cb(e.payload)),
  onJobProgress: (cb: (p: JobProgress) => void): Promise<UnlistenFn> =>
    listen<JobProgress>("job-progress", (e) => cb(e.payload)),
  onSummaryProgress: (cb: (p: SummaryProgress) => void): Promise<UnlistenFn> =>
    listen<SummaryProgress>("summary-progress", (e) => cb(e.payload)),
};
