use crate::destination_window::DestinationWindow;
use crate::hollow_border::{set_allow_screen_capture, HollowBorder};
use crate::monitors::MonitorInfo;
use crate::rec_indicator::RecIndicator;
use crate::platform;
use crate::settings::{should_allow_screen_capture, Settings};
use crate::AppState;
use rustframe_capture::capture::{CaptureEngine, CaptureFrame};
use rustframe_capture::window_filter::WindowIdentifier;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

#[cfg(not(target_os = "windows"))]
mod shared_tauri;

#[cfg(target_os = "windows")]
use windows as imp;
#[cfg(target_os = "macos")]
use macos as imp;
#[cfg(target_os = "linux")]
use linux as imp;

pub fn create_capture_engine_for_settings(
    settings: &Settings,
) -> Result<Box<dyn CaptureEngine>, String> {
    imp::create_capture_engine_for_settings(settings)
}

pub fn create_destination_window_for_settings(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    settings: &Settings,
) -> Result<DestinationWindow, String> {
    imp::create_destination_window_for_settings(x, y, width, height, settings)
}

pub fn build_capture_exclusion_list(settings: &Settings) -> Vec<WindowIdentifier> {
    imp::build_capture_exclusion_list(settings)
}

pub fn configure_preview_window_for_capture(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    imp::configure_preview_window_for_capture(x, y, width, height);
}

pub fn sync_preview_window_to_border(x: i32, y: i32, width: i32, height: i32) {
    imp::sync_preview_window_to_border(x, y, width, height);
}

pub fn handle_border_interaction_platform_updates(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    inner_width: i32,
    inner_height: i32,
) {
    imp::handle_border_interaction_platform_updates(
        x,
        y,
        width,
        height,
        inner_width,
        inner_height,
    );
}

pub fn update_capture_engine_after_border_interaction(
    engine: &Arc<Mutex<Option<Box<dyn CaptureEngine>>>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_w: i32,
) {
    imp::update_capture_engine_after_border_interaction(engine, x, y, width, height, border_w);
}

pub fn handle_border_live_move_platform_updates(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    imp::handle_border_live_move_platform_updates(x, y, width, height);
}

pub fn update_capture_engine_during_live_move(
    engine: &Arc<Mutex<Option<Box<dyn CaptureEngine>>>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_w: i32,
) {
    imp::update_capture_engine_during_live_move(engine, x, y, width, height, border_w);
}

pub fn render_frame_to_destination(
    window: &mut DestinationWindow,
    frame: CaptureFrame,
    use_gpu: bool,
    capture_clicks_enabled: bool,
    click_color: [u8; 4],
    click_dissolve_ms: u64,
    click_radius: u32,
) {
    imp::render_frame_to_destination(
        window,
        frame,
        use_gpu,
        capture_clicks_enabled,
        click_color,
        click_dissolve_ms,
        click_radius,
    );
}

pub fn render_frame_to_destination_if_available(
    is_interacting: bool,
    frame: CaptureFrame,
    use_gpu: bool,
    capture_clicks_enabled: bool,
    click_color: [u8; 4],
    click_dissolve_ms: u64,
    click_radius: u32,
) {
    let dest_lock = if is_interacting {
        crate::app_state::DESTINATION_WINDOW.lock().ok()
    } else {
        crate::app_state::DESTINATION_WINDOW.try_lock().ok()
    };
    if let Some(mut dest_lock) = dest_lock {
        if let Some(window) = dest_lock.as_mut() {
            render_frame_to_destination(
                window,
                frame,
                use_gpu,
                capture_clicks_enabled,
                click_color,
                click_dissolve_ms,
                click_radius,
            );
        }
    }
}

pub fn create_separation_layer_for_capture(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    imp::create_separation_layer_for_capture(x, y, width, height);
}

pub fn clear_separation_layer_for_capture() {
    imp::clear_separation_layer_for_capture();
}

pub fn ensure_border_hwnd_cleared() -> Result<(), String> {
    imp::ensure_border_hwnd_cleared()
}

pub fn post_create_destination_window(x: i32, y: i32, width: u32, height: u32) {
    imp::post_create_destination_window(x, y, width, height);
}

pub fn clear_cursor_filtering_after_capture() {
    imp::clear_cursor_filtering_after_capture();
}

pub fn cleanup_before_capture_start() -> Result<(), String> {
    tracing::info!("Cleaning up any existing borders before starting capture");

    crate::preview_border::clear_preview_border();

    let mut hollow = crate::app_state::HOLLOW_BORDER.lock().map_err(|e| e.to_string())?;
    if hollow.is_some() {
        tracing::info!("Closing existing capture border");
    }
    *hollow = None;
    drop(hollow);

    std::thread::sleep(Duration::from_millis(200));

    ensure_border_hwnd_cleared()?;
    Ok(())
}

pub fn clear_capture_windows() {
    tracing::debug!("Clearing HOLLOW_BORDER");
    *crate::app_state::HOLLOW_BORDER.lock().unwrap() = None;
    tracing::debug!("Clearing DESTINATION_WINDOW");
    *crate::app_state::DESTINATION_WINDOW.lock().unwrap() = None;
    tracing::debug!("Clearing REC_INDICATOR");
    *crate::app_state::REC_INDICATOR.lock().unwrap() = None;
}

pub fn get_capture_inner_rect() -> Option<(i32, i32, i32, i32)> {
    let border_guard = match crate::app_state::HOLLOW_BORDER.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    border_guard.as_ref().map(|b| b.get_inner_rect())
}

pub fn get_capture_rect() -> Option<(i32, i32, i32, i32)> {
    let border_guard = match crate::app_state::HOLLOW_BORDER.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    border_guard.as_ref().map(|b| b.get_rect())
}

pub fn clear_hollow_border_for_shutdown() {
    if let Ok(mut border) = crate::app_state::HOLLOW_BORDER.try_lock() {
        if border.is_some() {
            log::info!("Closing hollow border window");
            *border = None;
        }
    }
}

pub fn drop_capture_windows_in_background() {
    if let Ok(mut border) = crate::app_state::HOLLOW_BORDER.try_lock() {
        if let Some(b) = border.take() {
            std::thread::spawn(move || {
                drop(b);
            });
            tracing::debug!("Hollow border cleanup initiated");
        }
    }

    if let Ok(mut dest) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(d) = dest.take() {
            std::thread::spawn(move || {
                drop(d);
            });
            tracing::debug!("Destination window cleanup initiated");
        }
    }

    if let Ok(mut rec) = crate::app_state::REC_INDICATOR.try_lock() {
        if let Some(r) = rec.take() {
            std::thread::spawn(move || {
                drop(r);
            });
            tracing::debug!("REC indicator cleanup initiated");
        }
    }
}

pub fn cleanup_after_capture_stop() {
    tracing::debug!("Clearing HOLLOW_BORDER");
    *crate::app_state::HOLLOW_BORDER.lock().unwrap() = None;
    log::info!("Capture border cleared");

    tracing::debug!("Clearing PREVIEW_BORDER");
    crate::preview_border::clear_preview_border();
    log::info!("Preview border cleared");

    tracing::debug!("Clearing DESTINATION_WINDOW");
    *crate::app_state::DESTINATION_WINDOW.lock().unwrap() = None;

    tracing::debug!("Clearing separation layer");
    clear_separation_layer_for_capture();

    clear_cursor_filtering_after_capture();

    tracing::debug!("Clearing REC_INDICATOR");
    *crate::app_state::REC_INDICATOR.lock().unwrap() = None;
    log::info!("Capture windows cleaned up");
}

pub fn cleanup_after_capture_failed() {
    *crate::app_state::HOLLOW_BORDER.lock().unwrap() = None;
    crate::preview_border::clear_preview_border();
    *crate::app_state::DESTINATION_WINDOW.lock().unwrap() = None;
    *crate::app_state::REC_INDICATOR.lock().unwrap() = None;

    clear_separation_layer_for_capture();
}

pub fn register_border_callbacks(app: AppHandle, state: &AppState, border_w: i32) {
    use crate::hollow_border::{set_border_interaction_complete_callback, set_border_live_move_callback};

    let engine_for_cb = state.capture_engine.clone();
    let app_for_cb = app.clone();

    set_border_interaction_complete_callback(move |x, y, width, height| {
        log::info!(
            "🔄 Border interaction COMPLETE - Border window: x={}, y={}, w={}, h={}",
            x,
            y,
            width,
            height
        );

        let border_offset = border_w;
        let inner_width = (width - border_offset * 2).max(1);
        let inner_height = (height - border_offset * 2).max(1);
        log::info!(
            "🔄 Border offset: {}, Inner region: {}x{} pixels",
            border_offset,
            inner_width,
            inner_height
        );

        handle_border_interaction_platform_updates(
            x,
            y,
            width,
            height,
            inner_width,
            inner_height,
        );

        if let Err(e) = app_for_cb.emit(
            "region-changed",
            serde_json::json!({
                "x": x,
                "y": y,
                "width": width,
                "height": height
            }),
        ) {
            log::error!("Failed to emit region-changed event: {}", e);
        }

        update_capture_engine_after_border_interaction(
            &engine_for_cb,
            x,
            y,
            width,
            height,
            border_w,
        );

        update_rec_indicator_position(x, y, width, border_w);
    });

    let eng_for_callback = state.capture_engine.clone();
    set_border_live_move_callback(move |x, y, width, height| {
        log::info!(
            "🔍 [LiveCallback] Border reports: ({}, {}) {}x{}",
            x,
            y,
            width,
            height
        );

        handle_border_live_move_platform_updates(x, y, width, height);

        update_rec_indicator_position(x, y, width, border_w);

        update_capture_engine_during_live_move(
            &eng_for_callback,
            x,
            y,
            width,
            height,
            border_w,
        );
    });
}

pub fn create_hollow_border_for_settings(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    settings: &Settings,
) -> Result<HollowBorder, String> {
    // COLORREF format is 0x00BBGGRR, border_color is [R, G, B, A]
    let allow = should_allow_screen_capture(settings);
    log::info!("[MAIN] Setting allow_screen_capture flag to: {}", allow);
    tracing::info!("Setting allow_screen_capture flag to: {}", allow);
    set_allow_screen_capture(allow);
    log::info!("[MAIN] Flag set successfully");

    let border_color = platform::colors::border_rgba_to_native_color(settings.border_color);

    let hollow_border = HollowBorder::new(
        x,
        y,
        width as i32,
        height as i32,
        settings.border_width as i32,
        border_color,
    )
    .ok_or("Failed to create hollow border")?;

    // Capture mode: interior is click-through, only top edge drags
    hollow_border.set_capture_mode();

    // Apply show_border setting
    if !settings.show_border {
        hollow_border.hide();
    }

    Ok(hollow_border)
}

pub fn create_rec_indicator_for_settings(
    x: i32,
    y: i32,
    width: u32,
    settings: &Settings,
) -> Option<RecIndicator> {
    if !settings.show_rec_indicator {
        tracing::debug!("REC indicator disabled in settings");
        return None;
    }

    tracing::debug!("Creating REC indicator");
    if let Some(rec) = RecIndicator::new() {
        rec.set_size(&settings.rec_indicator_size);
        tracing::debug!("Showing REC indicator");
        rec.show(x, y, width as i32, settings.border_width as i32);
        tracing::debug!("Storing REC indicator in global state");
        tracing::info!("REC indicator created and shown");
        Some(rec)
    } else {
        log::warn!("[MAIN] RecIndicator::new() returned None");
        None
    }
}

pub fn create_preview_border(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_width: i32,
    border_color: u32,
) -> Result<HollowBorder, String> {
    let border = HollowBorder::new(x, y, width, height, border_width, border_color)
        .ok_or("Failed to create preview border")?;
    border.set_preview_mode();
    Ok(border)
}

pub fn update_border_for_preview(
    border: &mut HollowBorder,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_width: i32,
    border_color: u32,
) {
    border.update_rect(x, y, width, height);
    border.update_style(border_width, border_color);
    border.set_preview_mode();
    border.show();
}

pub fn store_hollow_border(border: HollowBorder) {
    *crate::app_state::HOLLOW_BORDER.lock().unwrap() = Some(border);
}

pub fn store_destination_window(window: DestinationWindow) {
    *crate::app_state::DESTINATION_WINDOW.lock().unwrap() = Some(window);
}

pub fn store_rec_indicator(rec: RecIndicator) {
    *crate::app_state::REC_INDICATOR.lock().unwrap() = Some(rec);
}

pub fn update_rec_indicator_position(x: i32, y: i32, width: i32, border_width: i32) {
    if let Ok(rec_lock) = crate::app_state::REC_INDICATOR.try_lock() {
        if let Some(rec) = rec_lock.as_ref() {
            rec.update_position(x, y, width, border_width);
        }
    }
}

pub fn prime_capture_border_interaction_from_shortcut() {
    if let Ok(border_lock) = crate::app_state::HOLLOW_BORDER.try_lock() {
        if let Some(border) = border_lock.as_ref() {
            border.prime_interaction_from_shortcut();
        }
    }
}

pub fn update_capture_border_for_preview(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_width: i32,
    border_color: u32,
) -> Result<(), String> {
    if let Ok(mut border_lock) = crate::app_state::HOLLOW_BORDER.try_lock() {
        if let Some(ref mut border) = *border_lock {
            update_border_for_preview(
                border,
                x,
                y,
                width,
                height,
                border_width,
                border_color,
            );
            return Ok(());
        }
        return Err("Capture is active but no border found".to_string());
    }
    tracing::warn!("Could not acquire HOLLOW_BORDER lock in show_preview_border");
    Err("Border is locked".to_string())
}

pub fn set_capture_border_mode_if_active() -> Result<bool, String> {
    if let Ok(mut border_lock) = crate::app_state::HOLLOW_BORDER.try_lock() {
        if let Some(ref mut border) = *border_lock {
            border.set_capture_mode();
            return Ok(true);
        }
        return Ok(false);
    }
    tracing::warn!("Could not acquire HOLLOW_BORDER lock in hide_preview_border");
    Err("Border is locked".to_string())
}

#[cfg(target_os = "windows")]
pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    imp::get_monitors()
}

#[cfg(not(target_os = "windows"))]
pub fn get_monitors(
    window: tauri::Window,
    state: &crate::AppState,
) -> Result<Vec<MonitorInfo>, String> {
    imp::get_monitors(window, state)
}
