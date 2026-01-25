use crate::platform::window_enumerator::{self, AvailableApp};
use crate::platform;

#[tauri::command]
pub fn get_border_rect() -> Option<[i32; 4]> {
    platform::services::get_capture_inner_rect()
        .map(|r| [r.0, r.1, r.2, r.3])
}

#[tauri::command]
pub async fn get_available_windows() -> Result<Vec<AvailableApp>, String> {
    window_enumerator::enumerate_windows()
        .map_err(|e| format!("Failed to enumerate windows: {}", e))
}
