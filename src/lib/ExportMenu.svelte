<script lang="ts">
  import type { ExportFormat } from "./api";
  import { t, type MessageKey } from "./i18n";
  import Icon from "./Icon.svelte";

  let { onFormat, onAll }: { onFormat: (f: ExportFormat) => void; onAll: () => void } = $props();

  const items: { format: ExportFormat; label: MessageKey; hint: MessageKey }[] = [
    { format: "srt", label: "formatSrt", hint: "formatSrtHint" },
    { format: "maxqda", label: "formatMaxqda", hint: "formatQdaHint" },
    { format: "atlasti", label: "formatAtlasti", hint: "formatQdaHint" },
    { format: "nvivo", label: "formatNvivo", hint: "formatQdaHint" },
    { format: "json", label: "formatJson", hint: "formatJsonHint" },
  ];

  let open = $state(false);
  let root: HTMLDivElement;

  function choose(action: () => void) {
    open = false;
    action();
  }

  function onWindowClick(e: MouseEvent) {
    if (open && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<div class="menu" bind:this={root}>
  <button class="btn secondary" aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    {t("moreFormats")}<Icon name="chevronDown" size={18} />
  </button>
  {#if open}
    <div class="popover" role="menu">
      {#each items as item (item.format)}
        <button class="item" role="menuitem" onclick={() => choose(() => onFormat(item.format))}>
          <span>{t(item.label)}</span>
          <small>{t(item.hint)}</small>
        </button>
      {/each}
      <hr />
      <button class="item" role="menuitem" onclick={() => choose(onAll)}>
        <span><Icon name="folder" size={16} /> {t("exportAll")}</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .menu {
    position: relative;
  }
  .popover {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 20;
    min-width: 280px;
    padding: 6px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: 0 8px 28px rgb(0 0 0 / 0.16);
  }
  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    min-height: 44px;
    padding: 8px 12px;
    border: none;
    border-radius: 8px;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .item span {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .item small {
    color: var(--muted);
    font-size: 13px;
  }
  .item:hover,
  .item:focus-visible {
    background: var(--surface-2);
    outline: none;
  }
  hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 6px 4px;
  }
</style>
