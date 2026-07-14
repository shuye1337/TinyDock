use std::collections::{HashMap, HashSet};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

struct ProcessSlot {
    child: Mutex<Option<Child>>,
    kill_signal: AtomicBool,
}

/// Tracks launched apps by owning their `Child` process handles.
pub struct ProcessTracker {
    running: Mutex<HashSet<String>>,
    children: Mutex<HashMap<String, Arc<ProcessSlot>>>,
}

impl ProcessTracker {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(HashSet::new()),
            children: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub fn launch_app(
    app_handle: AppHandle,
    state: State<'_, Arc<ProcessTracker>>,
    app_id: String,
    command: String,
) -> Result<(), String> {
    let child = Command::new(&command)
        .spawn()
        .map_err(|e| format!("Failed to launch app: {}", e))?;

    let slot = Arc::new(ProcessSlot {
        child: Mutex::new(Some(child)),
        kill_signal: AtomicBool::new(false),
    });

    {
        let mut children = state.children.lock().unwrap();
        children.insert(app_id.clone(), slot.clone());
        let mut running = state.running.lock().unwrap();
        running.insert(app_id.clone());
    }

    let _ = app_handle.emit("app-launched", app_id.clone());

    let tracker = state.inner().clone();
    let app_handle_for_thread = app_handle.clone();
    let app_id_for_thread = app_id.clone();

    thread::spawn(move || {
        loop {
            if slot.kill_signal.load(Ordering::Relaxed) {
                if let Ok(mut guard) = slot.child.lock() {
                    if let Some(ref mut child) = *guard {
                        let _ = child.kill();
                    }
                }
                break;
            }

            let exited = match slot.child.try_lock() {
                Ok(mut guard) => match guard.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(Some(_)) => true,
                        Ok(None) => false,
                        Err(_) => true,
                    },
                    None => true,
                },
                Err(_) => false,
            };
            if exited {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }

        if let Ok(mut guard) = slot.child.lock() {
            *guard = None;
        }

        {
            let mut children = tracker.children.lock().unwrap();
            children.remove(&app_id_for_thread);
            let mut running = tracker.running.lock().unwrap();
            running.remove(&app_id_for_thread);
        }

        let _ = app_handle_for_thread.emit("app-exited", app_id_for_thread);
    });

    Ok(())
}

#[tauri::command]
pub fn is_app_running(state: State<'_, Arc<ProcessTracker>>, app_id: String) -> bool {
    let running = state.running.lock().unwrap();
    running.contains(&app_id)
}

#[tauri::command]
pub fn mark_app_closed(state: State<'_, Arc<ProcessTracker>>, app_id: String) {
    let slot = {
        let children = state.children.lock().unwrap();
        children.get(&app_id).cloned()
    };
    if let Some(slot) = slot {
        slot.kill_signal.store(true, Ordering::Relaxed);
    }
    let mut running = state.running.lock().unwrap();
    running.remove(&app_id);
}

pub fn init_process_tracker() -> Arc<ProcessTracker> {
    Arc::new(ProcessTracker::new())
}