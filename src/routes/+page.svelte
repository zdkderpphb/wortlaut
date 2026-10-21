<script lang="ts">
  import { onMount } from "svelte";
  import { api, type ModelInfo } from "$lib/api";
  import Home from "$lib/Home.svelte";
  import { locale } from "$lib/i18n";
  import { library } from "$lib/library.svelte";
  import Setup from "$lib/Setup.svelte";

  let info = $state<ModelInfo | null>(null);
  let historyLoaded = $state(false);

  async function refresh() {
    info = await api.modelInfo();
    if (info.status.state === "ready" && !historyLoaded) {
      await library.init().catch(() => {});
      historyLoaded = true;
    }
  }

  onMount(() => {
    document.documentElement.lang = locale;
    library.onModelMissing = refresh;
    void refresh();
  });
</script>

{#if info?.status.state === "ready"}
  {#if historyLoaded}<Home />{/if}
{:else if info}
  <Setup {info} onReady={refresh} />
{/if}
