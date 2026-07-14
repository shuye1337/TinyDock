mod config;
mod launcher;
mod tray;
mod fullscreen_detector;

use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

type IconCache = Mutex<HashMap<String, String>>;

#[tauri::command]
fn notify_config_changed(app: tauri::AppHandle) {
    let _ = app.emit_to("main", "config-changed", ());
}

/// Calculate the optimal dock window geometry in logical pixels.
/// Returns (width, height, pos_x, pos_y) for centering at the bottom
/// of the work area with the given margin.
fn calc_dock_geometry(
    icon_size: f64,
    magnification: f64,
    app_count: u32,
    margin: f64,
    work_r: f64,
    work_b: f64,
) -> (f64, f64, f64, f64) {
    let item_width = icon_size + 8.0;
    let sep = if app_count > 1 { 17.0 } else { 0.0 };
    let layout_w = app_count as f64 * item_width + sep + 26.0;
    let mag_spread = icon_size * (magnification - 1.0).max(0.0);
    let dock_w = (layout_w + mag_spread + 20.0).max(200.0);
    let dock_h = (icon_size * magnification + 70.0).max(100.0);
    let pos_x = ((work_r - dock_w) / 2.0).max(0.0);
    let pos_y = (work_b - dock_h - margin).max(0.0);
    (dock_w, dock_h, pos_x, pos_y)
}

/// Returns the primary monitor's work area on Windows (excludes the taskbar).
/// Falls back to full-screen metrics if the call fails.
#[cfg(windows)]
fn work_area() -> (f64, f64, f64, f64) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA, SystemParametersInfoW,
    };
    unsafe {
        let mut rc = RECT::default();
        let result = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rc as *mut RECT as *mut std::ffi::c_void),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        if result.is_ok() {
            (
                rc.left as f64,
                rc.top as f64,
                rc.right as f64,
                rc.bottom as f64,
            )
        } else {
            // Fallback: full primary screen, treat bottom as screen bottom.
            let w = GetSystemMetrics(SM_CXSCREEN) as f64;
            let h = GetSystemMetrics(SM_CYSCREEN) as f64;
            (0.0, 0.0, w, h)
        }
    }
}

#[tauri::command]
fn update_dock_window(
    app: tauri::AppHandle,
    icon_size: u32,
    magnification: f64,
    app_count: u32,
    margin: u32,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let (_work_l, _work_t, work_r, work_b) = work_area();
        if let Some(window) = app.get_webview_window("main") {
            let scale = window.scale_factor().unwrap_or(1.0);
            let (w, h, x, y) = calc_dock_geometry(
                icon_size as f64,
                magnification,
                app_count,
                margin as f64,
                work_r / scale,
                work_b / scale,
            );
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)));
            let _ =
                window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (app, icon_size, magnification, app_count, margin);
    }
    Ok(())
}

#[tauri::command]
fn parse_lnk(path: String) -> Result<serde_json::Value, String> {
    #[cfg(windows)]
    {
        use std::fs;
        use std::path::Path;

        let path_obj = Path::new(&path);
        let name = path_obj
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Read .lnk file to extract target path
        let data = fs::read(&path).map_err(|e| format!("Failed to read .lnk: {}", e))?;

        // Parse .lnk shell link format
        // ShellLinkHeader starts at offset 0, LinkInfo starts at offset specified in header
        if data.len() < 0x4C {
            return Ok(serde_json::json!({ "name": name, "target": path }));
        }

        // Check header size
        let header_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if header_size != 0x4C {
            return Ok(serde_json::json!({ "name": name, "target": path }));
        }

        // Check LinkCLSID {00021401-0000-0000-c000-000000000046}
        let expected_clsid: [u8; 16] = [
            0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ];
        if data[4..20] != expected_clsid {
            return Ok(serde_json::json!({ "name": name, "target": path }));
        }

        // Read LinkFlags at offset 0x14
        let link_flags = u32::from_le_bytes([data[0x14], data[0x15], data[0x16], data[0x17]]);

        // HasLinkInfo flag = 0x02
        let has_link_info = link_flags & 0x02 != 0;

        let mut offset = 0x4C; // After ShellLinkHeader

        // Skip LinkTargetIDList if present (HasLinkIDList flag = 0x01)
        if link_flags & 0x01 != 0 {
            if offset + 2 > data.len() {
                return Ok(serde_json::json!({ "name": name, "target": path }));
            }
            let idlist_size = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2 + idlist_size;
        }

        // Parse LinkInfo if present
        let mut target_path = path.clone();

        if has_link_info && offset + 4 <= data.len() {
            let link_info_size = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
            let link_info_start = offset;
            let link_info_end = offset + link_info_size;

            if link_info_end <= data.len() {
                // LinkInfoHeaderSize at offset 0x04 within LinkInfo.
                // We need offsets up to LocalBasePathOffset (+0x10..+0x14 == +16..+20),
                // so require at least 20 bytes of header available.
                if link_info_start + 20 <= data.len() {
                    let _header_size = u32::from_le_bytes([
                        data[link_info_start + 4],
                        data[link_info_start + 5],
                        data[link_info_start + 6],
                        data[link_info_start + 7],
                    ]) as usize;

                    // VolumeIDAndLocalBasePath flag = 0x01
                    let link_info_flags = u32::from_le_bytes([
                        data[link_info_start + 8],
                        data[link_info_start + 9],
                        data[link_info_start + 10],
                        data[link_info_start + 11],
                    ]);

                    if link_info_flags & 0x01 != 0 {
                        // LocalBasePath offset within LinkInfo (after header)
                        let local_base_path_offset = u32::from_le_bytes([
                            data[link_info_start + 16],
                            data[link_info_start + 17],
                            data[link_info_start + 18],
                            data[link_info_start + 19],
                        ]) as usize;

                        let path_start = link_info_start + local_base_path_offset;
                        if path_start < data.len() {
                            // Read null-terminated ANSI string
                            let mut path_bytes = Vec::new();
                            for i in path_start..data.len() {
                                if data[i] == 0 {
                                    break;
                                }
                                path_bytes.push(data[i]);
                            }
                            if let Ok(p) = String::from_utf8(path_bytes) {
                                target_path = p;
                            }
                        }
                    }
                }
            }

            offset = link_info_end;
        }

        // Skip StringData (name, relative path, working dir, etc.)
        // Per MS-SHLLINK, the per-section flags are:
        //   HasName=0x04, HasRelativePath=0x08, HasWorkingDir=0x10,
        //   HasArguments=0x20, HasIconLocation=0x40.
        // HasUnicode (0x80) is a *global* flag that selects UTF-16 vs ANSI
        // encoding for every StringData entry — it is not a per-section flag.
        let string_flags = [0x04u32, 0x08, 0x10, 0x20, 0x40];
        let is_unicode = link_flags & 0x80 != 0;
        let char_bytes = if is_unicode { 2usize } else { 1usize };
        for &flag in &string_flags {
            if link_flags & flag != 0 && offset + 2 <= data.len() {
                let str_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                // CountCharacters (2) + string data (str_len * char_bytes)
                // + null terminator (char_bytes)
                offset += 2 + str_len * char_bytes + char_bytes;
            }
        }

        // ExtraData - look for EnvironmentProperty or FinalData block
        // Try to read ExpandedString in StringData (HasIconLocation = 0x40)
        if link_flags & 0x40 != 0 {
            // Already skipped above, but we can re-read from the .lnk
            // For simplicity, just use the target_path we found
        }

        Ok(serde_json::json!({ "name": name, "target": target_path }))
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Only supported on Windows".to_string())
    }
}

#[tauri::command]
async fn extract_icon(
    path: String,
    cache: tauri::State<'_, IconCache>,
) -> Result<String, String> {
    {
        let c = cache.lock().unwrap();
        if let Some(cached) = c.get(&path) {
            return Ok(cached.clone());
        }
    }

    let cache_key = path.clone();
    let result = tauri::async_runtime::spawn_blocking(move || extract_icon_blocking(path))
        .await
        .map_err(|e| format!("Task join error: {}", e))??;

    let mut c = cache.lock().unwrap();
    c.insert(cache_key, result.clone());
    Ok(result)
}

fn extract_icon_blocking(path: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};
            use windows::Win32::Graphics::Gdi::*;
            use std::io::Cursor;

            // Convert path to UTF-16
            let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

            // Try SHDefExtractIconW with desired size 256 (largest available)
            use windows::Win32::UI::Shell::SHDefExtractIconW;
            let mut hicon = HICON::default();
            let hr = SHDefExtractIconW(
                windows::core::PCWSTR(path_w.as_ptr()),
                0,
                0,
                Some(&mut hicon),
                Some(std::ptr::null_mut()),
                256,
            );

            if hr.is_err() || hicon.is_invalid() {
                // Fallback: ExtractIconExW with large icon (32x32)
                use windows::Win32::UI::Shell::ExtractIconExW;
                let mut hicon_fb = HICON::default();
                let count = ExtractIconExW(
                    windows::core::PCWSTR(path_w.as_ptr()),
                    0,
                    Some(&mut hicon_fb),
                    Some(std::ptr::null_mut()),
                    1,
                );
                if count == 0 {
                    return Err("No icon found in file".to_string());
                }
                hicon = hicon_fb;
            }

            // Get icon info to retrieve the color bitmap.
            // NOTE: GetIconInfo allocates hbmColor and hbmMask bitmaps that the
            // caller MUST delete via DeleteObject. DestroyIcon does NOT free them.
            let mut icon_info: windows::Win32::UI::WindowsAndMessaging::ICONINFO = std::mem::zeroed();
            if windows::Win32::UI::WindowsAndMessaging::GetIconInfo(hicon, &mut icon_info).is_err() {
                let _ = DestroyIcon(hicon);
                return Err("GetIconInfo failed".to_string());
            }

            let hbm_color = icon_info.hbmColor;
            let hbm_mask = icon_info.hbmMask;

            // Helper to release the GDI bitmaps that GetIconInfo allocated.
            // Always called before any early return / final cleanup.
            let free_bitmaps = || {
                if !hbm_color.is_invalid() {
                    let _ = DeleteObject(hbm_color);
                }
                if !hbm_mask.is_invalid() {
                    let _ = DeleteObject(hbm_mask);
                }
            };

            if hbm_color.is_invalid() {
                free_bitmaps();
                let _ = DestroyIcon(hicon);
                return Err("No color bitmap in icon".to_string());
            }

            // Get bitmap dimensions
            let mut bmp: BITMAP = std::mem::zeroed();
            let ret = GetObjectW(
                hbm_color,
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bmp as *mut BITMAP as *mut std::ffi::c_void),
            );
            if ret == 0 {
                free_bitmaps();
                let _ = DestroyIcon(hicon);
                return Err("GetObjectW failed".to_string());
            }

            let width = bmp.bmWidth as u32;
            let height = bmp.bmHeight as u32;
            if width == 0 || height == 0 {
                free_bitmaps();
                let _ = DestroyIcon(hicon);
                return Err("Icon has zero dimensions".to_string());
            }

            // Create a memory DC and select the bitmap into it
            let hdc = CreateCompatibleDC(None);
            if hdc.is_invalid() {
                free_bitmaps();
                let _ = DestroyIcon(hicon);
                return Err("CreateCompatibleDC failed".to_string());
            }

            let old_obj = SelectObject(hdc, HGDIOBJ(hbm_color.0));

            // Set up BITMAPINFO for 32-bit BGRA pixel retrieval (top-down)
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width as i32;
            bmi.bmiHeader.biHeight = -(height as i32); // top-down order
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let row_size = width * 4;
            let pixel_size = (row_size * height) as usize;
            let mut pixels = vec![0u8; pixel_size];

            let got = GetDIBits(
                hdc,
                HBITMAP(hbm_color.0),
                0,
                height,
                Some(pixels.as_mut_ptr() as *mut std::ffi::c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Cleanup GDI objects: restore DC selection, free DC, free bitmaps, free icon.
            let _ = SelectObject(hdc, old_obj);
            let _ = DeleteDC(hdc);
            free_bitmaps();
            let _ = DestroyIcon(hicon);

            if got == 0 {
                return Err("GetDIBits failed".to_string());
            }

            // GDI returns BGRA, need RGBA for image crate
            for pixel in pixels.chunks_exact_mut(4) {
                let b = pixel[0];
                pixel[0] = pixel[2]; // B→R
                pixel[2] = b;        // R→B
            }

            // Encode as PNG using image crate
            let img = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or("Failed to create image from raw pixels")?;

            let mut png_buf = Cursor::new(Vec::new());
            img.write_to(&mut png_buf, image::ImageFormat::Png)
                .map_err(|e| format!("PNG encoding failed: {}", e))?;

            // Base64 encode
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(png_buf.into_inner());
            Ok(format!("data:image/png;base64,{}", b64))
        }
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Icon extraction is only supported on Windows".to_string())
    }
}

#[tauri::command]
fn get_screen_size() -> (u32, u32) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYSCREEN) as u32;
            (width, height)
        }
    }
    #[cfg(not(windows))]
    {
        (1920, 1080)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(launcher::init_process_tracker())
        .manage(Mutex::new(HashMap::<String, String>::new()) as IconCache)
        .invoke_handler(tauri::generate_handler![
            config::load_config,
            config::save_config,
            launcher::launch_app,
            launcher::is_app_running,
            launcher::mark_app_closed,
            get_screen_size,
            parse_lnk,
            extract_icon,
            notify_config_changed,
            update_dock_window,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                // Position and show the main dock window
                if let Some(window) = app.get_webview_window("main") {
                    // Keep the dock off the taskbar but below all normal windows:
                    // we do NOT mark it always-on-top, so maximized / fullscreen
                    // windows properly cover it (issue #2 / #3), and we anchor
                    // it to the work-area bottom (just above the taskbar).
                    let _ = window.set_skip_taskbar(true);

                    // Get screen dimensions and position at bottom center of work area
                    #[cfg(windows)]
                    {
                        let (_work_l, _work_t, work_r, work_b) = work_area();
                        let scale = window.scale_factor().unwrap_or(1.0);
                        let saved_config = config::read_config(app.handle());
                        let (w, h, x, y) = calc_dock_geometry(
                            saved_config.icon_size as f64,
                            saved_config.magnification,
                            saved_config.apps.len() as u32,
                            saved_config.dock_margin as f64,
                            work_r / scale,
                            work_b / scale,
                        );
                        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)));
                        let _ =
                            window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
                    }

                    let _ = window.show();
                    let _ = window.set_decorations(false);

                    // Make the dock window non-activating so it never steals
                    // focus / topmost z-order from other windows. Without this,
                    // hovering/clicking on the dock pulls it above normal windows
                    // (issue #3 "普通窗口在 dock 之下") and full-screen apps fail
                    // to cover it (issue #2).
                    #[cfg(windows)]
                    {
                        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                        use windows::Win32::Foundation::HWND;
                        use windows::Win32::UI::WindowsAndMessaging::{
                            GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE,
                            SetWindowPos, HWND_BOTTOM, SWP_NOMOVE, SWP_NOSIZE,
                            SWP_NOACTIVATE, WS_EX_NOACTIVATE,
                        };
                        if let Ok(handle) = window.window_handle() {
                            if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                                unsafe {
                                    let hwnd = HWND(win32.hwnd.get() as *mut std::ffi::c_void);
                                    // Add WS_EX_NOACTIVATE: never steal focus.
                                    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                                    let _ = SetWindowLongPtrW(
                                        hwnd,
                                        GWL_EXSTYLE,
                                        ex | WS_EX_NOACTIVATE.0 as isize,
                                    );
                                    // Push to the very bottom of the z-order so
                                    // every normal / fullscreen window covers it.
                                    let _ = SetWindowPos(
                                        hwnd,
                                        HWND_BOTTOM,
                                        0, 0, 0, 0,
                                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                                    );
                                }
                            }
                        }
                    }
                }

                tray::setup_tray(app.handle())?;
                fullscreen_detector::start_fullscreen_detector(app.handle().clone());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
