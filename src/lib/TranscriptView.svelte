<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    asCommandError,
    exportFiles,
    type CommandError,
    type ExportFormat,
    type ExportLabels,
    type Segment,
    type Transcript,
  } from "./api";
  import ExportMenu from "./ExportMenu.svelte";
  import { fileName, formatClock, formatStamp } from "./format";
  import { errorText, locale, t, type MessageKey } from "./i18n";
  import Icon from "./Icon.svelte";
  import { library } from "./library.svelte";
  import { settings } from "./settings.svelte";
  import { countInTranscript, plainText, replaceInTranscript } from "./text";

  let { transcript }: { transcript: Transcript } = $props();

  const labels: ExportLabels = { summary: t("summaryTitle"), transcript: t("transcriptHeading") };
  const primary: { format: ExportFormat; label: MessageKey }[] = [
    { format: "docx", label: "saveWord" },
    { format: "txt", label: "saveText" },
    { format: "md", label: "saveMarkdown" },
  ];
  const formatNames: Record<ExportFormat, MessageKey> = {
    docx: "saveWord",
    txt: "saveText",
    md: "saveMarkdown",
    srt: "formatSrt",
    maxqda: "formatMaxqda",
    atlasti: "formatAtlasti",
    nvivo: "formatNvivo",
    json: "formatJson",
  };
  const dateFormat = new Intl.DateTimeFormat(locale === "de" ? "de-AT" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  });

  let notice = $state<{ kind: "success" | "danger"; text: string; reveal?: string } | null>(null);
  let copied = $state(false);

  let findOpen = $state(false);
  let query = $state("");
  let replacement = $state("");
  let replacedCount = $state<number | null>(null);
  const matches = $derived(countInTranscript(transcript, query));

  let summarizing = $state(false);
  let summaryStep = $state<{ done: number; total: number } | null>(null);
  let summaryError = $state<CommandError | null>(null);

  /** Paragraph whose speaker label is being renamed. */
  let renameAt = $state<number | null>(null);
  let renameValue = $state("");

  let audio = $state<HTMLAudioElement>();
  let audioUrl = $state<string | null>(null);
  let playerVisible = $state(false);
  let playing = $state(false);
  let currentMs = $state(0);
  let playbackNotice = $state<string | null>(null);
  const activeIndex = $derived(
    playerVisible ? transcript.segments.findIndex((s) => currentMs >= s.startMs && currentMs < s.endMs) : -1,
  );

  const hasText = $derived(transcript.segments.some((s) => s.text.trim().length > 0));
  const meta = $derived(
    [
      transcript.createdAt ? dateFormat.format(transcript.createdAt * 1000) : null,
      transcript.durationMs ? formatClock(transcript.durationMs) : null,
      transcript.speakers.length ? t("metaSpeakers", { count: transcript.speakers.length }) : null,
    ]
      .filter(Boolean)
      .join(" · "),
  );

  // Switching to another transcript starts fresh.
  $effect(() => {
    void transcript.id;
    notice = null;
    findOpen = false;
    replacedCount = null;
    renameAt = null;
    summaryError = null;
    playbackNotice = null;
    audio?.pause();
    audioUrl = null;
    playerVisible = false;
  });

  let unlisten: (() => void) | undefined;
  onMount(async () => {
    unlisten = await api.onSummaryProgress((p) => {
      if (p.id === transcript.id) summaryStep = { done: p.done, total: p.total };
    });
  });
  onDestroy(() => {
    unlisten?.();
    audio?.pause();
  });

  function changed() {
    library.scheduleSave(transcript);
  }

  function snapshot(): Transcript {
    return $state.snapshot(transcript) as Transcript;
  }

  /** Folder of the recording, so exports land next to it. */
  function sourceDir(): string {
    const p = transcript.sourcePath;
    return p ? p.slice(0, p.length - fileName(p).length) : "";
  }

  async function exportAs(format: ExportFormat) {
    const { ext, suffix } = exportFiles[format];
    const path = await save({
      defaultPath: `${sourceDir()}${transcript.title}${suffix}.${ext}`,
      filters: [{ name: t(formatNames[format]), extensions: [ext] }],
    });
    if (!path) return;
    try {
      await api.exportTranscript(snapshot(), format, settings.timestamps, labels, path);
      notice = { kind: "success", text: t("saved", { name: fileName(path) }), reveal: path };
    } catch (e) {
      notice = { kind: "danger", text: errorText(asCommandError(e).code) };
    }
  }

  async function exportAll() {
    const dir = await open({ directory: true, multiple: false, defaultPath: sourceDir() || undefined });
    if (!dir || Array.isArray(dir)) return;
    try {
      const files = await api.exportAll(snapshot(), settings.timestamps, labels, dir);
      const sep = dir.includes("\\") ? "\\" : "/";
      notice = { kind: "success", text: t("exportedAll", { count: files.length }), reveal: `${dir}${sep}${files[0]}` };
    } catch (e) {
      notice = { kind: "danger", text: errorText(asCommandError(e).code) };
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(plainText(transcript, settings.timestamps));
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  function replaceAll() {
    replacedCount = replaceInTranscript(transcript, query, replacement);
    if (replacedCount) changed();
  }

  async function summarize() {
    const target = transcript;
    summarizing = true;
    summaryError = null;
    summaryStep = null;
    try {
      target.summary = await api.summarize($state.snapshot(target) as Transcript, locale);
      library.scheduleSave(target);
    } catch (e) {
      summaryError = asCommandError(e);
    } finally {
      summarizing = false;
    }
  }

  function removeSummary() {
    transcript.summary = null;
    changed();
  }

  async function startRename(index: number, speaker: number) {
    renameAt = index;
    renameValue = transcript.speakers[speaker];
    await tick();
    // Old name selected: typing simply replaces it.
    const input = document.getElementById("rename-input") as HTMLInputElement | null;
    input?.focus();
    input?.select();
  }

  function commitRename(speaker: number) {
    const name = renameValue.trim();
    if (name && name !== transcript.speakers[speaker]) {
      transcript.speakers[speaker] = name;
      changed();
    }
    renameAt = null;
  }

  async function playFrom(segment: Segment) {
    playbackNotice = null;
    if (!audioUrl) {
      audioUrl = transcript.sourcePath ? await api.playbackUrl(transcript.sourcePath) : null;
      if (!audioUrl) {
        playbackNotice = t("playbackMissing");
        return;
      }
      await tick();
    }
    if (!audio) return;
    playerVisible = true;
    audio.currentTime = segment.startMs / 1000;
    try {
      await audio.play();
    } catch {
      playbackNotice = t("playbackUnsupported");
      playerVisible = false;
    }
  }

  function togglePlay() {
    if (!audio) return;
    if (audio.paused) void audio.play();
    else audio.pause();
  }

  function closePlayer() {
    audio?.pause();
    playerVisible = false;
  }
</script>

<article class="transcript">
  <header>
    <div class="heading">
      <h2 title={transcript.sourcePath ?? transcript.title}>{transcript.title}</h2>
      {#if meta}<p class="muted meta">{meta}</p>{/if}
    </div>
    <div class="actions">
      <button class="btn secondary" class:on={findOpen} onclick={() => (findOpen = !findOpen)} disabled={!hasText}>
        <Icon name="search" />{t("searchReplace")}
      </button>
      <button class="btn secondary" onclick={summarize} disabled={!hasText || summarizing}>
        <Icon name={summarizing ? "spinner" : "sparkle"} spin={summarizing} />{t("summarize")}
      </button>
      <button class="btn secondary" onclick={copy} disabled={!hasText}>
        <Icon name={copied ? "check" : "copy"} />{copied ? t("copied") : t("copy")}
      </button>
    </div>
  </header>

  <div class="toolbar">
    <span class="muted">{t("saveAs")}</span>
    {#each primary as p (p.format)}
      <button class="btn primary" onclick={() => exportAs(p.format)} disabled={!hasText}>
        <Icon name="download" />{t(p.label)}
      </button>
    {/each}
    <ExportMenu onFormat={exportAs} onAll={exportAll} />
    <label class="switch stamps">
      <input type="checkbox" bind:checked={settings.timestamps} />
      <span class="track" aria-hidden="true"><span class="thumb"></span></span>
      <span>{t("timestampsShow")}</span>
    </label>
  </div>

  {#if notice}
    <div class="notice {notice.kind}" role={notice.kind === "danger" ? "alert" : "status"}>
      <Icon name={notice.kind === "success" ? "check" : "alert"} />
      <span>{notice.text}</span>
      {#if notice.reveal}
        <button class="link" onclick={() => api.revealFile(notice!.reveal!)}>{t("showInFolder")}</button>
      {/if}
    </div>
  {/if}

  {#if findOpen}
    <div class="find" role="search">
      <input class="input" placeholder={t("searchPlaceholder")} bind:value={query} oninput={() => (replacedCount = null)} />
      <input class="input" placeholder={t("replacePlaceholder")} bind:value={replacement} />
      <button class="btn primary" onclick={replaceAll} disabled={matches === 0}>
        {t("replaceAll", { count: matches })}
      </button>
      <span class="muted find-status" aria-live="polite">
        {#if replacedCount !== null}{t("replaced", { count: replacedCount })}{:else if query && matches === 0}{t("noMatches")}{/if}
      </span>
      <button class="icon-btn" aria-label={t("close")} onclick={() => (findOpen = false)}><Icon name="x" size={18} /></button>
    </div>
  {/if}

  {#if summarizing}
    <div class="summary pending" aria-live="polite">
      <p class="status-line">
        <Icon name="spinner" spin />
        {summaryStep && summaryStep.total > 1
          ? t("summarizingStep", { done: Math.min(summaryStep.done + 1, summaryStep.total), total: summaryStep.total })
          : t("summarizing")}
      </p>
    </div>
  {:else if summaryError}
    <div class="notice danger" role="alert"><Icon name="alert" /><span>{errorText(summaryError.code)}</span></div>
  {:else if transcript.summary != null}
    <section class="summary">
      <div class="summary-head">
        <h3><Icon name="sparkle" size={18} />{t("summaryTitle")}</h3>
        <div class="summary-actions">
          <button class="link" onclick={summarize}>{t("summaryRedo")}</button>
          <button class="link" onclick={removeSummary}>{t("summaryRemove")}</button>
        </div>
      </div>
      <div
        class="summary-text"
        contenteditable="true"
        spellcheck="true"
        bind:innerText={() => transcript.summary ?? "", (v) => (transcript.summary = v)}
        oninput={changed}
      ></div>
    </section>
  {/if}

  {#if playbackNotice}
    <div class="notice danger" role="alert"><Icon name="alert" /><span>{playbackNotice}</span></div>
  {/if}

  {#if hasText}
    <p class="muted hint">{t("resultEditHint")} {t("autosaveHint")}</p>
    <div class="document">
      {#each transcript.segments as segment, i (segment.startMs)}
        <div class="paragraph" class:playing={i === activeIndex}>
          <div class="labels">
            {#if settings.timestamps || transcript.sourcePath}
              <button class="stamp" title={t("play")} onclick={() => playFrom(segment)}>
                <Icon name="play" size={12} />
                {#if settings.timestamps}{formatStamp(segment.startMs)}{/if}
              </button>
            {/if}
            {#if segment.speaker != null && transcript.speakers[segment.speaker] != null}
              {@const speaker = segment.speaker}
              {#if renameAt === i}
                <input
                  id="rename-input"
                  class="input rename"
                  aria-label={t("renameHint")}
                  title={t("renameHint")}
                  bind:value={renameValue}
                  onkeydown={(e) => {
                    if (e.key === "Enter") commitRename(speaker);
                    if (e.key === "Escape") renameAt = null;
                  }}
                  onblur={() => commitRename(speaker)}
                />
              {:else}
                <button
                  class="speaker"
                  style:--spk="var(--spk-{speaker % 6})"
                  title={t("renameSpeaker")}
                  onclick={() => startRename(i, speaker)}
                >
                  {transcript.speakers[speaker]}
                  <Icon name="edit" size={12} />
                </button>
              {/if}
            {/if}
          </div>
          <p
            contenteditable="true"
            spellcheck="true"
            bind:innerText={transcript.segments[i].text}
            oninput={changed}
          ></p>
        </div>
      {/each}
    </div>
  {:else}
    <p class="muted empty">{t("resultEmpty")}</p>
  {/if}

  <audio
    bind:this={audio}
    src={audioUrl ?? undefined}
    preload="metadata"
    ontimeupdate={() => (currentMs = (audio?.currentTime ?? 0) * 1000)}
    onplay={() => (playing = true)}
    onpause={() => (playing = false)}
    onerror={() => {
      if (audioUrl) {
        playbackNotice = t("playbackUnsupported");
        playerVisible = false;
      }
    }}
  ></audio>

  {#if playerVisible}
    <div class="player" role="region" aria-label={t("play")}>
      <button class="icon-btn big" aria-label={playing ? t("pause") : t("resume")} onclick={togglePlay}>
        <Icon name={playing ? "pause" : "play"} size={20} />
      </button>
      <span class="time">{formatClock(currentMs)} / {formatClock(transcript.durationMs)}</span>
      <div class="bar small"><div class="fill" style:width="{(currentMs / Math.max(transcript.durationMs, 1)) * 100}%"></div></div>
      <button class="icon-btn" aria-label={t("stopPlayback")} onclick={closePlayer}><Icon name="x" size={18} /></button>
    </div>
  {/if}
</article>

<style>
  .transcript {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
    padding-bottom: 72px;
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .heading {
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: 22px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    margin: 2px 0 0;
    font-size: 14px;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .btn.on {
    border-color: var(--primary);
    color: var(--primary-strong);
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
  .stamps {
    margin-left: auto;
    font-size: 15px;
  }
  .find {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 12px;
    border-radius: 12px;
    background: var(--surface-2);
  }
  .find .input {
    flex: 1 1 180px;
  }
  .find-status {
    font-size: 14px;
    min-width: 80px;
  }
  .summary {
    padding: 18px 22px;
    border-radius: 14px;
    border: 1px solid var(--border);
    background: var(--primary-soft);
    max-width: 75ch;
  }
  .summary.pending {
    background: var(--surface-2);
  }
  .summary-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 6px;
  }
  .summary h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 16px;
    color: var(--primary-strong);
  }
  .summary-actions {
    display: flex;
    gap: 14px;
    font-size: 14px;
  }
  .summary-text {
    white-space: pre-wrap;
    line-height: 1.65;
    border-radius: 6px;
    outline: none;
  }
  .summary-text:focus-visible {
    box-shadow: 0 0 0 2px var(--focus);
  }
  .status-line {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
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
  .paragraph {
    border-radius: 8px;
    margin: 0 -8px;
    padding: 4px 8px;
    transition: background-color 150ms;
  }
  .paragraph + .paragraph {
    margin-top: 10px;
  }
  .paragraph.playing {
    background: var(--primary-soft);
  }
  .labels {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 24px;
  }
  .stamp,
  .speaker {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    margin-left: -6px;
    border: none;
    border-radius: 6px;
    background: none;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .stamp {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .speaker {
    font-weight: 650;
    color: var(--spk);
  }
  .speaker :global(.icon) {
    opacity: 0;
    transition: opacity 150ms;
  }
  .speaker:hover :global(.icon),
  .speaker:focus-visible :global(.icon) {
    opacity: 1;
  }
  .stamp:hover,
  .speaker:hover {
    background: var(--surface-2);
  }
  .stamp:focus-visible,
  .speaker:focus-visible {
    outline: 2px solid var(--focus);
  }
  .rename {
    min-height: 30px;
    font-size: 14px;
    width: 220px;
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
  .paragraph p:hover {
    background: var(--surface-2);
  }
  .paragraph p:focus-visible {
    background: var(--surface-2);
    box-shadow: 0 0 0 2px var(--focus);
  }
  .empty {
    text-align: center;
    padding: 24px;
  }
  .player {
    position: sticky;
    bottom: 12px;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 75ch;
    padding: 8px 12px;
    border-radius: 14px;
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: 0 8px 28px rgb(0 0 0 / 0.14);
  }
  .player .time {
    font-variant-numeric: tabular-nums;
    font-size: 14px;
    white-space: nowrap;
  }
  .player .bar {
    flex: 1;
    height: 6px;
  }
  .icon-btn.big {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--primary);
    color: var(--on-primary);
  }
  .icon-btn.big:hover {
    background: var(--primary-hover);
    color: var(--on-primary);
  }
</style>
