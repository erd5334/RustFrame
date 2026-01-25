use std::sync::Mutex;

use tauri::State;

use crate::hollow_border::HollowBorder;
use crate::{platform, AppState};

// Preview border for settings - shows border without starting capture
#[allow(clippy::incompatible_msrv)]
static PREVIEW_BORDER: Mutex<Option<HollowBorder>> = Mutex::new(None);

pub(crate) fn clear_preview_border() {
    if let Ok(mut preview) = PREVIEW_BORDER.lock() {
        *preview = None;
    }
}

#[tauri::command]
pub fn show_preview_border(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    border_width: i32,
    border_color: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Check if capture is active - if so, switch existing border to preview mode
    // instead of creating a new one (they share global state)
    let is_capturing = *state.is_capturing.lock().unwrap();

    if is_capturing {
        // Capture is active - switch the capture border to preview mode
        // This makes it draggable from interior while maintaining capture
        return platform::services::update_capture_border_for_preview(
            x,
            y,
            width,
            height,
            border_width,
            border_color,
        );
    }

    // No capture active - create or update preview border
    let mut preview = PREVIEW_BORDER.lock().map_err(|e| e.to_string())?;

    // If preview border already exists, just update it
    if let Some(border) = preview.as_mut() {
        platform::services::update_border_for_preview(
            border,
            x,
            y,
            width,
            height,
            border_width,
            border_color,
        );

        // Update preview/destination window to match border
        platform::services::sync_preview_window_to_border(x, y, width, height);

        return Ok(());
    }

    // Create new preview border
    let border = platform::services::create_preview_border(
        x,
        y,
        width,
        height,
        border_width,
        border_color,
    )?;

    *preview = Some(border);

    // Update preview/destination window to match border
    platform::services::sync_preview_window_to_border(x, y, width, height);

    Ok(())
}

#[tauri::command]
pub fn hide_preview_border(state: State<'_, AppState>) -> Result<(), String> {
    // If capture is active, switch border back to capture mode
    let is_capturing = *state.is_capturing.lock().unwrap();

    if is_capturing {
        // Use try_lock with timeout to prevent deadlock
        let updated = platform::services::set_capture_border_mode_if_active()?;
        if updated {
            return Ok(());
        }
    }

    // No capture active - hide the preview border
    clear_preview_border();
    Ok(())
}

#[tauri::command]
pub fn update_preview_border(x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    let preview = PREVIEW_BORDER.lock().map_err(|e| e.to_string())?;
    if let Some(border) = preview.as_ref() {
        border.update_rect(x, y, width, height);
    }
    Ok(())
}

#[tauri::command]
pub fn update_preview_border_style(border_width: i32, border_color: u32) -> Result<(), String> {
    let preview = PREVIEW_BORDER.lock().map_err(|e| e.to_string())?;
    if let Some(border) = preview.as_ref() {
        border.update_style(border_width, border_color);
    }
    Ok(())
}

#[tauri::command]
pub fn get_preview_border_rect() -> Result<Option<(i32, i32, i32, i32)>, String> {
    let preview = PREVIEW_BORDER.lock().map_err(|e| e.to_string())?;
    if let Some(border) = preview.as_ref() {
        Ok(Some(border.get_rect()))
    } else {
        Ok(None)
    }
}
