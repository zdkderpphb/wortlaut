<script lang="ts">
  import { languages, t } from "./i18n";
  import Icon from "./Icon.svelte";
  import { settings } from "./settings.svelte";

  let { dragging, onPick, compact = false }: { dragging: boolean; onPick: () => void; compact?: boolean } = $props();

  const counts = [2, 3, 4, 5, 6, 7, 8, 9, 10];
</script>

<div class="new" class:compact>
  <button class="dropzone" class:active={dragging} onclick={onPick}>
    <span class="drop-icon"><Icon name="upload" size={36} /></span>
    <span class="drop-title">{dragging ? t("dropActive") : t("dropTitle")}</span>
    <span class="drop-subtitle">{t("dropSubtitle")}</span>
    <span class="drop-formats">{t("dropFormats")}</span>
  </button>

  <div class="options">
    <label class="field">
      <span>{t("languageLabel")}</span>
      <select class="select" bind:value={settings.language}>
        {#each languages as l (l.code)}
          <option value={l.code}>{t(l.label)}</option>
        {/each}
      </select>
    </label>

    <div class="speakers">
      <label class="switch">
        <input type="checkbox" bind:checked={settings.detectSpeakers} />
        <span class="track" aria-hidden="true"><span class="thumb"></span></span>
        <span class="switch-text">
          <span>{t("speakersLabel")}</span>
          <small>{t("speakersHint")}</small>
        </span>
      </label>
      {#if settings.detectSpeakers}
        <label class="count">
          <span class="muted">{t("speakerCountLabel")}</span>
          <select
            class="select small"
            value={settings.speakerCount ?? "auto"}
            onchange={(e) => {
              const v = e.currentTarget.value;
              settings.speakerCount = v === "auto" ? null : Number(v);
            }}
          >
            <option value="auto">{t("speakerCountAuto")}</option>
            {#each counts as n (n)}
              <option value={n}>{n}</option>
            {/each}
          </select>
        </label>
      {/if}
    </div>
  </div>

  <p class="muted privacy"><Icon name="shield" size={16} />{t("footerPrivacy")}</p>
</div>

<style>
  .new {
    width: 100%;
    max-width: 720px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 24px;
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
  .compact .dropzone {
    padding: 40px 24px;
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
  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 16px 32px;
    align-items: flex-end;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 14px;
    color: var(--muted);
  }
  .speakers {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 16px;
  }
  .count {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
  }
  .privacy {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin: 0;
    font-size: 14px;
  }
</style>
