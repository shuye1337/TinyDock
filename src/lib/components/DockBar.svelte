<script lang="ts">
  import { getConfig, getRunningApps, isDockVisible, launchApp } from '$lib/stores/dockStore.svelte';
  import DockItem from './DockItem.svelte';
  import DockSeparator from './DockSeparator.svelte';
  import type { DockApp } from '$lib/stores/dockStore.svelte';

  let { apps } = $props<{ apps: DockApp[] }>();

  let config = $derived(getConfig());
  let dockVisible = $derived(isDockVisible());

  let mouseNearBottom = $state(false);
  let autoHideEnabled = $derived(config.auto_hide);

  $effect(() => {
    if (!autoHideEnabled) return;

    const threshold = config.icon_size * 2;
    let rafId: number;

    const handler = (e: MouseEvent) => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        const nearBottom = window.innerHeight - e.clientY < threshold;
        mouseNearBottom = nearBottom;
      });
    };

    window.addEventListener('mousemove', handler, { passive: true });
    const handleLeave = () => { mouseNearBottom = false; };
    document.documentElement.addEventListener('mouseleave', handleLeave);
    return () => {
      window.removeEventListener('mousemove', handler);
      document.documentElement.removeEventListener('mouseleave', handleLeave);
      cancelAnimationFrame(rafId);
    };
  });

  let barTransform = $derived(
    !dockVisible
      ? 'translateX(-50%) translateY(100%)'
      : (autoHideEnabled && !mouseNearBottom
          ? 'translateX(-50%) translateY(100%)'
          : 'translateX(-50%) translateY(0)')
  );
</script>

<div
  class="dock-bar"
  class:auto-hide={autoHideEnabled}
  style:--bg-opacity="{config.opacity}"
  style:--blur-amount="{config.blur}px"
  style:--anim-speed="{config.animation_speed}ms"
  style:--dock-margin="{config.dock_margin}px"
  style:transform="{barTransform}"
  style:transition="transform {config.animation_speed}ms cubic-bezier(0.25, 0.46, 0.45, 0.94)"
>
  <div class="dock-inner-wrap">
    <div class="dock-inner">
      {#each apps as app, i (app.id)}
        <DockItem {app} index={i} total={apps.length} />
        {#if i === apps.length - 2}
          <DockSeparator />
        {/if}
      {/each}
    </div>
  </div>
</div>

<style>
  .dock-bar {
    position: fixed;
    /* Bar pinned to the *window* bottom so `translateY(100%)` auto-hide
     * fully moves it off-screen. The visual gap above the taskbar is created
     * by `padding-bottom` on the inner wrapper (not by moving the element up,
     * which would break translateY hiding). */
    bottom: 0;
    left: 50%;
    z-index: 9998;
    will-change: transform;
  }

  .dock-inner-wrap {
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }

  .dock-inner {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px 12px;
    border-radius: 18px;
    background: rgba(255, 255, 255, var(--bg-opacity, 0.45));
    backdrop-filter: blur(var(--blur-amount, 30px));
    -webkit-backdrop-filter: blur(var(--blur-amount, 30px));
    border: 1px solid rgba(255, 255, 255, 0.2);
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.2),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
    transition: transform var(--anim-speed, 150ms) cubic-bezier(0.25, 0.46, 0.45, 0.94);
  }

  .dock-bar.auto-hide .dock-inner {
    transition: transform var(--anim-speed, 150ms) cubic-bezier(0.25, 0.46, 0.45, 0.94);
  }
</style>
