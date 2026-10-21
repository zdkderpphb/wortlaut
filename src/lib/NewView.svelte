<script lang="ts">
  import { languages, t } from "./i18n";
  import Icon from "./Icon.svelte";
  import { settings } from "./settings.svelte";

  let { dragging, onPick }: { dragging: boolean; onPick: () => void } = $props();

  const counts = [2, 3, 4, 5, 6, 7, 8, 9, 10];
</script>

<div class="new">
  <button class="dropzone" class:active={dragging} onclick={onPick}>
    <span class="drop-icon"><Icon name="upload" size={30} /></span>
    <span class="drop-title">{dragging ? t("dropActive") : t("dropTitle")}</span>
    <span class="drop-subtitle">{t("dropSubtitle")}</span>
    <span class="btn choose">{t("chooseFile")}</span>
    <span class="drop-formats">{t("dropFormats")}</span>
  </button>

  <div>
    <h2 class="group-title">{t("groupOptions")}</h2>
    <div class="group">
      <label class="row">
        <span class="row-text">{t("languageLabel")}</span>
        <select class="select" bind:value={settings.language}>
          {#each languages as l (l.code)}
            <option value={l.code}>{t(l.label)}</option>
          {/each}
        </select>
      </label>
      <label class="row switch">
        <span class="row-text">
          {t("speakersLabel")}
          <small>{t("speakersHint")}</small>
        </span>
        <input type="checkbox" bind:checked={settings.detectSpeakers} />
        <span class="track" aria-hidden="true"><span class="thumb"></span></span>
      </label>
      {#if settings.detectSpeakers}
        <label class="row">
          <span class="row-text">{t("speakerCountLabel")}</span>
          <select
            class="select"
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
</div>

<style>
  .new {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 40px 24px 28px;
    border: 1.5px dashed var(--border-strong);
    border-radius: 16px;
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
    outline: none;
    box-shadow: 0 0 0 3px var(--focus);
  }
  .drop-icon {
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: 50%;
    background: var(--primary-soft);
    color: var(--primary-strong);
    margin-bottom: 6px;
  }
  .drop-title {
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.015em;
  }
  .drop-subtitle {
    font-size: 14px;
    color: var(--muted);
  }
  .choose {
    margin-top: 12px;
  }
  .drop-formats {
    margin-top: 14px;
    font-size: 12px;
    color: var(--faint);
  }
  .row.switch {
    display: flex;
    cursor: pointer;
  }
</style>
