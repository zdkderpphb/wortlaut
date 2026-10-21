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
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
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

{#if library.isEmpty}
  <div class="welcome">
    <div class="titlebar drag" data-tauri-drag-region></div>
    <main class="welcome-body">
      <header class="welcome-head">
        <span class="app-icon"><Icon name="logo" size={30} /></span>
        <h1>Wortlaut</h1>
        <p class="muted">{t("appTagline")}</p>
      </header>
      {@render skippedNotice()}
      <NewView {dragging} onPick={pickFiles} />
      <p class="privacy"><Icon name="shield" size={15} />{t("offlineLong")}</p>
    </main>
  </div>
{:else}
  <div class="window">
    <aside class="sidebar">
      <div class="sidebar-top drag" data-tauri-drag-region></div>
      <div class="brand drag" data-tauri-drag-region>
        <span class="brand-icon"><Icon name="logo" size={16} /></span>
        <strong>Wortlaut</strong>
      </div>

      <button class="new-file" class:selected={library.selection === null} onclick={() => (library.selection = null)}>
        <Icon name="plus" size={18} />
        <span>{dragging ? t("dropActive") : t("newFile")}</span>
      </button>

      <nav class="lists">
        {#if library.jobs.length}
          <h2 class="section">{t("sectionRunning")}</h2>
          <ul>
            {#each library.jobs as job (job.id)}
              <li>
                <button
                  class="item"
                  class:selected={library.selection?.id === job.id}
                  onclick={() => (library.selection = { kind: "job", id: job.id })}
                >
                  <span class="item-icon {job.status}">
                    <Icon name={statusIcon[job.status]} size={17} spin={job.status === "running"} />
                  </span>
                  <span class="item-text">
                    <span class="item-name" title={job.path}>{job.name}</span>
                    <span class="item-sub">
                      {t(statusLabel[job.status])}{#if job.status === "running" && job.totalMs > 0}
                        · {Math.round((job.doneMs / job.totalMs) * 100)} %{/if}
                    </span>
                  </span>
                </button>
                {#if job.status !== "running"}
                  <button class="icon-btn remove" aria-label={t("remove")} title={t("remove")} onclick={() => library.removeJob(job.id)}>
                    <Icon name="x" size={15} />
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}

        {#if library.history.length}
          <h2 class="section">{t("sectionHistory")}</h2>
          <ul>
            {#each library.history as item (item.meta.id)}
              <li>
                <button
                  class="item"
                  class:selected={library.selection?.id === item.meta.id}
                  onclick={() => library.open(item.meta.id)}
                >
                  <span class="item-icon done"><Icon name={item.meta.speakerCount ? "users" : "fileText"} size={17} /></span>
                  <span class="item-text">
                    <span class="item-name" title={item.meta.sourcePath ?? item.meta.title}>{item.meta.title}</span>
                    <span class="item-sub">
                      {item.meta.createdAt ? dateFormat.format(item.meta.createdAt * 1000) : ""}{#if item.meta.durationMs}
                        · {formatClock(item.meta.durationMs)}{/if}
                    </span>
                  </span>
                </button>
                <button class="icon-btn remove" aria-label={t("delete")} title={t("delete")} onclick={() => removeHistory(item.meta.id, item.meta.title)}>
                  <Icon name="trash" size={15} />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </nav>

      <p class="sidebar-footer"><Icon name="shield" size={14} />{t("sidebarPrivacy")}</p>
    </aside>

    <section class="content">
      <div class="titlebar drag" data-tauri-drag-region></div>
      <div class="content-body">
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
          <div class="new-page">
            <NewView {dragging} onPick={pickFiles} />
          </div>
        {/if}
      </div>
    </section>
  </div>
{/if}

<style>
  /* Space for the window controls / drag area at the top. */
  .titlebar {
    height: 12px;
    flex-shrink: 0;
  }
  :global(.mac) .titlebar {
    height: 38px;
  }

  .welcome {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--grouped);
  }
  .welcome-body {
    flex: 1;
    overflow-y: auto;
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
    padding: 8px 28px 32px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 20px;
  }
  .welcome-head {
    text-align: center;
  }
  .app-icon {
    display: inline-grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: 15px;
    background: linear-gradient(160deg, #19b4a3, #0d5f58);
    color: #fff;
    box-shadow: 0 4px 14px rgb(15 118 110 / 0.3);
  }
  .welcome-head h1 {
    margin: 12px 0 2px;
    font-size: 26px;
    font-weight: 700;
  }
  .welcome-head p {
    margin: 0;
    font-size: 15px;
  }
  .privacy {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin: 0;
    font-size: 13px;
    color: var(--faint);
  }

  .window {
    height: 100vh;
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--sidebar);
    border-right: 0.5px solid var(--separator);
  }
  .sidebar-top {
    height: 8px;
    flex-shrink: 0;
  }
  :global(.mac) .sidebar-top {
    height: 30px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 18px 12px;
    font-size: 15px;
  }
  .brand-icon {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    background: linear-gradient(160deg, #19b4a3, #0d5f58);
    color: #fff;
  }
  .new-file {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 10px 8px;
    min-height: 36px;
    padding: 0 10px;
    border: none;
    border-radius: 8px;
    background: none;
    color: var(--primary-strong);
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
  }
  .new-file:hover {
    background: var(--hover);
  }
  .new-file.selected {
    background: var(--selected);
  }
  .new-file:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--focus);
  }
  .lists {
    flex: 1;
    overflow-y: auto;
    padding: 0 10px 10px;
  }
  .section {
    margin: 14px 10px 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--faint);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    position: relative;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 46px;
    padding: 5px 34px 5px 10px;
    border: none;
    border-radius: 8px;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    background: var(--hover);
  }
  .item.selected {
    background: var(--selected);
  }
  .item:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--focus);
  }
  .item-icon {
    display: grid;
    place-items: center;
    color: var(--faint);
  }
  .item-icon.running,
  .item-icon.done {
    color: var(--primary-strong);
  }
  .item-icon.error {
    color: var(--danger);
  }
  .item-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.3;
  }
  .item-name {
    font-size: 14px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item-sub {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .remove {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
    transition: opacity 120ms;
  }
  li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }
  .sidebar-footer {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 10px 18px 14px;
    font-size: 12px;
    color: var(--faint);
    border-top: 0.5px solid var(--separator);
  }

  .content {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
  }
  .content-body {
    flex: 1;
    overflow-y: auto;
    padding: 4px 36px 36px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .new-page {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    width: 100%;
    max-width: 620px;
    margin: 0 auto;
  }
  .loading {
    display: flex;
    justify-content: center;
    padding: 40px;
  }
</style>
