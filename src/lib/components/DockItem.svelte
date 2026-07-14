<script lang="ts">
  import { getConfig, getRunningApps, launchApp, markAppClosed } from '$lib/stores/dockStore.svelte';
  import type { DockApp } from '$lib/stores/dockStore.svelte';

  let { app, index, total } = $props<{
    app: DockApp;
    index: number;
    total: number;
  }>();

  let hovered = $state(false);
  let scale = $state(1);
  let contextMenuVisible = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let cachedCenterX = $state(0);

  let config = $derived(getConfig());
  const runningApps = $derived(getRunningApps());

  const iconSize = $derived(config.icon_size);
  const animSpeed = $derived(config.animation_speed);
  const showRunning = $derived(config.show_running_indicators);

  function handleMouseMove(e: MouseEvent) {
    const distance = Math.abs(e.clientX - cachedCenterX);
    const maxDist = config.icon_size * config.magnification_range;

    if (distance < maxDist) {
      const factor = 1 - distance / maxDist;
      const smooth = factor * factor * (3 - 2 * factor);
      scale = 1 + (config.magnification - 1) * smooth;
    } else {
      scale = 1;
    }
  }

  function handleMouseLeave() {
    hovered = false;
    scale = 1;
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
    contextMenuVisible = true;
  }

  function handleImgError(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    img.style.display = 'none';
    const parent = img.parentElement;
    if (parent) {
      let fb = parent.querySelector('.icon-fallback') as HTMLElement;
      if (fb) fb.style.display = 'flex';
    }
  }

  function handleClick() {
    launchApp(app);
  }

  function closeContextMenu() {
    contextMenuVisible = false;
  }

  function handleMenuAction(action: string) {
    closeContextMenu();
    if (action === 'open') {
      launchApp(app);
    } else if (action === 'close') {
      markAppClosed(app.id);
    }
  }

  $effect(() => {
    if (contextMenuVisible) {
      const handler = () => closeContextMenu();
      window.addEventListener('click', handler, { once: true });
      return () => window.removeEventListener('click', handler);
    }
  });
</script>

<div
  class="dock-item"
  class:hovered
  style="--icon-size: {iconSize}px"
  onmouseenter={(e) => {
    hovered = true;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    cachedCenterX = rect.left + rect.width / 2;
  }}
  onmouseleave={handleMouseLeave}
  onmousemove={handleMouseMove}
  onclick={handleClick}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleClick(); }}
  oncontextmenu={handleContextMenu}
  role="button"
  tabindex="0"
  aria-label={app.name}
>
  {#if showRunning && runningApps.has(app.id)}
    <div class="running-bar"></div>
  {/if}
  <div class="icon-wrapper" style:transform="scale({scale})" style:transition="transform {animSpeed}ms cubic-bezier(0.25, 0.46, 0.45, 0.94)">
    <img
      src={app.icon_path}
      alt={app.name}
      width={iconSize}
      height={iconSize}
      loading="lazy"
      draggable="false"
      onerror={handleImgError}
    />
    <div class="icon-fallback" style="display:none;width:{iconSize}px;height:{iconSize}px;border-radius:12px;background:rgba(255,255,255,0.15);align-items:center;justify-content:center;font-size:{iconSize * 0.5}px;color:rgba(255,255,255,0.7);">
      {app.name.charAt(0).toUpperCase()}
    </div>
  </div>
  {#if hovered}
    <div class="tooltip" style:top="{-iconSize * (scale - 1) - 30}px">{app.name}</div>
  {/if}
</div>

{#if contextMenuVisible}
  <div
    class="context-menu"
    role="menu"
    tabindex="0"
    style:left="{contextMenuX}px"
    style:top="{contextMenuY}px"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => { if (e.key === 'Escape') closeContextMenu(); }}
  >
    <button class="menu-item" onclick={() => handleMenuAction('open')}>打开</button>
    {#if runningApps.has(app.id)}
      <button class="menu-item" onclick={() => handleMenuAction('close')}>关闭</button>
    {/if}
  </div>
{/if}

<style>
  .dock-item {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0 4px;
    will-change: transform;
  }

  .icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    will-change: transform;
    transform-origin: bottom center;
    backface-visibility: hidden;
    -webkit-backface-visibility: hidden;
  }

  .icon-wrapper img {
    border-radius: 12px;
    object-fit: cover;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
  }

  .tooltip {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(30, 30, 30, 0.9);
    color: white;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    white-space: nowrap;
    pointer-events: none;
    backdrop-filter: blur(10px);
    z-index: 1000;
  }

  .running-bar {
    position: absolute;
    bottom: -6px;
    left: 50%;
    transform: translateX(-50%);
    width: calc(var(--icon-size, 48px) * 0.6);
    max-width: 60%;
    height: 4px;
    border-radius: 2px;
    background: rgba(75, 195, 255, 0.95);
    box-shadow: 0 0 8px rgba(120, 180, 255, 0.7), 0 1px 3px rgba(0, 0, 0, 0.3);
    z-index: 2;
  }

  .context-menu {
    position: fixed;
    background: rgba(40, 40, 40, 0.95);
    backdrop-filter: blur(20px);
    border-radius: 8px;
    padding: 4px 0;
    min-width: 120px;
    z-index: 9999;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .menu-item {
    display: block;
    width: 100%;
    padding: 8px 16px;
    background: none;
    border: none;
    color: white;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
  }

  .menu-item:hover {
    background: rgba(255, 255, 255, 0.1);
  }
</style>
