<script lang="ts">
  import "../app.css";

  let { children } = $props();

  // macOS: the window content runs under the traffic lights (overlay title bar).
  if (typeof navigator !== "undefined" && /Mac/.test(navigator.userAgent)) {
    document.documentElement.classList.add("mac");
  }
</script>

{@render children()}
