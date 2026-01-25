use crate::destination_window::{DestinationWindow, DestinationWindowConfig};
use crate::settings::Settings;
use rustframe_capture::capture::{
    CaptureEngine, CaptureFrame, CaptureRect, GpuTextureHandle,
};
use rustframe_capture::window_filter::{WindowFilterMode, WindowIdentifier};
use rustframe_capture::{config, display_info};
use std::sync::{Arc, Mutex};

pub fn create_capture_engine_for_settings(
    settings: &Settings,
) -> Result<Box<dyn CaptureEngine>, String> {
    use rustframe_capture::capture::macos::MacOSCaptureEngine;
    let _ = settings; // CoreGraphics is the only method on macOS
    MacOSCaptureEngine::new()
        .map(|e| Box::new(e) as Box<dyn CaptureEngine>)
        .map_err(|e| e.to_string())
}

pub fn create_destination_window_for_settings(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    settings: &Settings,
) -> Result<DestinationWindow, String> {
    // macOS configuration optimized for screen sharing apps (Meet, Zoom, Discord)
    let config = DestinationWindowConfig {
        alpha: Some(255),
        // Don't use floating level - it hides from screen sharing pickers
        topmost: Some(false),
        click_through: Some(true),

        macos_floating_level: Some(false), // Use normal level for visibility
        macos_sharing_type: Some(if settings.capture_preview_window {
            1
        } else {
            0
        }), // 1=ReadOnly(Visible), 0=None(Hidden)
        macos_collection_behavior: None, // Use defaults (managed, joinable, etc.)
        macos_participates_in_cycle: Some(true), // Visible in window pickers

        // Windows fields (ignored on macOS)
        toolwindow: None,
        layered: None,
        appwindow: None,
        noactivate: None,
        overlapped: None,
    };

    tracing::debug!(
        preview_x = x,
        preview_y = y,
        preview_w = width,
        preview_h = height,
        "Creating DestinationWindow - SAME size as border"
    );

    let dest_window = DestinationWindow::new(x, y, width, height, config)
        .ok_or("Failed to create destination window")?;
    tracing::info!("Destination window created successfully");
    Ok(dest_window)
}

pub fn build_capture_exclusion_list(settings: &Settings) -> Vec<WindowIdentifier> {
    let preview_window_id = crate::app_state::DESTINATION_WINDOW
        .lock()
        .unwrap()
        .as_ref()
        .map(|dw| {
            let window_id = dw.get_window_id();
            WindowIdentifier {
                app_id: "com.rustframe.app".to_string(),
                window_name: format!("RustFrame Preview {}", window_id),
            }
        });

    match settings.window_filter.mode {
        WindowFilterMode::ExcludeList => {
            settings.window_filter.get_exclusions(preview_window_id.as_ref())
        }
        WindowFilterMode::None | WindowFilterMode::IncludeOnly => {
            if settings.window_filter.auto_exclude_preview && !settings.window_filter.dev_mode {
                preview_window_id.into_iter().collect()
            } else {
                Vec::new()
            }
        }
    }
}

pub fn configure_preview_window_for_capture(
    _x: i32,
    _y: i32,
    _width: u32,
    _height: u32,
) {
    // macOS preview window configuration is handled during creation.
}

pub fn sync_preview_window_to_border(x: i32, y: i32, width: i32, height: i32) {
    if let Ok(mut dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(ref mut dest_window) = *dest_lock {
            dest_window.set_pos(x, y);
            dest_window.resize(width as u32, height as u32);
        }
    }
}

pub fn handle_border_interaction_platform_updates(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    _inner_width: i32,
    _inner_height: i32,
) {
    log::info!("📍 Callback received: ({}, {}) {}x{}", x, y, width, height);
    if let Ok(border_lock) = crate::app_state::HOLLOW_BORDER.try_lock() {
        if let Some(border) = border_lock.as_ref() {
            let (bx, by, bw, bh) = border.get_rect();
            log::info!("  ✓ Border after: ({}, {}) {}x{}", bx, by, bw, bh);
        }
    }
    if let Ok(dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(dest) = dest_lock.as_ref() {
            if let Some((dx, dy, dw, dh)) = dest.get_rect() {
                log::info!("  ✓ Destination after: ({}, {}) {}x{}", dx, dy, dw, dh);
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

    let mut engine_lock = match engine.try_lock() {
        Ok(e) => e,
        Err(e) => {
            log::error!("❌ Failed to lock capture engine (macOS): {:?}", e);
            return;
        }
    };

    let needs_restart = if let Some(ref eng) = *engine_lock {
        if let Some(macos_eng) =
            eng.as_any()
                .downcast_ref::<rustframe_capture::capture::MacOSCaptureEngine>()
        {
            let current_origin = macos_eng.get_monitor_origin();

            use core_graphics::display::{CGDisplay, CGRect};
            use core_graphics::geometry::{CGPoint, CGSize};

            let rect = CGRect::new(
                &CGPoint::new(center_x as f64, center_y as f64),
                &CGSize::new(1.0, 1.0),
            );
            let display_count = 1;
            let mut display_id: u32 = 0;

            let changed = unsafe {
                if core_graphics::display::CGGetDisplaysWithRect(
                    rect,
                    display_count,
                    &mut display_id,
                    std::ptr::null_mut(),
                ) == 0
                {
                    let display = CGDisplay::new(display_id);
                    let bounds = display.bounds();
                    let new_origin = (bounds.origin.x as i32, bounds.origin.y as i32);

                    if current_origin.0 != new_origin.0 || current_origin.1 != new_origin.1 {
                        log::info!(
                            "🖥️  Monitor changed! Old origin: {:?}, New origin: {:?}",
                            current_origin,
                            new_origin
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

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
    if let Ok(dest_lock) = crate::app_state::DESTINATION_WINDOW.try_lock() {
        if let Some(ref dest) = *dest_lock {
            log::info!(
                "  → [Destination] Updating to: ({}, {}) {}x{}",
                x,
                y,
                width,
                height
            );
            dest.update_position(x, y, width as u32, height as u32);
        }
    }

    restore_window_z_order_macos();
    log::info!("  → Z-order restored during drag");
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

pub fn render_frame_to_destination(
    window: &mut DestinationWindow,
    mut frame: CaptureFrame,
    use_gpu: bool,
    capture_clicks_enabled: bool,
    click_color: [u8; 4],
    click_dissolve_ms: u64,
    click_radius: u32,
) {
    let mut macos_clicks: Vec<(f32, f32, f32, f32, f32, f32, f32)> = Vec::new();

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
            let [r, g, b, _a] = config::colors::rgba_u8_to_f32(click_color);
            let scale = display.scale_factor as f32;

            for click in &clicks {
                let frame_x = click.x as f32 - offset_x_pixels as f32;
                let frame_y = click.y as f32 - offset_y_pixels as f32;
                let age_ms = click.timestamp.elapsed().as_millis() as f32;
                let alpha = 1.0 - (age_ms / click_dissolve_ms as f32).min(1.0);
                macos_clicks.push((
                    frame_x / scale,
                    frame_y / scale,
                    click_radius as f32,
                    r,
                    g,
                    b,
                    alpha,
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
        if let Some(GpuTextureHandle::Metal {
            iosurface_ptr,
            crop_x,
            crop_y,
            crop_w,
            crop_h,
            ..
        }) = frame.gpu_texture
        {
            window.update_frame_from_iosurface_ptr(
                iosurface_ptr,
                crop_x,
                crop_y,
                crop_w,
                crop_h,
                Some(&macos_clicks),
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
            data[idx] = (color[0] as f32 * final_alpha + data[idx] as f32 * inv_alpha) as u8;
            data[idx + 1] =
                (color[1] as f32 * final_alpha + data[idx + 1] as f32 * inv_alpha) as u8;
            data[idx + 2] =
                (color[2] as f32 * final_alpha + data[idx + 2] as f32 * inv_alpha) as u8;
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

pub fn post_create_destination_window(x: i32, y: i32, width: u32, height: u32) {
    // Verify window is stored
    let lock = crate::app_state::DESTINATION_WINDOW.lock().unwrap();
    if lock.is_some() {
        tracing::debug!("Destination window stored successfully");
    } else {
        tracing::error!("Failed to store destination window - is None after assignment");
    }
    drop(lock);

    log::info!(
        "✅ Windows created (Border, Preview) with SAME dimensions: ({}, {}) {}x{}",
        x,
        y,
        width,
        height
    );
}

pub fn clear_cursor_filtering_after_capture() {
}

fn restore_window_z_order_macos() {
    // Separation layer removed - simplified Z-order
    if let Ok(border_lock) = crate::app_state::HOLLOW_BORDER.try_lock() {
        if let Some(border) = border_lock.as_ref() {
            // Border is topmost
            let _ = border;
        }
    }
}

pub fn get_monitors(
    window: tauri::Window,
    state: &crate::AppState,
) -> Result<Vec<crate::monitors::MonitorInfo>, String> {
    super::shared_tauri::get_tauri_monitors(window, state)
}
