use crate::destination_window::DestinationWindow;
use crate::settings::Settings;
use rustframe_capture::capture::{CaptureEngine, CaptureFrame, CaptureRect};
use rustframe_capture::window_filter::WindowIdentifier;
use std::sync::{Arc, Mutex};
use rustframe_capture::{config, display_info};

pub fn create_capture_engine_for_settings(
    settings: &Settings,
) -> Result<Box<dyn CaptureEngine>, String> {
    use rustframe_capture::capture::linux::LinuxCaptureEngine;
    let _ = settings; // Only stub method available on Linux for now
    LinuxCaptureEngine::new()
        .map(|e| Box::new(e) as Box<dyn CaptureEngine>)
        .map_err(|e| e.to_string())
}

pub fn build_capture_exclusion_list(_settings: &Settings) -> Vec<WindowIdentifier> {
    vec![WindowIdentifier::preview_window()]
}

pub fn configure_preview_window_for_capture(
    _x: i32,
    _y: i32,
    _width: u32,
    _height: u32,
) {
}

pub fn sync_preview_window_to_border(
    _x: i32,
    _y: i32,
    _width: i32,
    _height: i32,
) {
}

pub fn handle_border_interaction_platform_updates(
    _x: i32,
    _y: i32,
    _width: i32,
    _height: i32,
    _inner_width: i32,
    _inner_height: i32,
) {
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

    if let Ok(mut engine_lock) = engine.try_lock() {
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
}

pub fn handle_border_live_move_platform_updates(
    _x: i32,
    _y: i32,
    _width: i32,
    _height: i32,
) {
}

pub fn update_capture_engine_during_live_move(
    engine: &Arc<Mutex<Option<Box<dyn CaptureEngine>>>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_w: i32,
) {
    if let Ok(mut engine_lock) = engine.try_lock() {
        if let Some(ref mut eng) = *engine_lock {
            let scale_factor = crate::platform::input::get_screen_scale_factor();
            if let Err(e) = eng.set_scale_factor(scale_factor) {
                log::trace!("Failed to set scale factor: {}", e);
            }

            let border_offset = border_w;
            let inner_width = width - (border_offset * 2);
            let inner_height = height - (border_offset * 2);

            let new_region = CaptureRect {
                x: x + border_offset,
                y: y + border_offset,
                width: inner_width as u32,
                height: inner_height as u32,
            };

            if let Err(e) = eng.update_region(new_region) {
                log::trace!("Failed to update capture region during drag: {}", e);
            }
        }
    }
}

pub fn create_separation_layer_for_capture(
    _x: i32,
    _y: i32,
    _width: u32,
    _height: u32,
) {
}

pub fn clear_separation_layer_for_capture() {
}

pub fn ensure_border_hwnd_cleared() -> Result<(), String> {
    Ok(())
}

pub fn post_create_destination_window(_x: i32, _y: i32, _width: u32, _height: u32) {
    let lock = crate::app_state::DESTINATION_WINDOW.lock().unwrap();
    if lock.is_some() {
        tracing::debug!("Destination window stored successfully");
    } else {
        tracing::error!("Failed to store destination window - is None after assignment");
    }
}

pub fn clear_cursor_filtering_after_capture() {
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
    let _ = use_gpu;
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

    window.update_frame(frame.data, frame.width, frame.height);
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

pub fn get_monitors(
    window: tauri::Window,
    state: &crate::AppState,
) -> Result<Vec<crate::monitors::MonitorInfo>, String> {
    super::shared_tauri::get_tauri_monitors(window, state)
}
