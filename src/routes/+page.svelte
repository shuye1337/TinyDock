<script lang="ts">
  import { onMount } from 'svelte';
  import { getConfig, loadConfig, setupEventListeners } from '$lib/stores/dockStore.svelte';
  import DockBar from '$lib/components/DockBar.svelte';

  let config = $derived(getConfig());

  let apps = $derived(config.apps);

  onMount(() => {
    let cleanup: (() => Promise<void>) | undefined;
    setupEventListeners().then((fn) => (cleanup = fn));
    loadConfig();
    return () => {
      cleanup?.();
    };
  });
</script>

{#if apps.length > 0}
  <DockBar {apps} />
{:else}
  <div class="empty-dock">
    <span>右键托盘图标 → 打开设置添加应用</span>
  </div>
{/if}

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: transparent !important;
  }

  :global(html) {
    background: transparent !important;
  }

  .empty-dock {
    position: fixed;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    padding: 12px 24px;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.3);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.8);
    font-size: 13px;
    white-space: nowrap;
  }
</style>
