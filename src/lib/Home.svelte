<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, type ExportFormat } from "./api";
  import { extension, fileName, stem } from "./format";
  import { languages, t } from "./i18n";
  import Icon from "./Icon.svelte";
  import JobView from "./JobView.svelte";
  import { queue, type JobStatus } from "./jobs.svelte";
  import { toTranscript } from "./text";

  const SUPPORTED = ["mp3", "m4a", "aac", "wav", "flac", "ogg", "oga", "opus", "webm", "mp4", "m4v", "mov", "mkv", "aif", "aiff", "caf"];

  let language = $state(load("language", "auto"));
  let timestamps = $state(load("timestamps", "false") === "true");
  let dragging = $state(false);
  let skipped = $state<string[]>([]);
  let savedAll = $state<number | null>(null);

  $effect(() => store("language", language));
  $effect(() => store("timestamps", String(timestamps)));

  const doneJobs = $derived(queue.jobs.filter((j) => j.status === "done"));

  function load(key: string, fallback: string): string {
    try {
      return localStorage.getItem(`wortlaut.${key}`) ?? fallback;
    } catch {
      return fallback;
    }
  }
  function store(key: string, value: string) {
    try {
      localStorage.setItem(`wortlaut.${key}`, value);
    } catch {
      /* settings are a convenience only */
    }
  }

  function addFiles(paths: string[]) {
    const ok = paths.filter((p) => SUPPORTED.includes(extension(p)));
    skipped = paths.filter((p) => !ok.includes(p)).map(fileName);
    if (ok.length) queue.add(ok, language);
  }

  async function pickFiles() {
    const picked = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "Audio & Video", extensions: SUPPORTED }],
    });
    if (picked) addFiles(Array.isArray(picked) ? picked : [picked]);
  }

  async function saveAll(format: ExportFormat) {
    const dir = await open({ directory: true, multiple: false });
    if (!dir || Array.isArray(dir)) return;
    const sep = dir.includes("\\") ? "\\" : "/";
    for (const job of doneJobs) {
      const transcript = toTranscript(stem(job.path), job.language, job.segments);
      await api.exportTranscript(transcript, format, timestamps, `${dir}${sep}${stem(job.path)}.${format}`);
    }
    savedAll = doneJobs.length;
    setTimeout(() => (savedAll = null), 4000);
  }

  const statusIcon: Record<JobStatus, "clock" | "spinner" | "check" | "alert" | "x"> = {
    waiting: "clock",
    running: "spinner",
    done: "check",
    error: "alert",
    cancelled: "x",
  };
  const statusLabel = {
    waiting: "statusWaiting",
    running: "statusRunning",
    done: "statusDone",
    error: "statusError",
    cancelled: "statusCancelled",
  } as const;

  let unlisten: (() => void)[] = [];
  onMount(async () => {
    unlisten.push(await api.onJobProgress((p) => queue.handleProgress(p)));
    unlisten.push(
      await getCurrentWebview().onDragDropEvent((event) => {
        const kind = event.payload.type;
        if (kind === "enter" || kind === "over") dragging = true;
        else if (kind === "leave") dragging = false;
        else if (kind === "drop") {
          dragging = false;
          addFiles(event.payload.paths);
        }
      }),
    );
  });
  onDestroy(() => unlisten.forEach((u) => u()));
</script>

{#snippet options()}
  <div class="options">
    <label class="field">
      <span>{t("languageLabel")}</span>
      <select bind:value={language}>
        {#each languages as l (l.code)}
          <option value={l.code}>{t(l.label)}</option>
        {/each}
      </select>
    </label>
    <label class="switch">
      <input type="checkbox" bind:checked={timestamps} />
      <span class="track" aria-hidden="true"><span class="thumb"></span></span>
      <span class="switch-text">
        <span>{t("timestampsLabel")}</span>
        <small>{t("timestampsHint")}</small>
      </span>
    </label>
  </div>
{/snippet}

{#snippet skippedNotice()}
  {#if skipped.length}
    <div class="notice danger" role="alert">
      <Icon name="alert" />
      <span>{t("unsupportedFiles", { names: skipped.join(", ") })}</span>
      <button class="icon-btn" aria-label={t("remove")} onclick={() => (skipped = [])}><Icon name="x" size={16} /></button>
    </div>
  {/if}
{/snippet}

<div class="app">
  <header class="topbar">
    <div class="brand">
      <span class="brand-logo"><Icon name="logo" size={18} /></span>
      <strong>Wortlaut</strong>
      <span class="muted tagline">{t("appTagline")}</span>
    </div>
    <span class="badge"><Icon name="shield" size={16} />{t("offlineBadge")}</span>
  </header>

  {#if queue.jobs.length === 0}
    <main class="empty-state">
      <button class="dropzone" class:active={dragging} onclick={pickFiles}>
        <span class="drop-icon"><Icon name="upload" size={36} /></span>
        <span class="drop-title">{dragging ? t("dropActive") : t("dropTitle")}</span>
        <span class="drop-subtitle">{t("dropSubtitle")}</span>
        <span class="drop-formats">{t("dropFormats")}</span>
      </button>
      {@render skippedNotice()}
      {@render options()}
      <p class="muted privacy"><Icon name="shield" size={16} />{t("footerPrivacy")}</p>
    </main>
  {:else}
    <div class="workspace">
      <aside>
        <button class="dropzone small" class:active={dragging} onclick={pickFiles}>
          <Icon name="plus" />
          <span>{dragging ? t("dropActive") : t("addMore")}</span>
        </button>
        {@render skippedNotice()}
        <ul class="jobs">
          {#each queue.jobs as job (job.id)}
            <li>
              <button
                class="job-item"
                class:selected={job.id === queue.selectedId}
                onclick={() => (queue.selectedId = job.id)}
                aria-current={job.id === queue.selectedId}
              >
                <span class="job-status {job.status}">
                  <Icon name={statusIcon[job.status]} size={18} spin={job.status === "running"} />
                </span>
                <span class="job-text">
                  <span class="job-name" title={job.path}>{job.name}</span>
                  <span class="job-sub muted">
                    {t(statusLabel[job.status])}{#if job.status === "running" && job.totalMs > 0}
                      · {Math.round((job.doneMs / job.totalMs) * 100)} %{/if}
                  </span>
                </span>
              </button>
              {#if job.status !== "running"}
                <button class="icon-btn remove" aria-label={t("remove")} title={t("remove")} onclick={() => queue.remove(job.id)}>
                  <Icon name="x" size={16} />
                </button>
              {/if}
            </li>
          {/each}
        </ul>

        {#if doneJobs.length > 1}
          <div class="save-all">
            <span class="muted">{t("saveAllAs")}</span>
            <div class="row">
              <button class="btn secondary" onclick={() => saveAll("docx")}>{t("saveWord")}</button>
              <button class="btn secondary" onclick={() => saveAll("txt")}>{t("saveText")}</button>
              <button class="btn secondary" onclick={() => saveAll("md")}>{t("saveMarkdown")}</button>
            </div>
            {#if savedAll !== null}
              <p class="success-text" role="status"><Icon name="check" size={16} />{t("savedAll", { count: savedAll })}</p>
            {/if}
          </div>
        {/if}

        {@render options()}
      </aside>

      <main class="detail">
        {#if queue.selected}
          <JobView job={queue.selected} {timestamps} />
        {/if}
      </main>
    </div>
  {/if}
</div>

<style>
  .app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 24px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .brand-logo {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 9px;
    background: var(--primary);
    color: var(--on-primary);
  }
  .tagline {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 999px;
    background: var(--primary-soft);
    color: var(--primary-strong);
    font-size: 14px;
    white-space: nowrap;
  }

  .empty-state {
    flex: 1;
    width: 100%;
    max-width: 720px;
    margin: 0 auto;
    padding: 40px 32px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    justify-content: center;
  }
  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 56px 24px;
    border: 2px dashed var(--border-strong);
    border-radius: 20px;
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    transition:
      border-color 150ms,
      background-color 150ms;
  }
  .dropzone:hover,
  .dropzone.active {
    border-color: var(--primary);
    background: var(--primary-soft);
  }
  .dropzone:focus-visible {
    outline: 3px solid var(--focus);
    outline-offset: 3px;
  }
  .drop-icon {
    display: grid;
    place-items: center;
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: var(--primary-soft);
    color: var(--primary-strong);
    margin-bottom: 8px;
  }
  .drop-title {
    font-size: 24px;
    font-weight: 650;
  }
  .drop-subtitle {
    font-size: 17px;
    color: var(--muted);
  }
  .drop-formats {
    margin-top: 12px;
    font-size: 13px;
    color: var(--muted);
  }
  .dropzone.small {
    flex-direction: row;
    justify-content: center;
    padding: 14px;
    border-radius: 12px;
    font-weight: 600;
  }
  .privacy {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin: 0;
    font-size: 14px;
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 16px 32px;
    align-items: center;
  }
  .workspace aside .options {
    flex-direction: column;
    align-items: stretch;
    gap: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 14px;
    color: var(--muted);
  }
  select {
    appearance: none;
    min-height: 44px;
    min-width: 220px;
    padding: 0 40px 0 14px;
    border-radius: 10px;
    border: 1px solid var(--border-strong);
    background-color: var(--surface);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%237a8b8a' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 14px center;
    color: var(--text);
    font: inherit;
    font-size: 16px;
    cursor: pointer;
    transition: border-color 150ms;
  }
  select:hover {
    border-color: var(--primary);
  }
  select:focus-visible {
    outline: 3px solid var(--focus);
    outline-offset: 1px;
  }

  .switch {
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    min-height: 44px;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .track {
    position: relative;
    width: 44px;
    height: 26px;
    border-radius: 999px;
    background: var(--border-strong);
    transition: background-color 150ms;
    flex-shrink: 0;
  }
  .thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.25);
    transition: transform 150ms;
  }
  .switch input:checked + .track {
    background: var(--primary);
  }
  .switch input:checked + .track .thumb {
    transform: translateX(18px);
  }
  .switch input:focus-visible + .track {
    outline: 3px solid var(--focus);
    outline-offset: 2px;
  }
  .switch-text {
    display: flex;
    flex-direction: column;
  }
  .switch-text small {
    color: var(--muted);
    font-size: 13px;
  }

  .workspace {
    flex: 1;
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    min-height: 0;
  }
  aside {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
    overflow-y: auto;
  }
  .detail {
    padding: 28px 32px;
    overflow-y: auto;
  }
  .jobs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .jobs li {
    position: relative;
  }
  .job-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 56px;
    padding: 8px 40px 8px 12px;
    border: 1px solid transparent;
    border-radius: 12px;
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font: inherit;
    transition: background-color 150ms;
  }
  .job-item:hover {
    background: var(--surface);
  }
  .job-item.selected {
    background: var(--surface);
    border-color: var(--border);
    box-shadow: var(--shadow);
  }
  .job-item:focus-visible {
    outline: 3px solid var(--focus);
    outline-offset: 1px;
  }
  .job-status {
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .job-status.running {
    color: var(--primary-strong);
  }
  .job-status.done {
    color: var(--success);
  }
  .job-status.error {
    color: var(--danger);
  }
  .job-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .job-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .job-sub {
    font-size: 13px;
  }
  .remove {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
  }
  .save-all {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 14px;
  }
  .save-all .row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .success-text {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--success);
  }

  @media (max-width: 820px) {
    .workspace {
      grid-template-columns: 1fr;
    }
    aside {
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .tagline {
      display: none;
    }
  }
</style>
