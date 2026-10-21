<script lang="ts">
  import { api } from "./api";
  import { formatClock, formatDuration } from "./format";
  import { errorText, t } from "./i18n";
  import Icon from "./Icon.svelte";
  import { library, remainingSeconds, type Job } from "./library.svelte";

  let { job }: { job: Job } = $props();

  let now = $state(Date.now());

  // Re-render the time estimate every second while running.
  $effect(() => {
    if (job.status !== "running") return;
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  const percent = $derived(job.totalMs > 0 ? Math.min(100, (job.doneMs / job.totalMs) * 100) : 0);
  const remaining = $derived.by(() => {
    void now;
    return remainingSeconds(job);
  });
  const stageText = $derived(
    {
      reading: t("stageReading"),
      diarizing: t("stageDiarizing"),
      loading: t("stageLoading"),
      detecting: t("stageDetecting"),
      transcribing: t("stageTranscribing"),
    }[job.stage ?? "reading"],
  );
</script>

<article class="job">
  <header>
    <div class="title">
      <h1 title={job.path}>{job.name}</h1>
      <p class="meta">{job.options.detectSpeakers ? t("speakersLabel") : t("tabTranscript")}</p>
    </div>
    {#if job.status === "running" || job.status === "waiting"}
      <button class="btn secondary" onclick={() => library.cancel(job.id)}>
        <Icon name="x" />{t("cancel")}
      </button>
    {:else}
      <div class="actions">
        <button class="btn secondary" onclick={() => library.retry(job.id)}>
          <Icon name="retry" />{t("retry")}
        </button>
        <button class="btn secondary" onclick={() => library.removeJob(job.id)}>
          <Icon name="x" />{t("remove")}
        </button>
      </div>
    {/if}
  </header>

  {#if job.status === "waiting"}
    <p class="muted status-line"><Icon name="clock" />{t("stageWaiting")}</p>
  {:else if job.status === "running"}
    <div class="progress" aria-live="polite">
      <p class="status-line"><Icon name="spinner" spin />{stageText}</p>
      <div
        class="bar"
        class:indeterminate={job.stage !== "transcribing"}
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(percent)}
      >
        <div class="fill" style:width="{percent}%"></div>
      </div>
      {#if job.stage === "transcribing"}
        <p class="numbers muted">
          <span>{t("progressOf", { done: formatClock(job.doneMs), total: formatClock(job.totalMs) })}</span>
          {#if remaining !== null}<span>{t("remaining", { time: formatDuration(remaining) })}</span>{/if}
        </p>
      {/if}
    </div>
  {:else if job.status === "error" && job.error}
    <div class="error" role="alert">
      <Icon name="alert" />
      <div>
        <p class="error-title">{t("errorTitle")}</p>
        <p>{errorText(job.error.code)}</p>
        <details>
          <summary>{t("errorDetails")}</summary>
          <code>{job.error.detail}</code>
        </details>
        <button class="link" onclick={() => api.openLogDir()}>{t("openLog")}</button>
      </div>
    </div>
  {:else if job.status === "cancelled"}
    <p class="muted">{t("statusCancelled")}</p>
  {/if}

  {#if job.segments.some((s) => s.text.trim())}
    <div class="paper">
      {#each job.segments as segment (segment.startMs)}
        {#if segment.text.trim()}
          <div class="paragraph">
            {#if segment.speaker != null}
              <span class="speaker" style:--spk="var(--spk-{segment.speaker % 6})">
                {job.options.speakerLabel}
                {segment.speaker + 1}
              </span>
            {/if}
            <p>{segment.text}</p>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</article>

<style>
  .job {
    width: 100%;
    max-width: 780px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .title {
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
  .status-line {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .numbers {
    display: flex;
    justify-content: space-between;
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  .paper {
    background: var(--surface);
    border: 0.5px solid var(--border);
    border-radius: 14px;
    box-shadow: var(--shadow);
    padding: 26px 34px;
  }
  .paragraph + .paragraph {
    margin-top: 14px;
  }
  .paragraph p {
    margin: 0;
    font-size: 16px;
    line-height: 1.65;
    color: var(--muted);
  }
  .speaker {
    display: inline-block;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--spk);
  }
  .error {
    display: flex;
    gap: 12px;
    padding: 16px;
    border-radius: 12px;
    background: var(--danger-soft);
    color: var(--danger);
  }
  .error p {
    margin: 0 0 6px;
    color: var(--text);
  }
  .error-title {
    font-weight: 600;
  }
</style>
