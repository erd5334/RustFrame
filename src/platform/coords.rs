#[cfg(target_os = "macos")]
use crate::display_info;

/// Convert Y for a rect between top-left and bottom-left origins (macOS).
/// For non-mac platforms, returns the input unchanged.
pub fn flip_y_rect_with_height(
    y_points: f64,
    height_points: f64,
    screen_height_points: f64,
) -> f64 {
    #[cfg(target_os = "macos")]
    {
        display_info::flip_y_rect_with_height(y_points, height_points, screen_height_points)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = height_points;
        let _ = screen_height_points;
        y_points
    }
}


/// Convert macOS CGEvent coordinates to capture-space (top-left, pixels).
#[allow(dead_code)]
pub fn macos_event_to_screen_pixels(x: f64, y: f64) -> (i32, i32) {
    #[cfg(target_os = "macos")]
    {
        let display = display_info::get();
        display.macos_event_to_screen_pixels(x, y)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = x;
        let _ = y;
        (0, 0)
    }
}

