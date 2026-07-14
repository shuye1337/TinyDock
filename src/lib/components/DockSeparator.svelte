<script lang="ts">
  import { getConfig } from '$lib/stores/dockStore.svelte';

  let config = $derived(getConfig());
</script>

<div class="separator" style:height="{config.icon_size * 0.7}px"></div>

<style>
  .separator {
    width: 1px;
    margin: 0 8px;
    background: rgba(255, 255, 255, 0.25);
    align-self: center;
    flex-shrink: 0;
  }
</style>
