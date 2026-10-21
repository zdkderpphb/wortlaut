<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, asCommandError, type CommandError, type ExportFormat } from "./api";
  import { fileName, formatClock, formatDuration, formatStamp, siblingPath, stem } from "./format";
  import { errorText, t, type MessageKey } from "./i18n";
  import Icon from "./Icon.svelte";
  import { queue, remainingSeconds, type Job } from "./jobs.svelte";
  import { plainText, toTranscript } from "./text";

  let { job, timestamps }: { job: Job; timestamps: boolean } = $props();

  const formats: { format: ExportFormat; label: MessageKey; filter: string }[] = [
    { format: "docx", label: "saveWord", filter: "Word" },
    { format: "txt", label: "saveText", filter: "Text" },
    { format: "md", label: "saveMarkdown", filter: "Markdown" },
  ];

  let savedPath = $state<string | null>(null);
  let saveError = $state<CommandError | null>(null);
  let copied = $state(false);
  let now = $state(Date.now());

  // Re-render the time estimate every second while running.
  $effect(() => {
    if (job.status !== "running") return;
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  // Forget the "saved" notice when switching files.
  $effect(() => {
    void job.id;
    savedPath = null;
    saveError = null;
  });

  const percent = $derived(job.totalMs > 0 ? Math.min(100, (job.doneMs / job.totalMs) * 100) : 0);
  const remaining = $derived.by(() => {
    void now;
    return remainingSeconds(job);
  });
  const hasText = $derived(job.segments.some((s) => s.text.trim().length > 0));
  const stageText = $derived(
    job.stage === "loading" ? t("stageLoading") : job.stage === "transcribing" ? t("stageTranscribing") : t("stageReading"),
  );

  async function exportAs(format: ExportFormat, filter: string) {
    saveError = null;
    const path = await save({
      defaultPath: siblingPath(job.path, format),
      filters: [{ name: filter, extensions: [format] }],
    });
    if (!path) return;
    try {
      await api.exportTranscript(toTranscript(stem(job.path), job.language, job.segments), format, timestamps, path);
      savedPath = path;
    } catch (e) {
      saveError = asCommandError(e);
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(plainText(job.segments, timestamps));
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<article class="job">
  <header>
    <div class="title">
      <Icon name="file" />
      <h2 title={job.path}>{job.name}</h2>
    </div>
    {#if job.status === "running" || job.status === "waiting"}
      <button class="btn secondary" onclick={() => queue.cancel(job.id)}>
        <Icon name="x" />{t("cancel")}
      </button>
    {:else if job.status === "error" || job.status === "cancelled"}
      <button class="btn secondary" onclick={() => queue.retry(job.id)}>
        <Icon name="retry" />{t("retry")}
      </button>
    {:else if job.status === "done"}
      <button class="btn secondary" onclick={copy} disabled={!hasText}>
        <Icon name={copied ? "check" : "copy"} />{copied ? t("copied") : t("copy")}
      </button>
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

  {#if job.status === "done"}
    <div class="toolbar">
      <span class="muted">{t("saveAs")}</span>
      {#each formats as f (f.format)}
        <button class="btn primary" onclick={() => exportAs(f.format, f.filter)}>
          <Icon name="download" />{t(f.label)}
        </button>
      {/each}
    </div>
    {#if savedPath}
      <div class="notice success" role="status">
        <Icon name="check" />
        <span>{t("saved", { name: fileName(savedPath) })}</span>
        <button class="link" onclick={() => api.revealFile(savedPath!)}>{t("showInFolder")}</button>
      </div>
    {/if}
    {#if saveError}
      <div class="notice danger" role="alert">
        <Icon name="alert" /><span>{errorText(saveError.code)}</span>
      </div>
    {/if}
  {/if}

  {#if job.segments.length > 0}
    {#if job.status === "done" && hasText}<p class="muted hint">{t("resultEditHint")}</p>{/if}
    <div class="document" class:live={job.status === "running"}>
      {#each job.segments as segment, i (segment.startMs)}
        {#if segment.text.trim() || job.status === "done"}
          <div class="paragraph">
            {#if timestamps}<span class="stamp">{formatStamp(segment.startMs)}</span>{/if}
            {#if job.status === "done"}
              <p contenteditable="true" spellcheck="true" bind:innerText={job.segments[i].text}></p>
            {:else}
              <p>{segment.text}</p>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {/if}

  {#if job.status === "done" && !hasText}
    <p class="muted empty">{t("resultEmpty")}</p>
  {/if}
</article>

<style>
  .job {
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
  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    color: var(--primary-strong);
  }
  h2 {
    margin: 0;
    font-size: 20px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .toolbar > .muted {
    margin-right: 4px;
  }
  .hint {
    margin: 0;
    font-size: 14px;
  }
  .document {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 24px 28px;
    max-width: 75ch;
  }
  .document.live p {
    color: var(--muted);
  }
  .paragraph + .paragraph {
    margin-top: 14px;
  }
  .stamp {
    display: block;
    font-size: 13px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    margin-bottom: 2px;
  }
  .paragraph p {
    margin: 0;
    font-size: 17px;
    line-height: 1.7;
    white-space: pre-wrap;
    border-radius: 6px;
    outline: none;
    transition: background-color 150ms;
  }
  .paragraph p[contenteditable]:hover {
    background: var(--surface-2);
  }
  .paragraph p[contenteditable]:focus-visible {
    background: var(--surface-2);
    box-shadow: 0 0 0 2px var(--focus);
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
  .empty {
    text-align: center;
    padding: 24px;
  }
</style>
