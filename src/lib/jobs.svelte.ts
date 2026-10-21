// The transcription queue: one file at a time, in the order they were added.

import { api, asCommandError, type CommandError, type JobProgress, type Segment, type Stage } from "./api";
import { fileName } from "./format";

export type JobStatus = "waiting" | "running" | "done" | "error" | "cancelled";

export interface Job {
  id: string;
  path: string;
  name: string;
  language: string;
  status: JobStatus;
  stage: Stage | null;
  doneMs: number;
  totalMs: number;
  /** Filled live while running; editable once done. */
  segments: Segment[];
  error: CommandError | null;
  /** When the first piece of text started, for the time estimate. */
  startedAt: number | null;
}

let counter = 0;

class Queue {
  jobs = $state<Job[]>([]);
  selectedId = $state<string | null>(null);
  /** Called when the model turns out to be missing (e.g. deleted). */
  onModelMissing: () => void = () => {};

  selected = $derived(this.jobs.find((j) => j.id === this.selectedId) ?? null);

  add(paths: string[], language: string) {
    for (const path of paths) {
      if (this.jobs.some((j) => j.path === path && (j.status === "waiting" || j.status === "running"))) continue;
      const job: Job = {
        id: `job-${Date.now()}-${counter++}`,
        path,
        name: fileName(path),
        language,
        status: "waiting",
        stage: null,
        doneMs: 0,
        totalMs: 0,
        segments: [],
        error: null,
        startedAt: null,
      };
      this.jobs.push(job);
      if (!this.selectedId || this.selected?.status === "done") this.selectedId = job.id;
    }
    void this.pump();
  }

  private async pump() {
    if (this.jobs.some((j) => j.status === "running")) return;
    const job = this.jobs.find((j) => j.status === "waiting");
    if (!job) return;

    job.status = "running";
    job.stage = "reading";
    try {
      const transcript = await api.transcribe(job.id, job.path, job.language);
      job.segments = transcript.segments;
      job.status = "done";
    } catch (e) {
      const err = asCommandError(e);
      job.status = err.code === "cancelled" ? "cancelled" : "error";
      job.error = err.code === "cancelled" ? null : err;
      if (err.code === "model_missing") this.onModelMissing();
    } finally {
      job.stage = null;
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

  remove(id: string) {
    this.cancel(id);
    const index = this.jobs.findIndex((j) => j.id === id);
    if (index < 0) return;
    this.jobs.splice(index, 1);
    if (this.selectedId === id) this.selectedId = this.jobs[Math.min(index, this.jobs.length - 1)]?.id ?? null;
  }
}

export const queue = new Queue();

/** Seconds left, once there is enough progress to guess. */
export function remainingSeconds(job: Job): number | null {
  if (!job.startedAt || job.doneMs <= 0 || job.totalMs <= 0) return null;
  const elapsed = (Date.now() - job.startedAt) / 1000;
  const rate = job.doneMs / 1000 / elapsed;
  return rate > 0 ? (job.totalMs - job.doneMs) / 1000 / rate : null;
}
