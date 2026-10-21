<script lang="ts">
  import { onMount } from "svelte";
  import { api, type ModelInfo } from "$lib/api";
  import Home from "$lib/Home.svelte";
  import { locale } from "$lib/i18n";
  import { queue } from "$lib/jobs.svelte";
  import Setup from "$lib/Setup.svelte";

  let info = $state<ModelInfo | null>(null);

  async function refresh() {
    info = await api.modelInfo();
  }

  onMount(() => {
    document.documentElement.lang = locale;
    queue.onModelMissing = refresh;
    void refresh();
  });
</script>

{#if info?.status.state === "ready"}
  <Home />
{:else if info}
  <Setup {info} onReady={refresh} />
{/if}
