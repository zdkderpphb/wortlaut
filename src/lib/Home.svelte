<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { api } from "./api";
  import { extension, fileName, formatClock } from "./format";
  import { errorText, locale, t } from "./i18n";
  import Icon from "./Icon.svelte";
  import JobView from "./JobView.svelte";
  import { library, type JobStatus } from "./library.svelte";
  import NewView from "./NewView.svelte";
  import { settings } from "./settings.svelte";
  import TranscriptView from "./TranscriptView.svelte";

  const SUPPORTED = ["mp3", "m4a", "aac", "wav", "flac", "ogg", "oga", "opus", "webm", "mp4", "m4v", "mov", "mkv", "aif", "aiff", "caf"];

  let dragging = $state(false);
  let skipped = $state<string[]>([]);

  const dateFormat = new Intl.DateTimeFormat(locale === "de" ? "de-AT" : "en-US", {
    dateStyle: "short",
    timeStyle: "short",
  });

  function addFiles(paths: string[]) {
    const ok = paths.filter((p) => SUPPORTED.includes(extension(p)));
    skipped = paths.filter((p) => !ok.includes(p)).map(fileName);
    if (!ok.length) return;
    library.add(ok, {
      language: settings.language,
      detectSpeakers: settings.detectSpeakers,
      speakerCount: settings.detectSpeakers ? settings.speakerCount : null,
      speakerLabel: t("speakerLabel"),
    });
  }

  async function pickFiles() {
    const picked = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "Audio & Video", extensions: SUPPORTED }],
    });
    if (picked) addFiles(Array.isArray(picked) ? picked : [picked]);
  }

  async function removeHistory(id: string, title: string) {
    const confirmed = await ask(t("deleteConfirm", { name: title }), {
      title: t("deleteTitle"),
      kind: "warning",
      okLabel: t("delete"),
      cancelLabel: t("cancel"),
    });
    if (confirmed) await library.remove(id);
  }

  const statusIcon: Record<JobStatus, "clock" | "spinner" | "alert" | "x"> = {
    waiting: "clock",
    running: "spinner",
    error: "alert",
    cancelled: "x",
  };
  const statusLabel = {
    waiting: "statusWaiting",
    running: "statusRunning",
    error: "statusError",
    cancelled: "statusCancelled",
  } as const;

  let unlisten: (() => void)[] = [];
  onMount(async () => {
    unlisten.push(await api.onJobProgress((p) => library.handleProgress(p)));
    addFiles(await api.takeStartupFiles());
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

{#snippet skippedNotice()}
  {#if skipped.length}
    <div class="notice danger" role="alert">
      <Icon name="alert" />
      <span>{t("unsupportedFiles", { names: skipped.join(", ") })}</span>
      <button class="icon-btn" aria-label={t("close")} onclick={() => (skipped = [])}><Icon name="x" size={16} /></button>
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

  {#if library.isEmpty}
    <main class="empty-state">
      {@render skippedNotice()}
      <NewView {dragging} onPick={pickFiles} />
    </main>
  {:else}
    <div class="workspace">
      <aside>
        <button class="new-file" class:selected={library.selection === null} onclick={() => (library.selection = null)}>
          <Icon name="plus" />
          <span>{dragging ? t("dropActive") : t("newFile")}</span>
        </button>

        {#if library.jobs.length}
          <h3 class="section">{t("sectionRunning")}</h3>
          <ul class="list">
            {#each library.jobs as job (job.id)}
              <li>
                <button
                  class="entry"
                  class:selected={library.selection?.id === job.id}
                  onclick={() => (library.selection = { kind: "job", id: job.id })}
                >
                  <span class="entry-icon {job.status}">
                    <Icon name={statusIcon[job.status]} size={18} spin={job.status === "running"} />
                  </span>
                  <span class="entry-text">
                    <span class="entry-name" title={job.path}>{job.name}</span>
                    <span class="entry-sub muted">
                      {t(statusLabel[job.status])}{#if job.status === "running" && job.totalMs > 0}
                        · {Math.round((job.doneMs / job.totalMs) * 100)} %{/if}
                    </span>
                  </span>
                </button>
                {#if job.status !== "running"}
                  <button class="icon-btn remove" aria-label={t("remove")} title={t("remove")} onclick={() => library.removeJob(job.id)}>
                    <Icon name="x" size={16} />
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}

        {#if library.history.length}
          <h3 class="section">{t("sectionHistory")}</h3>
          <ul class="list">
            {#each library.history as item (item.meta.id)}
              <li>
                <button
                  class="entry"
                  class:selected={library.selection?.id === item.meta.id}
                  onclick={() => library.open(item.meta.id)}
                >
                  <span class="entry-icon done"><Icon name={item.meta.speakerCount ? "users" : "fileText"} size={18} /></span>
                  <span class="entry-text">
                    <span class="entry-name" title={item.meta.sourcePath ?? item.meta.title}>{item.meta.title}</span>
                    <span class="entry-sub muted">
                      {item.meta.createdAt ? dateFormat.format(item.meta.createdAt * 1000) : ""}{#if item.meta.durationMs}
                        · {formatClock(item.meta.durationMs)}{/if}
                    </span>
                  </span>
                </button>
                <button
                  class="icon-btn remove"
                  aria-label={t("delete")}
                  title={t("delete")}
                  onclick={() => removeHistory(item.meta.id, item.meta.title)}
                >
                  <Icon name="trash" size={16} />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </aside>

      <main class="detail">
        {@render skippedNotice()}
        {#if library.selectedJob}
          <JobView job={library.selectedJob} />
        {:else if library.selectedItem}
          {#if library.selectedItem.transcript}
            <TranscriptView transcript={library.selectedItem.transcript} />
          {:else if library.selectedItem.error}
            <div class="notice danger" role="alert">
              <Icon name="alert" /><span>{errorText(library.selectedItem.error.code)}</span>
            </div>
          {:else}
            <p class="muted loading"><Icon name="spinner" spin /></p>
          {/if}
        {:else}
          <NewView {dragging} onPick={pickFiles} compact />
        {/if}
      </main>
    </div>
  {/if}
</div>

<style>
  .app {
    height: 100vh;
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
    overflow-y: auto;
    padding: 40px 32px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 16px;
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
    gap: 6px;
    padding: 20px 16px;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
    overflow-y: auto;
  }
  .detail {
    padding: 28px 32px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .new-file {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 48px;
    border: 2px dashed var(--border-strong);
    border-radius: 12px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    transition:
      border-color 150ms,
      background-color 150ms;
  }
  .new-file:hover,
  .new-file.selected {
    border-color: var(--primary);
    background: var(--primary-soft);
  }
  .new-file:focus-visible {
    outline: 3px solid var(--focus);
    outline-offset: 2px;
  }
  .section {
    margin: 14px 4px 2px;
    font-size: 13px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .list li {
    position: relative;
  }
  .entry {
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
  .entry:hover {
    background: var(--surface);
  }
  .entry.selected {
    background: var(--surface);
    border-color: var(--border);
    box-shadow: var(--shadow);
  }
  .entry:focus-visible {
    outline: 3px solid var(--focus);
    outline-offset: 1px;
  }
  .entry-icon {
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .entry-icon.running,
  .entry-icon.done {
    color: var(--primary-strong);
  }
  .entry-icon.error {
    color: var(--danger);
  }
  .entry-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .entry-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry-sub {
    font-size: 13px;
    white-space: nowrap;
  }
  .remove {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
    transition: opacity 150ms;
  }
  .list li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }
  .loading {
    display: flex;
    justify-content: center;
    padding: 40px;
  }

  @media (max-width: 820px) {
    .workspace {
      grid-template-columns: 1fr;
    }
    aside {
      border-right: none;
      border-bottom: 1px solid var(--border);
      max-height: 40vh;
    }
    .tagline {
      display: none;
    }
  }
</style>
