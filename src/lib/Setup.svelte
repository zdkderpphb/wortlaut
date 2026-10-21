<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { api, asCommandError, type CommandError, type DownloadProgress, type ModelInfo } from "./api";
  import { formatBytes, formatDuration, formatSpeed } from "./format";
  import { errorText, t } from "./i18n";
  import Icon from "./Icon.svelte";

  let { info, onReady }: { info: ModelInfo; onReady: () => void } = $props();

  let downloading = $state(false);
  let progress = $state<DownloadProgress | null>(null);
  let error = $state<CommandError | null>(null);

  const partialBytes = $derived(info.status.state === "partial" ? info.status.bytes : 0);
  /** Only a small addition is missing (e.g. new models after an update). */
  const isUpdate = $derived(partialBytes > 0 && info.sizeBytes - partialBytes < 100_000_000);
  /** In the update case only the missing part counts. */
  const base = $derived(isUpdate ? partialBytes : 0);
  const percent = $derived(progress ? ((progress.downloaded - base) / Math.max(progress.total - base, 1)) * 100 : 0);
  const remainingSecs = $derived(
    progress && progress.bytesPerSec > 0 ? (progress.total - progress.downloaded) / progress.bytesPerSec : null,
  );

  let unlisten: (() => void) | undefined;
  onMount(async () => {
    unlisten = await api.onDownloadProgress((p) => (progress = p));
  });
  onDestroy(() => unlisten?.());

  async function start() {
    error = null;
    downloading = true;
    progress = { downloaded: partialBytes, total: info.sizeBytes, bytesPerSec: 0, verifying: false };
    try {
      await api.downloadModel();
      onReady();
    } catch (e) {
      const err = asCommandError(e);
      if (err.code !== "cancelled") error = err;
      info = await api.modelInfo();
    } finally {
      downloading = false;
    }
  }

  function errorMessage(err: CommandError): string {
    const space = /need (\d+) bytes, (\d+) free/.exec(err.detail);
    return space
      ? errorText(err.code, { needed: formatBytes(+space[1]), free: formatBytes(+space[2]) })
      : errorText(err.code);
  }
</script>

<main class="setup">
  <div class="intro">
    <div class="logo"><Icon name="logo" size={34} /></div>
    <h1>{t("setupTitle")}</h1>
    <p class="lead">{t("setupIntro")}</p>
    <ul class="points">
      <li><Icon name="shield" />{t("setupPointPrivate")}</li>
      <li><Icon name="fileText" />{t("setupPointFormats")}</li>
      <li><Icon name="globe" />{t("setupPointLanguages")}</li>
    </ul>
  </div>

  <section class="card" aria-live="polite">
    {#if downloading && progress}
      <h2>{progress.verifying ? t("setupVerifying") : isUpdate ? t("setupUpdateDownloading") : t("setupDownloading")}</h2>
      <div
        class="bar"
        class:indeterminate={progress.verifying}
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(percent)}
      >
        <div class="fill" style:width="{percent}%"></div>
      </div>
      {#if !progress.verifying}
        <p class="numbers">
          <span>
            {t("setupProgress", {
              done: formatBytes(progress.downloaded - base),
              total: formatBytes(progress.total - base),
              speed: formatSpeed(progress.bytesPerSec),
            })}
          </span>
          {#if remainingSecs !== null}
            <span>{t("setupRemaining", { time: formatDuration(remainingSecs) })}</span>
          {/if}
        </p>
        <p class="hint">{t("setupKeepOpen")}</p>
        <button class="btn secondary" onclick={() => api.cancelDownload()}>
          <Icon name="pause" />{t("setupPause")}
        </button>
      {/if}
    {:else if isUpdate}
      <h2>{t("setupUpdateTitle")}</h2>
      <p>{t("setupUpdateText", { size: formatBytes(info.sizeBytes - partialBytes) })}</p>
    {:else}
      <h2>{t("setupCardTitle")}</h2>
      <p>{t("setupCardText", { size: formatBytes(info.sizeBytes) })}</p>
      <p class="hint">{t("setupFreeSpace", { free: formatBytes(info.freeBytes) })}</p>
    {/if}
    {#if !(downloading && progress)}

      {#if error}
        <div class="error" role="alert">
          <Icon name="alert" />
          <div>
            <p>{errorMessage(error)}</p>
            <details>
              <summary>{t("errorDetails")}</summary>
              <code>{error.detail}</code>
            </details>
          </div>
        </div>
      {/if}

      {#if partialBytes > 0 && !isUpdate}
        <p class="hint">
          {t("setupResumeHint", { done: formatBytes(partialBytes), total: formatBytes(info.sizeBytes) })}
        </p>
      {/if}
      <button class="btn primary large" onclick={start}>
        <Icon name={error ? "retry" : "download"} />
        {error ? t("setupRetry") : isUpdate ? t("setupUpdateStart") : partialBytes > 0 ? t("setupResume") : t("setupStart")}
      </button>
    {/if}
  </section>
</main>

<style>
  .setup {
    max-width: 640px;
    margin: 0 auto;
    padding: 56px 32px 40px;
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .intro {
    text-align: center;
  }
  .logo {
    display: inline-grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: 18px;
    background: var(--primary);
    color: var(--on-primary);
    margin-bottom: 20px;
  }
  h1 {
    font-size: 30px;
    margin: 0 0 12px;
  }
  .lead {
    font-size: 18px;
    color: var(--muted);
    margin: 0 auto 24px;
    max-width: 52ch;
  }
  .points {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 10px 24px;
  }
  .points li {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text);
  }
  .points :global(.icon) {
    color: var(--primary-strong);
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 28px;
    box-shadow: var(--shadow);
  }
  h2 {
    font-size: 20px;
    margin: 0 0 8px;
  }
  .card p {
    margin: 0 0 12px;
  }
  .hint {
    color: var(--muted);
    font-size: 15px;
  }
  .numbers {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    font-variant-numeric: tabular-nums;
  }
  .card .btn {
    margin-top: 8px;
  }
  .error {
    display: flex;
    gap: 12px;
    padding: 14px 16px;
    border-radius: 12px;
    background: var(--danger-soft);
    color: var(--danger);
    margin-bottom: 16px;
  }
  .error p {
    margin: 0 0 4px;
    color: var(--text);
  }
</style>
