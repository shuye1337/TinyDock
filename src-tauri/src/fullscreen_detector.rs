use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
pub fn start_fullscreen_detector(app_handle: AppHandle) {
    let app_handle = Arc::new(app_handle);
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    thread::spawn(move || {
        let mut was_hidden = false;

        while r.load(Ordering::Relaxed) {
            // Continuously re-pin the dock to the bottom of the z-order.
            // This only runs while the window is visible; when hidden we
            // skip it to save the (small) per-tick cost.
            if !was_hidden {
                push_dock_to_bottom(&app_handle);
            }

            let is_fullscreen = check_fullscreen_windows();
            let should_hide = is_fullscreen && crate::config::cached_hide_on_fullscreen();

            if should_hide != was_hidden {
                was_hidden = should_hide;
                if let Some(window) = app_handle.get_webview_window("main") {
                    if should_hide {
                        // Hide the OS window entirely: stops the webview
                        // rendering loop and frees the on-screen presence,
                        // saving real resources while a fullscreen app runs.
                        let _ = window.hide();
                        let _ = app_handle.emit("fullscreen-detected", true);
                    } else {
                        let _ = window.show();
                        // Re-pin z-order immediately after showing so it
                        // doesn't briefly appear on top of other windows.
                        push_dock_to_bottom(&app_handle);
                        let _ = app_handle.emit("fullscreen-cleared", false);
                    }
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });
}

#[cfg(windows)]
fn push_dock_to_bottom(app_handle: &AppHandle) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_BOTTOM, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };

    let Some(window) = app_handle.get_webview_window("main") else {
        return;
    };
    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else { return };

    unsafe {
        let hwnd = HWND(win32.hwnd.get() as *mut std::ffi::c_void);
        // Re-apply WS_EX_NOACTIVATE in case some Tauri/edge path reset it.
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if ex & (WS_EX_NOACTIVATE.0 as isize) == 0 {
            let _ = SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                ex | WS_EX_NOACTIVATE.0 as isize,
            );
        }
        // Push to the very bottom of the z-order.
        let _ = SetWindowPos(
            hwnd,
            HWND_BOTTOM,
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(windows)]
fn check_fullscreen_windows() -> bool {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect, GetClassNameW};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND(std::ptr::null_mut()) {
            return false; // No foreground window → desktop → don't hide dock
        }

        // Exclude desktop/background windows — dock should be visible on desktop
        let mut class_buf = [0u16; 128];
        let len = GetClassNameW(hwnd, &mut class_buf);
        if len > 0 {
            let upper = class_buf[..len as usize]
                .iter()
                .map(|&c| if c >= b'a' as u16 && c <= b'z' as u16 { c - 32 } else { c })
                .collect::<Vec<_>>();
            let class_str = String::from_utf16_lossy(&upper);
            // Progman = desktop icon layer, WorkerW = desktop wallpaper behind
            if class_str == "PROGMAN" || class_str == "WORKERW" || class_str.starts_with("SYSLISTVIEW") {
                return false;
            }
        }

        let mut window_rect = RECT::default();
        if GetWindowRect(hwnd, &mut window_rect).is_err() {
            return false;
        }

        let hmonitor = MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
        let mut monitor_info: MONITORINFO = std::mem::zeroed();
        monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

        if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
            let monitor_rect = monitor_info.rcMonitor;

            window_rect.left <= monitor_rect.left
                && window_rect.top <= monitor_rect.top
                && window_rect.right >= monitor_rect.right
                && window_rect.bottom >= monitor_rect.bottom
        } else {
            false
        }
    }
}

#[cfg(not(windows))]
pub fn start_fullscreen_detector(_app_handle: AppHandle) {}
