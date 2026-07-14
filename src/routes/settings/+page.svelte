<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    getConfig,
    loadConfig,
    updateConfig,
    saveConfigNow,
    addApp,
    removeApp,
    getSaveStatus,
  } from '$lib/stores/dockStore.svelte';

  let config = $derived(getConfig());
  const saveStatus = $derived(getSaveStatus());

  let newAppName = $state('');
  let newAppIcon = $state('');
  let newAppCommand = $state('');
  const animSpeed = $derived(config.animation_speed);

  async function handleAddApp() {
    if (!newAppName || !newAppCommand) return;
    addApp({
      name: newAppName,
      icon_path: newAppIcon || 'icons/32x32.png',
      command: newAppCommand,
    });
    newAppName = '';
    newAppIcon = '';
    newAppCommand = '';
  }

  async function handlePickIcon() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Images', extensions: ['png', 'ico', 'jpg', 'jpeg', 'svg'] }],
    });
    if (selected) {
      newAppIcon = selected as string;
    }
  }

  async function handlePickCommand() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Executables', extensions: ['exe'] }],
    });
    if (selected) {
      newAppCommand = selected as string;
      // 自动提取图标
      try {
        const dataUrl = await invoke<string>('extract_icon', { path: selected });
        newAppIcon = dataUrl;
      } catch { /* 提取失败，保留当前图标 */ }
    }
  }

  async function handleQuickAddLnk() {
    const selected = await open({
      multiple: false,
      filters: [{ name: '快捷方式', extensions: ['lnk'] }],
    });
    if (!selected) return;

    const lnkPath = selected as string;

    try {
      const info = await invoke<{ name: string; target: string }>('parse_lnk', { path: lnkPath });
      const target = info.target || lnkPath;
      let iconPath = '';

      // 尝试从目标 .exe 提取真实图标
      if (target.toLowerCase().endsWith('.exe')) {
        try {
          const dataUrl = await invoke<string>('extract_icon', { path: target });
          iconPath = dataUrl;
        } catch {
          iconPath = target; // 提取失败，DockItem 的 onerror 会显示字母回退
        }
      } else {
        iconPath = lnkPath;
      }

      addApp({
        name: info.name,
        icon_path: iconPath,
        command: target,
      });
    } catch (e) {
      console.error('parse_lnk failed:', e);
      const name = lnkPath.replace(/^.*[\\/]/, '').replace(/\.lnk$/i, '');
      addApp({
        name,
        icon_path: '',
        command: lnkPath,
      });
    }
  }

  let activeTab = $state('appearance');

  onMount(() => {
    loadConfig();

    const handler = () => {
      saveConfigNow();
    };
    window.addEventListener('beforeunload', handler);
    return () => {
      window.removeEventListener('beforeunload', handler);
    };
  });
</script>

<div class="settings-container" style:--anim-speed="{animSpeed}ms">
  <header class="settings-header">
    <h1>Tiny Dock 设置</h1>
  </header>

  <nav class="tabs">
    <button
      class="tab"
      class:active={activeTab === 'appearance'}
      onclick={() => (activeTab = 'appearance')}
    >
      外观
    </button>
    <button
      class="tab"
      class:active={activeTab === 'behavior'}
      onclick={() => (activeTab = 'behavior')}
    >
      行为
    </button>
    <button
      class="tab"
      class:active={activeTab === 'apps'}
      onclick={() => (activeTab = 'apps')}
    >
      应用
    </button>
    <button
      class="tab"
      class:active={activeTab === 'theme'}
      onclick={() => (activeTab = 'theme')}
    >
      主题
    </button>
  </nav>

  <div class="tab-content">
    {#if activeTab === 'appearance'}
      <section class="section">
        <h2>图标大小</h2>
        <div class="control-row">
          <input
            type="range"
            min="24"
            max="96"
            step="4"
            value={config.icon_size}
            oninput={(e) => updateConfig({ icon_size: parseInt(e.currentTarget.value) })}
          />
          <span class="value">{config.icon_size}px</span>
        </div>
      </section>

      <section class="section">
        <h2>放大级别</h2>
        <div class="control-row">
          <input
            type="range"
            min="1"
            max="3"
            step="0.1"
            value={config.magnification}
            oninput={(e) => updateConfig({ magnification: parseFloat(e.currentTarget.value) })}
          />
          <span class="value">{config.magnification.toFixed(1)}x</span>
        </div>
      </section>

      <section class="section">
        <h2>放大范围</h2>
        <div class="control-row">
          <input
            type="range"
            min="1"
            max="5"
            step="1"
            value={config.magnification_range}
            oninput={(e) => updateConfig({ magnification_range: parseInt(e.currentTarget.value) })}
          />
          <span class="value">{config.magnification_range} 个图标</span>
        </div>
      </section>

      <section class="section">
        <h2>背景透明度</h2>
        <div class="control-row">
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={config.opacity}
            oninput={(e) => updateConfig({ opacity: parseFloat(e.currentTarget.value) })}
          />
          <span class="value">{Math.round(config.opacity * 100)}%</span>
        </div>
      </section>

      <section class="section">
        <h2>模糊度</h2>
        <div class="control-row">
          <input
            type="range"
            min="0"
            max="60"
            step="5"
            value={config.blur}
            oninput={(e) => updateConfig({ blur: parseInt(e.currentTarget.value) })}
          />
          <span class="value">{config.blur}px</span>
        </div>
      </section>
    {:else if activeTab === 'behavior'}
      <section class="section">
        <h2>自动隐藏</h2>
        <div class="control-row">
          <label class="switch-label">
            <input
              type="checkbox"
              checked={config.auto_hide}
              onchange={(e) => updateConfig({ auto_hide: e.currentTarget.checked })}
            />
            <span>启用自动隐藏</span>
          </label>
        </div>
      </section>

      <section class="section">
        <h2>全屏时隐藏</h2>
        <div class="control-row">
          <label class="switch-label">
            <input
              type="checkbox"
              checked={config.hide_on_fullscreen}
              onchange={(e) => updateConfig({ hide_on_fullscreen: e.currentTarget.checked })}
            />
            <span>全屏应用存在时隐藏 Dock</span>
          </label>
        </div>
      </section>

      <section class="section">
        <h2>显示运行指示器</h2>
        <div class="control-row">
          <label class="switch-label">
            <input
              type="checkbox"
              checked={config.show_running_indicators}
              onchange={(e) => updateConfig({ show_running_indicators: e.currentTarget.checked })}
            />
            <span>在运行中的应用图标下显示小圆条</span>
          </label>
        </div>
      </section>

      <section class="section">
        <h2>动画速度</h2>
        <div class="control-row">
          <input
            type="range"
            min="50"
            max="500"
            step="25"
            value={config.animation_speed}
            oninput={(e) => updateConfig({ animation_speed: parseInt(e.currentTarget.value) })}
          />
          <span class="value">{config.animation_speed}ms</span>
        </div>
      </section>

      <section class="section">
        <h2>开机自启</h2>
        <div class="control-row">
          <label class="switch-label">
            <input
              type="checkbox"
              checked={config.auto_start}
              onchange={(e) => updateConfig({ auto_start: e.currentTarget.checked })}
            />
            <span>开机自动启动 Tiny Dock</span>
          </label>
        </div>
      </section>

      <section class="section">
        <h2>屏幕边缘距离</h2>
        <div class="control-row">
          <input
            type="range"
            min="0"
            max="60"
            step="2"
            value={config.dock_margin}
            oninput={(e) => updateConfig({ dock_margin: parseInt(e.currentTarget.value) })}
          />
          <span class="value">{config.dock_margin}px</span>
        </div>
      </section>
    {:else if activeTab === 'apps'}
      <section class="section">
        <h2>快速添加</h2>
        <div class="add-app-form">
          <button class="btn-primary" onclick={handleQuickAddLnk}>从快捷方式添加 (.lnk)</button>
          <p class="hint">选择桌面或开始菜单中的 .lnk 快捷方式快速添加</p>
        </div>
      </section>

      <section class="section">
        <h2>手动添加</h2>
        <div class="add-app-form">
          <input
            type="text"
            placeholder="应用名称"
            bind:value={newAppName}
          />
          <div class="input-row">
            <input
              type="text"
              placeholder="图标路径"
              bind:value={newAppIcon}
            />
            <button class="btn-secondary" onclick={handlePickIcon}>选择</button>
          </div>
          <div class="input-row">
            <input
              type="text"
              placeholder="启动命令 / 可执行文件路径"
              bind:value={newAppCommand}
            />
            <button class="btn-secondary" onclick={handlePickCommand}>选择</button>
          </div>
          <button class="btn-primary" onclick={handleAddApp}>添加</button>
        </div>
      </section>

      <section class="section">
        <h2>Dock 应用列表</h2>
        <div class="app-list">
          {#each config.apps as app (app.id)}
            <div class="app-list-item">
              <img src={app.icon_path} alt={app.name} width="32" height="32" loading="lazy" onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = 'none'; }} />
              <div class="app-info">
                <span class="app-name">{app.name}</span>
                <span class="app-cmd">{app.command}</span>
              </div>
              <button class="btn-remove" onclick={() => removeApp(app.id)}>移除</button>
            </div>
          {:else}
            <p class="empty-text">暂未添加任何应用</p>
          {/each}
        </div>
      </section>
    {:else if activeTab === 'theme'}
      <section class="section">
        <h2>主题</h2>
        <div class="control-row">
          <select
            value={config.theme}
            onchange={(e) => updateConfig({ theme: e.currentTarget.value })}
          >
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </div>
      </section>

      <section class="section">
        <h2>关于</h2>
        <div class="about">
          <p><strong>Tiny Dock v0.1.0</strong></p>
          <p>一个使用 Tauri + Svelte 构建的 Windows 苹果风格 Dock</p>
        </div>
      </section>
    {/if}
  </div>

  <footer class="settings-footer">
    <div class="save-status" class:saving={saveStatus === 'saving'} class:saved={saveStatus === 'saved'}>
      {#if saveStatus === 'saving'}
        <span class="status-dot saving"></span>
        保存中...
      {:else if saveStatus === 'saved'}
        <span class="status-dot saved"></span>
        已保存
      {/if}
    </div>
    <button class="btn-save" onclick={saveConfigNow}>
       保存设置
     </button>
  </footer>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #1e1e1e;
    color: #e0e0e0;
    user-select: none;
  }

  :global(html) {
    background: #1e1e1e;
  }

  .settings-container {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .settings-header {
    padding: 20px 24px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
  }

  .settings-header h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: #fff;
  }

  .tabs {
    display: flex;
    gap: 0;
    padding: 0 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
    background: rgba(255, 255, 255, 0.02);
  }

  .tab {
    position: relative;
    padding: 10px 20px;
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: color var(--anim-speed, 150ms);
    outline: none;
  }

  .tab::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 0;
    height: 2px;
    border-radius: 1px;
    background: #0078d4;
    transition: width var(--anim-speed, 150ms);
  }

  .tab.active {
    color: #fff;
  }

  .tab.active::after {
    width: 60%;
  }

  .tab:hover {
    color: rgba(255, 255, 255, 0.85);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px 24px 24px;
  }

  .tab-content::-webkit-scrollbar {
    width: 6px;
  }

  .tab-content::-webkit-scrollbar-track {
    background: transparent;
  }

  .tab-content::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 3px;
  }

  .section {
    margin-bottom: 8px;
    padding: 16px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .section h2 {
    font-size: 11px;
    font-weight: 600;
    margin: 0 0 14px;
    color: rgba(255, 255, 255, 0.45);
    text-transform: uppercase;
    letter-spacing: 0.8px;
  }

  .control-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .control-row input[type='range'] {
    flex: 1;
    accent-color: #0078d4;
    height: 4px;
  }

  .control-row select {
    flex: 1;
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.08);
    color: #e0e0e0;
    border: 1px solid rgba(255, 255, 255, 0.12);
    font-size: 13px;
    outline: none;
    transition: border-color var(--anim-speed, 150ms);
  }

  .control-row select:focus {
    border-color: #0078d4;
  }

  .value {
    min-width: 56px;
    text-align: right;
    font-size: 12px;
    font-weight: 500;
    color: #fff;
    color: rgba(255, 255, 255, 0.7);
  }

  .switch-label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    font-size: 13px;
  }

  .switch-label input[type='checkbox'] {
    accent-color: #0078d4;
    width: 16px;
    height: 16px;
  }

  .add-app-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .add-app-form input {
    padding: 9px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    color: #e0e0e0;
    border: 1px solid rgba(255, 255, 255, 0.12);
    font-size: 13px;
    outline: none;
    transition: border-color var(--anim-speed, 150ms);
  }

  .add-app-form input:focus {
    border-color: #0078d4;
  }

  .add-app-form input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  .hint {
    margin: 8px 0 0;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.3);
    text-align: center;
  }

  .input-row {
    display: flex;
    gap: 8px;
  }

  .input-row input {
    flex: 1;
  }

  .btn-primary {
    padding: 9px 18px;
    border-radius: 6px;
    background: #0078d4;
    color: #fff;
    border: none;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--anim-speed, 150ms);
  }

  .btn-primary:hover {
    background: #1a8ae8;
  }

  .btn-primary:active {
    background: #005a9e;
  }

  .btn-secondary {
    padding: 9px 14px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.08);
    color: #d0d0d0;
    border: 1px solid rgba(255, 255, 255, 0.12);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--anim-speed, 150ms);
    white-space: nowrap;
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .btn-remove {
    padding: 5px 12px;
    border-radius: 5px;
    background: rgba(255, 59, 48, 0.1);
    color: #ff453a;
    border: 1px solid rgba(255, 59, 48, 0.2);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all var(--anim-speed, 150ms);
  }

  .btn-remove:hover {
    background: rgba(255, 59, 48, 0.25);
    border-color: rgba(255, 59, 48, 0.4);
  }

  .app-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .app-list-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    transition: background var(--anim-speed, 150ms);
  }

  .app-list-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .app-list-item img {
    border-radius: 7px;
    flex-shrink: 0;
  }

  .app-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .app-name {
    font-size: 13px;
    font-weight: 500;
    color: #e0e0e0;
  }

  .app-cmd {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.35);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-text {
    color: rgba(255, 255, 255, 0.3);
    font-size: 13px;
    text-align: center;
    padding: 24px 20px;
  }

  .about p {
    margin: 4px 0;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.45);
  }

  .about p strong {
    color: rgba(255, 255, 255, 0.7);
  }

  .settings-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 24px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
    flex-shrink: 0;
  }

  .save-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.4);
    transition: color var(--anim-speed, 150ms);
  }

  .save-status.saving {
    color: rgba(255, 255, 255, 0.6);
  }

  .save-status.saved {
    color: rgba(52, 199, 89, 0.9);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.3);
  }

  .status-dot.saving {
    background: #ffaa00;
    animation: pulse 1s infinite;
  }

  .status-dot.saved {
    background: #34c759;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .btn-save {
    padding: 8px 20px;
    border-radius: 6px;
    background: #0078d4;
    color: white;
    border: none;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all var(--anim-speed, 150ms);
  }

  .btn-save:hover {
    background: #1a8ae8;
  }

  .btn-save:active {
    background: #005a9e;
  }
</style>
