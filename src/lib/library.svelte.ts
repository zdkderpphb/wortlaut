// Everything in the list on the left: files in progress (one at a time, in
// the order added) and the saved history of finished transcripts.

import {
  api,
  asCommandError,
  type CommandError,
  type JobProgress,
  type Segment,
  type Stage,
  type TranscribeOptions,
  type Transcript,
  type TranscriptMeta,
} from "./api";
import { fileName } from "./format";

export type JobStatus = "waiting" | "running" | "error" | "cancelled";

export interface Job {
  id: string;
  path: string;
  name: string;
  options: TranscribeOptions;
  status: JobStatus;
  stage: Stage | null;
  doneMs: number;
  totalMs: number;
  /** Shown live while running. */
  segments: Segment[];
  error: CommandError | null;
  /** When the first piece of text started, for the time estimate. */
  startedAt: number | null;
}

export interface HistoryItem {
  meta: TranscriptMeta;
  /** Loaded when first opened. */
  transcript: Transcript | null;
  error: CommandError | null;
}

export type Selection = { kind: "job"; id: string } | { kind: "history"; id: string } | null;

const SAVE_DELAY_MS = 800;
let counter = 0;

export function metaOf(t: Transcript): TranscriptMeta {
  return {
    id: t.id,
    title: t.title,
    createdAt: t.createdAt,
    durationMs: t.durationMs,
    speakerCount: t.speakers.length,
    sourcePath: t.sourcePath,
  };
}

class Library {
  jobs = $state<Job[]>([]);
  history = $state<HistoryItem[]>([]);
  selection = $state<Selection>(null);
  /** Called when the model turns out to be missing (e.g. deleted). */
  onModelMissing: () => void = () => {};

  private saveTimers = new Map<string, ReturnType<typeof setTimeout>>();

  selectedJob = $derived(
    this.selection?.kind === "job" ? (this.jobs.find((j) => j.id === this.selection!.id) ?? null) : null,
  );
  selectedItem = $derived(
    this.selection?.kind === "history"
      ? (this.history.find((h) => h.meta.id === this.selection!.id) ?? null)
      : null,
  );
  isEmpty = $derived(this.jobs.length === 0 && this.history.length === 0);

  async init() {
    const metas = await api.historyList();
    this.history = metas.map((meta) => ({ meta, transcript: null, error: null }));
  }

  // ------------------------------------------------------------ jobs

  add(paths: string[], options: TranscribeOptions) {
    let first: string | null = null;
    for (const path of paths) {
      if (this.jobs.some((j) => j.path === path && (j.status === "waiting" || j.status === "running"))) continue;
      const job: Job = {
        id: `job-${Date.now()}-${counter++}`,
        path,
        name: fileName(path),
        options: { ...options },
        status: "waiting",
        stage: null,
        doneMs: 0,
        totalMs: 0,
        segments: [],
        error: null,
        startedAt: null,
      };
      this.jobs.push(job);
      first ??= job.id;
    }
    if (first) this.selection = { kind: "job", id: first };
    void this.pump();
  }

  private async pump() {
    if (this.jobs.some((j) => j.status === "running")) return;
    const job = this.jobs.find((j) => j.status === "waiting");
    if (!job) return;

    job.status = "running";
    job.stage = "reading";
    try {
      const transcript = await api.transcribe(job.id, job.path, job.options);
      this.jobs.splice(this.jobs.indexOf(job), 1);
      this.history.unshift({ meta: metaOf(transcript), transcript, error: null });
      if (this.selection?.kind === "job" && this.selection.id === job.id) {
        this.selection = { kind: "history", id: transcript.id };
      }
    } catch (e) {
      const err = asCommandError(e);
      job.status = err.code === "cancelled" ? "cancelled" : "error";
      job.error = err.code === "cancelled" ? null : err;
      job.stage = null;
      if (err.code === "model_missing") this.onModelMissing();
    } finally {
      void this.pump();
    }
  }

  handleProgress(p: JobProgress) {
    const job = this.jobs.find((j) => j.id === p.jobId);
    if (!job || job.status !== "running") return;
    job.stage = p.stage;
    job.totalMs = p.totalMs;
    job.doneMs = p.doneMs;
    if (p.stage === "transcribing" && job.startedAt === null) job.startedAt = Date.now();
    if (p.segment) job.segments.push(p.segment);
  }

  cancel(id: string) {
    const job = this.jobs.find((j) => j.id === id);
    if (job?.status === "waiting") job.status = "cancelled";
    else if (job?.status === "running") void api.cancelTranscription(id);
  }

  retry(id: string) {
    const job = this.jobs.find((j) => j.id === id);
    if (!job) return;
    Object.assign(job, { status: "waiting", stage: null, doneMs: 0, totalMs: 0, segments: [], error: null, startedAt: null });
    void this.pump();
  }

  removeJob(id: string) {
    this.cancel(id);
    this.jobs = this.jobs.filter((j) => j.id !== id);
    if (this.selection?.id === id) this.selection = null;
  }

  // --------------------------------------------------------- history

  async open(id: string) {
    this.selection = { kind: "history", id };
    const item = this.history.find((h) => h.meta.id === id);
    if (!item || item.transcript) return;
    try {
      item.transcript = await api.historyLoad(id);
      item.error = null;
    } catch (e) {
      item.error = asCommandError(e);
    }
  }

  /** Saves shortly after the last change, so typing stays smooth. */
  scheduleSave(t: Transcript) {
    const item = this.history.find((h) => h.meta.id === t.id);
    if (item) item.meta = metaOf(t);
    clearTimeout(this.saveTimers.get(t.id));
    this.saveTimers.set(
      t.id,
      setTimeout(() => {
        this.saveTimers.delete(t.id);
        void api.historySave($state.snapshot(t) as Transcript);
      }, SAVE_DELAY_MS),
    );
  }

  async remove(id: string) {
    clearTimeout(this.saveTimers.get(id));
    this.saveTimers.delete(id);
    await api.historyDelete(id);
    this.history = this.history.filter((h) => h.meta.id !== id);
    if (this.selection?.id === id) this.selection = null;
  }
}

export const library = new Library();

/** Seconds left, once there is enough progress to guess. */
export function remainingSeconds(job: Job): number | null {
  if (!job.startedAt || job.doneMs <= 0 || job.totalMs <= 0) return null;
  const elapsed = (Date.now() - job.startedAt) / 1000;
  const rate = job.doneMs / 1000 / elapsed;
  return rate > 0 ? (job.totalMs - job.doneMs) / 1000 / rate : null;
}
