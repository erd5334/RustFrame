#[cfg(not(target_os = "windows"))]
use tauri::State;

use crate::monitors::MonitorInfo;
use crate::{display_info, platform, platform_info};
#[cfg(not(target_os = "windows"))]
use crate::AppState;

// Windows implementation
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    platform::services::get_monitors()
}

// Non-Windows implementation using Tauri API
#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub async fn get_monitors(
    window: tauri::Window,
    state: State<'_, AppState>,
) -> Result<Vec<MonitorInfo>, String> {
    platform::services::get_monitors(window, state.inner())
}

// Windows implementation
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn get_screen_dimensions() -> Result<(u32, u32), String> {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN) as u32;
        let h = GetSystemMetrics(SM_CYSCREEN) as u32;
        Ok((w, h))
    }
}

// Non-Windows stub
#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub async fn get_screen_dimensions() -> Result<(u32, u32), String> {
    // TODO: Implement for macOS and Linux using proper APIs
    Ok((1920, 1080))
}

// Windows implementation
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn get_monitor_refresh_rate() -> Result<u32, String> {
    use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    unsafe {
        let mut devmode: DEVMODEW = std::mem::zeroed();
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        if EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut devmode).as_bool() {
            return Ok(devmode.dmDisplayFrequency.max(30));
        }
        Ok(60)
    }
}

// Non-Windows stub
#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub async fn get_monitor_refresh_rate() -> Result<u32, String> {
    // TODO: Implement for macOS and Linux
    Ok(60) // Default to 60Hz
}

// ============================================================================
// Platform Info
// ============================================================================

#[tauri::command]
pub fn is_dev_mode() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
pub fn get_platform_info() -> platform_info::PlatformInfo {
    platform_info::PlatformInfo::detect()
}

/// Get the display scale factor for DPI-aware UI sizing
/// Returns the scale factor (1.0 for standard displays, 2.0 for Retina, etc.)
#[tauri::command]
pub fn get_display_scale_factor() -> f64 {
    let display_info = display_info::get();
    display_info.scale_factor
}

/// Get the application version from Cargo.toml
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get recommended window size based on display scale
/// Returns (width, height) in logical pixels that accounts for DPI/scaling
#[tauri::command]
pub fn get_recommended_window_size(window: tauri::Window) -> (u32, u32) {
    let (work_width, work_height) = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|monitor| {
            let size = monitor.work_area().size;
            let scale_factor = monitor.scale_factor();
            (
                size.width as f64 / scale_factor,
                size.height as f64 / scale_factor,
            )
        })
        .unwrap_or_else(|| {
            let display_info = display_info::get();
            if display_info.width_points > 0.0 && display_info.height_points > 0.0 {
                (display_info.width_points, display_info.height_points)
            } else {
                (0.0, 0.0)
            }
        });

    // Use the monitor work area to avoid oversized defaults on smaller screens.
    let work_height = if work_height > 0.0 { work_height } else { 900.0 };
    let work_width = if work_width > 0.0 { work_width } else { 1200.0 };

    let base_height = work_height * 0.6;
    let max_height = (work_height * 0.85).min(820.0);
    let min_height = (work_height * 0.5).min(640.0).min(max_height);
    let target_height = base_height.clamp(min_height, max_height);

    let base_width = target_height * 1.05;
    let max_width = (work_width * 0.9).min(980.0);
    let min_width = (work_width * 0.5).min(760.0).min(max_width);
    let target_width = base_width.clamp(min_width, max_width);

    (target_width.round() as u32, target_height.round() as u32)
}
