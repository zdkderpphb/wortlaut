<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { api, asCommandError, type CommandError, type Segment, type Transcript } from "./api";
  import ExportView from "./ExportView.svelte";
  import { formatClock } from "./format";
  import { errorText, locale, t } from "./i18n";
  import Icon from "./Icon.svelte";
  import { library } from "./library.svelte";
  import { settings } from "./settings.svelte";
  import { countInTranscript, plainText, replaceInTranscript } from "./text";

  let { transcript }: { transcript: Transcript } = $props();

  const dateFormat = new Intl.DateTimeFormat(locale === "de" ? "de-AT" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  });

  /** "export" shows the full-page format chooser instead of the text. */
  let page = $state<"transcript" | "export">("transcript");
  let tab = $state<"text" | "summary">("text");
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
    page = "transcript";
    tab = "text";
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

  async function copy() {
    const text = tab === "summary" ? (transcript.summary ?? "") : plainText(transcript, settings.timestamps);
    await navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  function replaceAll() {
    replacedCount = replaceInTranscript(transcript, query, replacement);
    if (replacedCount) changed();
  }

  async function summarize() {
    const target = transcript;
    tab = "summary";
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

{#if page === "export"}
  <ExportView {transcript} onBack={() => (page = "transcript")} />
{:else}
  <article class="transcript">
    <header class="head">
      <div class="title">
        <h1 title={transcript.sourcePath ?? transcript.title}>{transcript.title}</h1>
        {#if meta}<p class="meta">{meta}</p>{/if}
      </div>
      <button class="btn primary save" onclick={() => (page = "export")} disabled={!hasText}>
        <Icon name="download" size={17} />{t("saveButton")}
      </button>
    </header>

    <div class="toolbar">
      <div class="segmented" role="tablist">
        <button role="tab" aria-selected={tab === "text"} onclick={() => (tab = "text")}>{t("tabTranscript")}</button>
        <button role="tab" aria-selected={tab === "summary"} onclick={() => (tab = "summary")}>{t("tabSummary")}</button>
      </div>
      <div class="tools">
        {#if tab === "text"}
          <button class="btn plain" class:on={findOpen} onclick={() => (findOpen = !findOpen)} disabled={!hasText}>
            <Icon name="search" size={16} />{t("search")}
          </button>
        {/if}
        <button
          class="btn plain"
          onclick={copy}
          disabled={tab === "summary" ? !transcript.summary : !hasText}
        >
          <Icon name={copied ? "check" : "copy"} size={16} />{copied ? t("copied") : t("copy")}
        </button>
      </div>
    </div>

    {#if findOpen && tab === "text"}
      <div class="find" role="search">
        <span class="field">
          <Icon name="search" size={15} />
          <input placeholder={t("searchPlaceholder")} bind:value={query} oninput={() => (replacedCount = null)} />
        </span>
        <span class="field">
          <input placeholder={t("replacePlaceholder")} bind:value={replacement} />
        </span>
        <button class="btn" onclick={replaceAll} disabled={matches === 0}>
          {t("replaceAll", { count: matches })}
        </button>
        <span class="find-status" aria-live="polite">
          {#if replacedCount !== null}{t("replaced", { count: replacedCount })}{:else if query && matches === 0}{t("noMatches")}{/if}
        </span>
        <button class="icon-btn" aria-label={t("close")} onclick={() => (findOpen = false)}><Icon name="x" size={16} /></button>
      </div>
    {/if}

    {#if playbackNotice}
      <div class="notice danger" role="alert"><Icon name="alert" /><span>{playbackNotice}</span></div>
    {/if}

    {#if tab === "text"}
      {#if hasText}
        <div class="paper">
          {#each transcript.segments as segment, i (segment.startMs)}
            <div class="paragraph" class:playing={i === activeIndex}>
              <div class="labels">
                <button class="stamp" title={t("play")} aria-label={t("play")} onclick={() => playFrom(segment)}>
                  <Icon name="play" size={11} />{formatClock(segment.startMs)}
                </button>
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
                      <Icon name="edit" size={11} />
                    </button>
                  {/if}
                {/if}
              </div>
              <p contenteditable="true" spellcheck="true" bind:innerText={transcript.segments[i].text} oninput={changed}></p>
            </div>
          {/each}
        </div>
        <p class="hint">{t("resultEditHint")} {t("autosaveHint")}</p>
      {:else}
        <p class="muted empty">{t("resultEmpty")}</p>
      {/if}
    {:else if summarizing}
      <div class="summary-state" aria-live="polite">
        <Icon name="spinner" spin size={22} />
        <p>
          {summaryStep && summaryStep.total > 1
            ? t("summarizingStep", { done: Math.min(summaryStep.done + 1, summaryStep.total), total: summaryStep.total })
            : t("summarizing")}
        </p>
      </div>
    {:else if transcript.summary != null}
      <div class="paper">
        <div class="summary-head">
          <span class="summary-label"><Icon name="sparkle" size={15} />{t("summaryTitle")}</span>
          <span class="summary-actions">
            <button class="link" onclick={summarize}>{t("summaryRedo")}</button>
            <button class="link" onclick={removeSummary}>{t("summaryRemove")}</button>
          </span>
        </div>
        <div
          class="summary-text"
          contenteditable="true"
          spellcheck="true"
          bind:innerText={() => transcript.summary ?? "", (v) => (transcript.summary = v)}
          oninput={changed}
        ></div>
      </div>
      {#if summaryError}
        <div class="notice danger" role="alert"><Icon name="alert" /><span>{errorText(summaryError.code)}</span></div>
      {/if}
    {:else}
      <div class="summary-state">
        <span class="summary-icon"><Icon name="sparkle" size={26} /></span>
        <h2>{t("summaryEmptyTitle")}</h2>
        <p class="muted">{t("summaryEmptyText")}</p>
        <button class="btn" onclick={summarize} disabled={!hasText}>
          <Icon name="sparkle" size={16} />{t("summaryCreate")}
        </button>
        {#if summaryError}
          <div class="notice danger" role="alert"><Icon name="alert" /><span>{errorText(summaryError.code)}</span></div>
        {/if}
      </div>
    {/if}

    {#if playerVisible}
      <div class="player" role="region" aria-label={t("play")}>
        <button class="play-btn" aria-label={playing ? t("pause") : t("resume")} onclick={togglePlay}>
          <Icon name={playing ? "pause" : "play"} size={16} />
        </button>
        <span class="time">{formatClock(currentMs)}</span>
        <div class="bar"><div class="fill" style:width="{(currentMs / Math.max(transcript.durationMs, 1)) * 100}%"></div></div>
        <span class="time muted">{formatClock(transcript.durationMs)}</span>
        <button class="icon-btn" aria-label={t("stopPlayback")} onclick={closePlayer}><Icon name="x" size={16} /></button>
      </div>
    {/if}
  </article>
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

<style>
  .transcript {
    width: 100%;
    max-width: 780px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-bottom: 64px;
  }
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
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
  .save {
    flex-shrink: 0;
    margin-top: 2px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .tools {
    display: flex;
    gap: 2px;
  }
  .btn.plain.on {
    background: var(--selected);
    color: var(--text);
  }
  .find {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 8px;
    border-radius: 12px;
    background: var(--surface-2);
  }
  .field {
    flex: 1 1 180px;
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 32px;
    padding: 0 10px;
    border-radius: 8px;
    background: var(--surface);
    border: 0.5px solid var(--border);
    color: var(--faint);
  }
  .field:focus-within {
    box-shadow: 0 0 0 3px var(--focus);
  }
  .field input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 14px;
  }
  .find-status {
    font-size: 13px;
    color: var(--muted);
    min-width: 70px;
  }
  .paper {
    background: var(--surface);
    border-radius: 14px;
    border: 0.5px solid var(--border);
    box-shadow: var(--shadow);
    padding: 26px 34px;
  }
  .paragraph {
    margin: 0 -10px;
    padding: 6px 10px;
    border-radius: 10px;
    transition: background-color 150ms;
  }
  .paragraph + .paragraph {
    margin-top: 8px;
  }
  .paragraph.playing {
    background: var(--primary-soft);
  }
  .labels {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 22px;
    margin-left: -6px;
  }
  .stamp,
  .speaker {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    border: none;
    border-radius: 6px;
    background: none;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .stamp {
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .speaker {
    font-weight: 600;
    color: var(--spk);
  }
  .speaker :global(.icon) {
    opacity: 0;
    transition: opacity 120ms;
  }
  .speaker:hover :global(.icon),
  .speaker:focus-visible :global(.icon) {
    opacity: 0.8;
  }
  .stamp:hover,
  .speaker:hover {
    background: var(--hover);
  }
  .stamp:hover {
    color: var(--primary-strong);
  }
  .stamp:focus-visible,
  .speaker:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--focus);
  }
  .rename {
    width: 220px;
    font-size: 13px;
  }
  .paragraph p {
    margin: 0;
    font-size: 16px;
    line-height: 1.65;
    white-space: pre-wrap;
    border-radius: 6px;
    outline: none;
  }
  .paragraph p:focus-visible {
    box-shadow: 0 0 0 3px var(--focus);
  }
  .hint {
    margin: 0;
    text-align: center;
    font-size: 12px;
    color: var(--faint);
  }
  .empty {
    text-align: center;
    padding: 32px;
  }
  .summary-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 48px 24px;
    text-align: center;
    color: var(--muted);
  }
  .summary-state h2 {
    margin: 6px 0 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }
  .summary-state p {
    margin: 0 0 8px;
    max-width: 44ch;
  }
  .summary-icon {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--primary-soft);
    color: var(--primary-strong);
  }
  .summary-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .summary-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--primary-strong);
  }
  .summary-actions {
    display: flex;
    gap: 14px;
    font-size: 13px;
  }
  .summary-text {
    white-space: pre-wrap;
    font-size: 16px;
    line-height: 1.65;
    border-radius: 6px;
    outline: none;
  }
  .summary-text:focus-visible {
    box-shadow: 0 0 0 3px var(--focus);
  }
  .player {
    position: sticky;
    bottom: 16px;
    z-index: 10;
    align-self: center;
    display: flex;
    align-items: center;
    gap: 10px;
    width: min(520px, 100%);
    padding: 8px 10px 8px 8px;
    border-radius: 999px;
    background: var(--surface);
    box-shadow: var(--shadow-float);
  }
  .player .bar {
    flex: 1;
    height: 4px;
  }
  .time {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .play-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 50%;
    background: var(--primary);
    color: var(--on-primary);
    cursor: pointer;
  }
  .play-btn:hover {
    background: var(--primary-hover);
  }
</style>
