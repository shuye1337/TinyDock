use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

static CACHED_HIDE_ON_FULLSCREEN: AtomicBool = AtomicBool::new(true);

pub fn cached_hide_on_fullscreen() -> bool {
    CACHED_HIDE_ON_FULLSCREEN.load(Ordering::Relaxed)
}

pub fn set_cached_hide_on_fullscreen(val: bool) {
    CACHED_HIDE_ON_FULLSCREEN.store(val, Ordering::Relaxed);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockApp {
    pub id: String,
    pub name: String,
    pub icon_path: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockConfig {
    pub icon_size: u32,
    pub magnification: f64,
    pub magnification_range: u32,
    pub auto_hide: bool,
    pub opacity: f64,
    pub blur: u32,
    pub theme: String,
    pub animation_speed: u32,
    pub show_running_indicators: bool,
    pub auto_start: bool,
    pub hide_on_fullscreen: bool,
    pub dock_margin: u32,
    pub apps: Vec<DockApp>,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            icon_size: 48,
            magnification: 1.6,
            magnification_range: 2,
            auto_hide: false,
            opacity: 0.45,
            blur: 30,
            theme: "system".to_string(),
            animation_speed: 150,
            show_running_indicators: true,
            auto_start: false,
            hide_on_fullscreen: true,
            dock_margin: 8,
            apps: vec![],
        }
    }
}

fn config_path(app_handle: &AppHandle) -> Option<PathBuf> {
    let dir = app_handle.path().app_data_dir().ok()?;
    fs::create_dir_all(&dir).ok();
    Some(dir.join("config.json"))
}

/// Read config from disk without being a Tauri command (for use in setup).
/// Falls back to `Default` if the data dir is unavailable or the file is
/// missing / unreadable — never panics the setup chain.
pub fn read_config(app_handle: &AppHandle) -> DockConfig {
    let Some(path) = config_path(app_handle) else {
        return DockConfig::default();
    };
    if !path.exists() {
        return DockConfig::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .inspect(|cfg: &DockConfig| set_cached_hide_on_fullscreen(cfg.hide_on_fullscreen))
        .unwrap_or_default()
}

#[tauri::command]
pub fn load_config(app_handle: AppHandle) -> Result<DockConfig, String> {
    let path = config_path(&app_handle)
        .ok_or_else(|| "Could not resolve app data directory".to_string())?;
    if !path.exists() {
        return Ok(DockConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config: DockConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn save_config(app_handle: AppHandle, config: DockConfig) -> Result<(), String> {
    let path = config_path(&app_handle)
        .ok_or_else(|| "Could not resolve app data directory".to_string())?;
    let content = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    set_cached_hide_on_fullscreen(config.hide_on_fullscreen);
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}
