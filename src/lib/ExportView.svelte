<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, asCommandError, exportFiles, type ExportFormat, type ExportLabels, type Transcript } from "./api";
  import { fileName } from "./format";
  import { errorText, t, type MessageKey } from "./i18n";
  import Icon, { type IconName } from "./Icon.svelte";
  import { settings } from "./settings.svelte";

  let { transcript, onBack }: { transcript: Transcript; onBack: () => void } = $props();

  interface Option {
    format: ExportFormat;
    label: MessageKey;
    hint: MessageKey;
    color: string;
    icon?: IconName;
    monogram?: string;
  }

  const groups: { title: MessageKey; footer?: MessageKey; options: Option[] }[] = [
    {
      title: "groupDocument",
      options: [
        { format: "docx", label: "saveWord", hint: "formatWordHint", color: "#2f6fde", icon: "fileText" },
        { format: "txt", label: "saveText", hint: "formatTextHint", color: "#8e8e93", icon: "file" },
        { format: "md", label: "saveMarkdown", hint: "formatMarkdownHint", color: "#3a3a3c", icon: "hash" },
      ],
    },
    {
      title: "groupQda",
      footer: "groupQdaFooter",
      options: [
        { format: "maxqda", label: "formatMaxqda", hint: "formatMaxqdaHint", color: "#d9572b", monogram: "M" },
        { format: "atlasti", label: "formatAtlasti", hint: "formatAtlastiHint", color: "#2c7a7b", monogram: "A" },
        { format: "nvivo", label: "formatNvivo", hint: "formatNvivoHint", color: "#7c3aed", monogram: "N" },
      ],
    },
    {
      title: "groupMore",
      options: [
        { format: "srt", label: "formatSrt", hint: "formatSrtHint", color: "#af52de", icon: "captions" },
        { format: "json", label: "formatJson", hint: "formatJsonHint", color: "#636366", icon: "code" },
      ],
    },
  ];

  const labels: ExportLabels = { summary: t("summaryTitle"), transcript: t("transcriptHeading") };

  let notice = $state<{ kind: "success" | "danger"; text: string; reveal?: string } | null>(null);

  function snapshot(): Transcript {
    return $state.snapshot(transcript) as Transcript;
  }

  /** Folder of the recording, so exports land next to it. */
  function sourceDir(): string {
    const p = transcript.sourcePath;
    return p ? p.slice(0, p.length - fileName(p).length) : "";
  }

  async function exportAs(option: Option) {
    const { ext, suffix } = exportFiles[option.format];
    const path = await save({
      defaultPath: `${sourceDir()}${transcript.title}${suffix}.${ext}`,
      filters: [{ name: t(option.label), extensions: [ext] }],
    });
    if (!path) return;
    try {
      await api.exportTranscript(snapshot(), option.format, settings.timestamps, labels, path);
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
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onBack()} />

<div class="export">
  <button class="back" onclick={onBack}>
    <Icon name="chevronLeft" size={18} />{t("back")}
  </button>

  <header>
    <h1>{t("exportTitle")}</h1>
    <p class="muted">{t("exportSubtitle")}</p>
  </header>

  {#if notice}
    <div class="notice {notice.kind}" role={notice.kind === "danger" ? "alert" : "status"}>
      <Icon name={notice.kind === "success" ? "check" : "alert"} />
      <span>{notice.text}</span>
      {#if notice.reveal}
        <button class="link" onclick={() => api.revealFile(notice!.reveal!)}>{t("showInFolder")}</button>
      {/if}
    </div>
  {/if}

  {#each groups as group (group.title)}
    <section>
      <h2 class="group-title">{t(group.title)}</h2>
      <div class="group">
        {#each group.options as option (option.format)}
          <button class="row" onclick={() => exportAs(option)}>
            <span class="row-icon" style:background={option.color}>
              {#if option.icon}<Icon name={option.icon} size={17} />{:else}<b>{option.monogram}</b>{/if}
            </span>
            <span class="row-text">
              {t(option.label)}
              <small>{t(option.hint)}</small>
            </span>
            <span class="row-meta">.{exportFiles[option.format].ext}</span>
            <span class="chevron"><Icon name="chevronRight" size={16} /></span>
          </button>
        {/each}
        {#if group.title === "groupMore"}
          <button class="row" onclick={exportAll}>
            <span class="row-icon" style:background="#0a84ff"><Icon name="folder" size={17} /></span>
            <span class="row-text">
              {t("exportAll")}
              <small>{t("formatAllHint")}</small>
            </span>
            <span class="chevron"><Icon name="chevronRight" size={16} /></span>
          </button>
        {/if}
      </div>
      {#if group.footer}<p class="group-footer">{t(group.footer)}</p>{/if}
    </section>
  {/each}

  <section>
    <h2 class="group-title">{t("groupOptions")}</h2>
    <div class="group">
      <label class="row switch">
        <span class="row-text">
          {t("timestampsLabel")}
          <small>{t("timestampsExportHint")}</small>
        </span>
        <input type="checkbox" bind:checked={settings.timestamps} />
        <span class="track" aria-hidden="true"><span class="thumb"></span></span>
      </label>
    </div>
  </section>
</div>

<style>
  .export {
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
    padding-bottom: 24px;
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-left: -6px;
    padding: 4px 8px 4px 2px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--primary-strong);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
  }
  .back:hover {
    background: var(--hover);
  }
  .back:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--focus);
  }
  header {
    margin: 10px 0 6px;
  }
  h1 {
    margin: 0 0 2px;
    font-size: 26px;
    font-weight: 700;
  }
  header p {
    margin: 0;
  }
  .notice {
    margin-top: 14px;
  }
  .row-icon b {
    font-size: 15px;
    font-weight: 700;
  }
  .row.switch {
    display: flex;
    cursor: pointer;
  }
</style>
