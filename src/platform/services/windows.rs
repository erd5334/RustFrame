use crate::destination_window::{DestinationWindow, DestinationWindowConfig};
use crate::monitors::MonitorInfo;
use crate::settings::{CaptureMethod, PreviewMode, Settings};
use rustframe_capture::capture::{CaptureEngine, CaptureFrame, CaptureRect, GpuTextureHandle};
use rustframe_capture::window_filter::WindowIdentifier;
use std::sync::{Arc, Mutex};
use rustframe_capture::{config, display_info};

pub fn create_capture_engine_for_settings(
    settings: &Settings,
) -> Result<Box<dyn CaptureEngine>, String> {
    use rustframe_capture::capture::windows::{
        WindowsCaptureEngine, WindowsGdiCopyCaptureEngine,
    };

    match settings.capture_method {
        CaptureMethod::Wgc => WindowsCaptureEngine::new()
            .map(|e| Box::new(e) as Box<dyn CaptureEngine>)
            .map_err(|e| e.to_string()),
        CaptureMethod::GdiCopy => WindowsGdiCopyCaptureEngine::new()
            .map(|e| Box::new(e) as Box<dyn CaptureEngine>)
            .map_err(|e| e.to_string()),
    }
}

pub fn create_destination_window_for_settings(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    settings: &Settings,
) -> Result<DestinationWindow, String> {
    match settings.preview_mode {
        PreviewMode::WinApiGdi => {
            let config = DestinationWindowConfig {
                alpha: settings.winapi_destination_alpha,
                topmost: settings.winapi_destination_topmost,
                click_through: settings.winapi_destination_click_through,
                toolwindow: settings.winapi_destination_toolwindow,
                layered: settings.winapi_destination_layered,
                appwindow: settings.winapi_destination_appwindow,
                noactivate: settings.winapi_destination_noactivate,
                overlapped: settings.winapi_destination_overlapped,
            };
            let dest_window = DestinationWindow::new(x, y, width, height, config)
                .ok_or("Failed to create destination window")?;

            log::info!("Destination window created successfully");

            // Optional: after a delay, hide the preview window from taskbar/Alt-Tab.
            // This is useful for Discord: keep it "app-like" long enough to select in the picker,
            // then hide it.
            if let Some(delay_ms) = settings.winapi_destination_hide_taskbar_after_ms {
                let hwnd_value = dest_window.hwnd_value();
                if hwnd_value != 0 {
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms as u64));
                        unsafe {
                            use windows::Win32::Foundation::HWND;
                            use windows::Win32::UI::WindowsAndMessaging::{
                                GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
                                SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
                                WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
                            };

                            let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
                            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                            if ex != 0 {
                                // Hide from taskbar/Alt-Tab by marking as TOOLWINDOW.
                                // Also remove APPWINDOW if present.
                                let new_ex = (ex | (WS_EX_TOOLWINDOW.0 as isize))
                                    & !(WS_EX_APPWINDOW.0 as isize);
                                let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex);
                                let _ = SetWindowPos(
                                    hwnd,
                                    None,
                                    0,
                                    0,
                                    0,
                                    0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                                );
                            }
                        }
                    });
                }
            }

            Ok(dest_window)
        }
        PreviewMode::TauriCanvas => {
            log::warn!("Tauri Canvas mode not yet implemented");
            Err("Tauri Canvas mode not yet implemented".to_string())
        }
    }
}

pub fn build_capture_exclusion_list(_settings: &Settings) -> Vec<WindowIdentifier> {
    vec![WindowIdentifier::preview_window()]
}

pub fn configure_preview_window_for_capture(x: i32, y: i32, width: u32, height: u32) {
    use rustframe_capture::capture::windows::set_preview_bounds_and_check_overlap;

    // Get destination window bounds
    if let Ok(mut dest_lock) = crate::app_state::DESTINATION_WINDOW.lock() {
        if let Some(ref mut dest_window) = *dest_lock {
            // DO NOT exclude preview from screen capture - this causes black screen in Meet/Discord!
            // We rely on z-order (putting it at bottom) or window filtering to avoid infinite mirror.
            // dest_window.exclude_from_capture();
            log::info!("✅ Preview window configured for capture visibility");

            // RegionToShare approach: Position preview at border location
            // It will be below separation layer in z-order
            dest_window.set_pos(x, y);
            dest_window.resize(width, height);
            log::info!(
                "✅ Preview positioned at border location: ({}, {}) {}x{}",
                x,
                y,
                width,
                height
            );
            dest_window.disable_masking();

            // Setup z-order: Preview below separation layer
            // Get separation layer HWND
            if let Ok(sep_lock) = crate::app_state::SEPARATION_LAYER.try_lock() {
                if let Some(ref sep) = *sep_lock {
                    use windows::Win32::Foundation::HWND;
                    use windows::Win32::UI::WindowsAndMessaging::{
                        SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
                    };

                    let sep_hwnd = HWND(sep.hwnd_value() as *mut _);
                    let preview_hwnd = HWND(dest_window.hwnd_value() as *mut _);

                    // Position preview below separation layer
                    let _ = unsafe {
                        SetWindowPos(
                            preview_hwnd,
                            Some(sep_hwnd),
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        )
                    };
                    log::info!("✅ Z-order established: Border → Separation Layer → Preview");
                }
            }

            if let Some((px, py, pw, ph)) = dest_window.get_rect() {
                tracing::info!(
                    preview_x = px,
                    preview_y = py,
                    preview_width = pw,
                    preview_height = ph,
                    capture_x = x,
                    capture_y = y,
                    capture_width = width,
                    capture_height = height,
                    "Preview positioned at border for RegionToShare approach"
                );

                // Check overlap for cursor filtering
                set_preview_bounds_and_check_overlap(
                    px,
                    py,
                    pw,
                    ph, // preview bounds
                    x,
                    y,
                    width as i32,
                    height as i32, // capture bounds
                );
            }
        }
    }
}

pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    use std::mem;
    use windows::core::BOOL;
    use windows::Win32::Foundation::{LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW, DEVMODEW,
        ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    let mut monitors = Vec::new();

    unsafe extern "system" fn enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);

        let mut info: MONITORINFOEXW = mem::zeroed();
        info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmonitor, &mut info.monitorInfo as *mut _ as *mut _).as_bool() {
            let rect = info.monitorInfo.rcMonitor;
            let name = String::from_utf16_lossy(&info.szDevice);
            let device_name = windows::core::PCWSTR::from_raw(info.szDevice.as_ptr());

            // Get DPI
            let mut dpi_x = 96;
            let mut dpi_y = 96;
            let _ = GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
            let scale_factor = dpi_x as f64 / 96.0;

            // Get refresh rate from display settings
            let mut devmode: DEVMODEW = mem::zeroed();
            devmode.dmSize = mem::size_of::<DEVMODEW>() as u16;
            let refresh_rate =
                if EnumDisplaySettingsW(device_name, ENUM_CURRENT_SETTINGS, &mut devmode).as_bool()
                {
                    devmode.dmDisplayFrequency
                } else {
                    60 // Default to 60Hz
                };

            monitors.push(MonitorInfo {
                id: monitors.len(),
                name: name.trim_end_matches('\0').to_string(),
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left) as u32,
                height: (rect.bottom - rect.top) as u32,
                scale_factor,
                is_primary: info.monitorInfo.dwFlags == 1,
                refresh_rate,
            });
        }

        BOOL::from(true)
    }

    unsafe {
        let monitors_ptr = &mut monitors as *mut Vec<MonitorInfo> as isize;
        let _ = EnumDisplayMonitors(
            Some(HDC::default()),
            None,
            Some(enum_proc),
            LPARAM(monitors_ptr),
        );
    }

    Ok(monitors)
}

pub fn sync_preview_window_to_border(x: i32, y: i32, width: i32, height: i32) {
    if let Ok(mut dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(ref mut dest_window) = *dest_lock {
            dest_window.set_pos(x, y);
            dest_window.resize(width as u32, height as u32);
        }
    }

    if let Ok(sep_lock) = crate::app_state::SEPARATION_LAYER.try_lock() {
        if let Some(ref sep) = *sep_lock {
            sep.update_position(x, y, width, height);
        }
    }
}

pub fn handle_border_interaction_platform_updates(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    inner_width: i32,
    inner_height: i32,
) {
    if let Ok(sep_lock) = crate::app_state::SEPARATION_LAYER.try_lock() {
        if let Some(ref sep) = *sep_lock {
            sep.update_position(x, y, width, height);
        }
    }

    if let Ok(mut dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(ref mut dest) = *dest_lock {
            dest.resize(inner_width as u32, inner_height as u32);
            dest.set_pos(x, y);
            log::info!(
                "✅ Preview resized and positioned at border: {}x{} at ({}, {})",
                inner_width,
                inner_height,
                x,
                y
            );
        }
    }

    // Update cursor filtering based on new border position
    use rustframe_capture::capture::windows::set_preview_bounds_and_check_overlap;
    if let Ok(dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(ref dest_window) = *dest_lock {
            if let Some((px, py, pw, ph)) = dest_window.get_rect() {
                set_preview_bounds_and_check_overlap(
                    px,
                    py,
                    pw,
                    ph, // preview bounds
                    x,
                    y,
                    width,
                    height, // capture (border) bounds
                );
            }
        }
    }
}

pub fn update_capture_engine_after_border_interaction(
    engine: &Arc<Mutex<Option<Box<dyn CaptureEngine>>>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_w: i32,
) {
    let border_offset = border_w;
    let inner_width = (width - border_offset * 2).max(1);
    let inner_height = (height - border_offset * 2).max(1);

    let center_x = x + width / 2;
    let center_y = y + height / 2;

    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    let current_monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: center_x,
                y: center_y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };

    if current_monitor.is_invalid() {
        return;
    }

    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if !unsafe { GetMonitorInfoW(current_monitor, &mut monitor_info) }.as_bool() {
        return;
    }

    let monitor_left = monitor_info.rcMonitor.left;
    let monitor_top = monitor_info.rcMonitor.top;

    // Get monitor DPI for scaling calculations
    let mut dpi_x: u32 = 96;
    let mut dpi_y: u32 = 96;
    if unsafe {
        GetDpiForMonitor(
            current_monitor,
            MDT_EFFECTIVE_DPI,
            &mut dpi_x,
            &mut dpi_y,
        )
    }
    .is_ok()
    {
        let scale_factor = dpi_x as f32 / 96.0;
        log::info!(
            "🖥️  Monitor DPI: {}x{}, Scale factor: {:.2}x",
            dpi_x,
            dpi_y,
            scale_factor
        );
    }

    let mut engine_lock = match engine.try_lock() {
        Ok(e) => e,
        Err(e) => {
            log::error!(
                "❌ Failed to lock capture engine during border move: {:?}",
                e
            );
            return;
        }
    };

    let needs_restart = if let Some(ref eng) = *engine_lock {
        if let Some(wce) =
            eng.as_any()
                .downcast_ref::<rustframe_capture::capture::WindowsCaptureEngine>()
        {
            let current_origin = wce.get_monitor_origin();
            let changed = current_origin.0 != monitor_left || current_origin.1 != monitor_top;
            if changed {
                log::info!(
                    "🖥️  Monitor changed! Old origin: {:?}, New origin: ({}, {})",
                    current_origin,
                    monitor_left,
                    monitor_top
                );
            }
            changed
        } else {
            false
        }
    } else {
        false
    };

    if needs_restart {
        if let Some(ref mut eng) = *engine_lock {
            log::info!("Stopping capture to switch monitors...");
            eng.stop();
        }

        if let Some(ref mut eng) = *engine_lock {
            let new_region = CaptureRect {
                x: x + border_offset,
                y: y + border_offset,
                width: inner_width as u32,
                height: inner_height as u32,
            };

            log::info!("Restarting capture on new monitor with region: {:?}", new_region);
            if let Err(e) = eng.start(new_region, true, None) {
                log::error!("Failed to restart capture: {}", e);
            } else {
                log::info!("✅ Capture restarted on new monitor");
            }
        }
        return;
    }

    if let Some(ref mut eng) = *engine_lock {
        let new_region = CaptureRect {
            x: x + border_offset,
            y: y + border_offset,
            width: inner_width as u32,
            height: inner_height as u32,
        };
        if let Err(e) = eng.update_region(new_region) {
            log::error!("Failed to update capture region: {}", e);
        } else {
            log::info!(
                "✅ Capture region updated: x={}, y={}, w={}, h={}",
                new_region.x,
                new_region.y,
                new_region.width,
                new_region.height
            );
        }
    }
}

pub fn handle_border_live_move_platform_updates(x: i32, y: i32, width: i32, height: i32) {
    if let Ok(sep_lock) = crate::app_state::SEPARATION_LAYER.try_lock() {
        if let Some(ref sep) = *sep_lock {
            sep.update_position(x, y, width, height);
        }
    }

    if let Ok(dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(ref dest) = *dest_lock {
            dest.set_position_and_size(x, y, width, height);
        }
    }

    use rustframe_capture::capture::windows::set_preview_bounds_and_check_overlap;
    if let Ok(dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(ref dest_window) = *dest_lock {
            if let Some((px, py, pw, ph)) = dest_window.get_rect() {
                set_preview_bounds_and_check_overlap(
                    px,
                    py,
                    pw,
                    ph, // preview bounds
                    x,
                    y,
                    width,
                    height, // capture (border) bounds
                );
            }
        }
    }
}

pub fn update_capture_engine_during_live_move(
    engine: &Arc<Mutex<Option<Box<dyn CaptureEngine>>>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    _border_w: i32,
) {
    if let Ok(mut engine_lock) = engine.try_lock() {
        if let Some(ref mut eng) = *engine_lock {
            let scale_factor = crate::platform::input::get_screen_scale_factor();
            if let Err(e) = eng.set_scale_factor(scale_factor) {
                log::trace!("Failed to set scale factor: {}", e);
            }

            let new_region = CaptureRect {
                x,
                y,
                width: width as u32,
                height: height as u32,
            };

            if let Err(e) = eng.update_region(new_region) {
                log::trace!("Failed to update capture region during drag: {}", e);
            }
        }
    }
}

pub fn render_frame_to_destination(
    window: &mut DestinationWindow,
    mut frame: CaptureFrame,
    use_gpu: bool,
    capture_clicks_enabled: bool,
    click_color: [u8; 4],
    click_dissolve_ms: u64,
    click_radius: u32,
) {
    let mut click_shader_data: Option<(f32, f32, f32, f32, [f32; 4])> = None;

    if capture_clicks_enabled {
        let display = display_info::get();

        let offset_x_pixels = display.points_to_pixels(frame.offset_x as f64);
        let offset_y_pixels = display.points_to_pixels(frame.offset_y as f64);
        let width_pixels = display.points_to_pixels(frame.width as f64) as u32;
        let height_pixels = display.points_to_pixels(frame.height as f64) as u32;

        let clicks = crate::platform::input::get_recent_clicks(
            offset_x_pixels,
            offset_y_pixels,
            width_pixels,
            height_pixels,
            click_dissolve_ms,
        );

        if !clicks.is_empty() {
            if let Some(latest_click) = clicks.last() {
                let age_ms = latest_click.timestamp.elapsed().as_millis() as f32;
                let alpha_factor = 1.0 - (age_ms / click_dissolve_ms as f32).min(1.0);
                let scaled_radius = display.points_to_pixels(click_radius as f64);

                let frame_x = latest_click.x as f32 - offset_x_pixels as f32;
                let frame_y = latest_click.y as f32 - offset_y_pixels as f32;

                let [r, g, b, a] = config::colors::rgba_u8_to_f32(click_color);

                click_shader_data = Some((
                    frame_x,
                    frame_y,
                    scaled_radius as f32,
                    alpha_factor,
                    [r, g, b, a],
                ));
            }

            if !use_gpu {
                for click in clicks {
                    let frame_x = click.x - offset_x_pixels;
                    let frame_y = click.y - offset_y_pixels;
                    let age_ms = click.timestamp.elapsed().as_millis() as f32;
                    let alpha = 1.0 - (age_ms / click_dissolve_ms as f32).min(1.0);
                    let radius = display.points_to_pixels(click_radius as f64);

                    draw_click_highlight(
                        &mut frame.data,
                        frame.width as i32,
                        frame.height as i32,
                        frame_x,
                        frame_y,
                        click_color,
                        alpha,
                        radius,
                    );
                }
            }
        }
    }

    if use_gpu {
        if let Some(GpuTextureHandle::D3D11 {
            texture_ptr,
            crop_x,
            crop_y,
            crop_width,
            crop_height,
            ..
        }) = frame.gpu_texture
        {
            window.update_frame_from_texture(
                texture_ptr,
                crop_x,
                crop_y,
                crop_width,
                crop_height,
                click_shader_data,
            );
        } else {
            window.update_frame(frame.data, frame.width, frame.height);
        }
    } else {
        window.update_frame(frame.data, frame.width, frame.height);
    }
}

fn draw_click_highlight(
    data: &mut [u8],
    width: i32,
    height: i32,
    center_x: i32,
    center_y: i32,
    color: [u8; 4],
    alpha_factor: f32,
    radius: i32,
) {
    let inner_radius = (radius as f32 * 0.4).max(4.0) as i32;

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let px = center_x + dx;
            let py = center_y + dy;

            if px < 0 || px >= width || py < 0 || py >= height {
                continue;
            }

            let dist_sq = dx * dx + dy * dy;
            let dist = (dist_sq as f32).sqrt();
            if dist > radius as f32 {
                continue;
            }

            let ring_alpha = if dist <= inner_radius as f32 {
                1.0
            } else {
                1.0 - (dist - inner_radius as f32) / (radius - inner_radius) as f32
            };

            let final_alpha = config::colors::normalize_alpha(color[3]) * alpha_factor * ring_alpha;
            if final_alpha <= 0.0 {
                continue;
            }

            let idx = ((py * width + px) * 4) as usize;
            if idx + 3 >= data.len() {
                continue;
            }

            let inv_alpha = 1.0 - final_alpha;
            data[idx] = (color[2] as f32 * final_alpha + data[idx] as f32 * inv_alpha) as u8;
            data[idx + 1] =
                (color[1] as f32 * final_alpha + data[idx + 1] as f32 * inv_alpha) as u8;
            data[idx + 2] =
                (color[0] as f32 * final_alpha + data[idx + 2] as f32 * inv_alpha) as u8;
        }
    }
}

fn position_preview_below_separation() {
    if let (Ok(sep_lock), Ok(dest_lock)) = (
        crate::app_state::SEPARATION_LAYER.try_lock(),
        crate::app_state::DESTINATION_WINDOW.try_lock(),
    ) {
        if let (Some(sep), Some(dest_window)) = (sep_lock.as_ref(), dest_lock.as_ref()) {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            };

            let sep_hwnd = HWND(sep.hwnd_value() as *mut _);
            let preview_hwnd = HWND(dest_window.hwnd_value() as *mut _);

            if !sep_hwnd.0.is_null() && !preview_hwnd.0.is_null() {
                let _ = unsafe {
                    SetWindowPos(
                        preview_hwnd,
                        Some(sep_hwnd),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    )
                };
                log::info!("Z-order set: border -> separation -> preview");
            }
        }
    }
}

pub fn create_separation_layer_for_capture(x: i32, y: i32, width: u32, height: u32) {
    let separation_color = 0x4682B4; // Steel Blue like RegionToShare
    let (sep_x, sep_y, sep_width, sep_height) = (x, y, width as i32, height as i32);

    std::thread::spawn(move || {
        if let Some(separation) = crate::separation_layer::SeparationLayer::new(
            sep_x,
            sep_y,
            sep_width,
            sep_height,
            separation_color,
        ) {
            let border_alive = crate::app_state::HOLLOW_BORDER
                .lock()
                .ok()
                .and_then(|guard| guard.as_ref().map(|_| ()))
                .is_some();

            if !border_alive {
                log::warn!("Separation layer created after border closed; dropping");
                drop(separation);
                return;
            }

            *crate::app_state::SEPARATION_LAYER.lock().unwrap() = Some(separation);
            log::info!("Separation layer created (RegionToShare style)");
            position_preview_below_separation();
        } else {
            log::warn!("Failed to create separation layer");
        }
    });
}

pub fn clear_separation_layer_for_capture() {
    *crate::app_state::SEPARATION_LAYER.lock().unwrap() = None;
    log::info!("Separation layer cleared");
}

pub fn ensure_border_hwnd_cleared() -> Result<(), String> {
    use crate::hollow_border::is_hollow_hwnd_valid;
    let mut retries = 0;
    while is_hollow_hwnd_valid() && retries < 15 {
        tracing::debug!("Waiting for HOLLOW_HWND to be cleared (retry {})", retries);
        std::thread::sleep(std::time::Duration::from_millis(30));
        retries += 1;
    }
    if is_hollow_hwnd_valid() {
        tracing::error!(
            "HOLLOW_HWND still valid after {} retries - forcing cleanup",
            retries
        );
        return Err("Failed to clean up previous border window".to_string());
    }
    if retries > 0 {
        tracing::info!("HOLLOW_HWND cleared after {} retries", retries);
    }
    Ok(())
}

pub fn post_create_destination_window(_x: i32, _y: i32, _width: u32, _height: u32) {
}

pub fn clear_cursor_filtering_after_capture() {
    use rustframe_capture::capture::windows::clear_cursor_filtering;
    clear_cursor_filtering();
    tracing::debug!("Cursor filtering cleared");
}

