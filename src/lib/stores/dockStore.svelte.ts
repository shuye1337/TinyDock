import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';

export interface DockApp {
  id: string;
  name: string;
  icon_path: string;
  command: string;
}

export interface DockConfig {
  icon_size: number;
  magnification: number;
  magnification_range: number;
  auto_hide: boolean;
  opacity: number;
  blur: number;
  theme: string;
  animation_speed: number;
  show_running_indicators: boolean;
  auto_start: boolean;
  hide_on_fullscreen: boolean;
  dock_margin: number;
  apps: DockApp[];
}

const defaultConfig: DockConfig = {
  icon_size: 48,
  magnification: 1.6,
  magnification_range: 2,
  auto_hide: false,
  opacity: 0.45,
  blur: 30,
  theme: 'system',
  animation_speed: 150,
  show_running_indicators: true,
  auto_start: false,
  hide_on_fullscreen: true,
  dock_margin: 8,
  apps: [],
};

let config = $state<DockConfig>({ ...defaultConfig });
let runningApps = $state<Set<string>>(new Set());
let dockVisible = $state(true);
let saveTimeout: ReturnType<typeof setTimeout> | null = null;
let _saveStatus = $state<'idle' | 'saving' | 'saved'>('idle');

export function getSaveStatus() {
  return _saveStatus;
}

export function getConfig() {
  return config;
}

export function getRunningApps() {
  return runningApps;
}

export function isDockVisible() {
  return dockVisible;
}

export async function loadConfig() {
  try {
    const loaded = await invoke<DockConfig>('load_config');
    Object.assign(config, defaultConfig, loaded);
    scheduleDockResize();
  } catch (e) {
    console.error('Failed to load config:', e);
  }
}

async function saveToDisk() {
  _saveStatus = 'saving';
  try {
    await invoke('save_config', { config });
    _saveStatus = 'saved';
    // Notify other windows (e.g. dock window) with actual config data
    // Frontend emit() sends to ALL windows (each has its own module scope)
    emit('config-changed', config);
  } catch (e) {
    console.error('Failed to save config:', e);
    _saveStatus = 'idle';
  }
}

export function saveConfigDebounced() {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(() => {
    saveToDisk();
  }, 300);
}

export async function saveConfigNow() {
  if (saveTimeout) clearTimeout(saveTimeout);
  await saveToDisk();
}

let resizeTimeout: ReturnType<typeof setTimeout> | null = null;

function scheduleDockResize() {
  if (resizeTimeout) clearTimeout(resizeTimeout);
  resizeTimeout = setTimeout(() => {
    invoke('update_dock_window', {
      iconSize: config.icon_size,
      magnification: config.magnification,
      appCount: config.apps.length,
      margin: config.dock_margin,
    }).catch(() => {});
  }, 100);
}

export function updateConfig(partial: Partial<DockConfig>) {
  Object.assign(config, partial);
  if ('icon_size' in partial || 'magnification' in partial || 'dock_margin' in partial) {
    scheduleDockResize();
  }
  saveConfigDebounced();
}

export function addApp(app: Omit<DockApp, 'id'>) {
  const newApp: DockApp = {
    id: crypto.randomUUID(),
    ...app,
  };
  config.apps = [...config.apps, newApp];
  scheduleDockResize();
  saveConfigDebounced();
}

export function removeApp(id: string) {
  config.apps = config.apps.filter((a) => a.id !== id);
  scheduleDockResize();
  saveConfigDebounced();
}

export function reorderApps(fromIndex: number, toIndex: number) {
  const apps = [...config.apps];
  const [moved] = apps.splice(fromIndex, 1);
  apps.splice(toIndex, 0, moved);
  config.apps = apps;
  saveConfigDebounced();
}

export async function launchApp(app: DockApp) {
  try {
    await invoke('launch_app', { appId: app.id, command: app.command });
    // Don't add locally — the backend-emitted `app-launched` event is the
    // single source of truth for runningApps and updates the Set reactively.
  } catch (e) {
    console.error('Failed to launch app:', e);
  }
}

export function markAppClosed(id: string) {
  const next = new Set(runningApps);
  next.delete(id);
  runningApps = next;
}

export function setDockVisible(visible: boolean) {
  dockVisible = visible;
}

export async function setupEventListeners(): Promise<() => Promise<void>> {
  const unlistens: Array<() => void> = [];

  unlistens.push(
    await listen<string>('app-launched', (event) => {
      runningApps = new Set(runningApps).add(event.payload as unknown as string);
    }),
  );

  unlistens.push(
    await listen('app-exited', (event) => {
      const next = new Set(runningApps);
      next.delete(event.payload as unknown as string);
      runningApps = next;
    }),
  );

  unlistens.push(
    await listen('fullscreen-detected', () => {
      if (config.hide_on_fullscreen) {
        dockVisible = false;
      }
    }),
  );

  unlistens.push(
    await listen('fullscreen-cleared', () => {
      dockVisible = true;
    }),
  );

  // Listen for config changes from other windows (e.g. settings window)
  unlistens.push(
    await listen<DockConfig>('config-changed', (event) => {
      Object.assign(config, event.payload as unknown as DockConfig);
      scheduleDockResize();
    }),
  );

  return async () => {
    for (const u of unlistens) u();
  };
}
