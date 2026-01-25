use rustframe_capture::config;

pub fn border_rgba_to_native_color(rgba: [u8; 4]) -> u32 {
    #[cfg(windows)]
    {
        config::colors::rgba_to_bgr_u32(rgba)
    }

    #[cfg(target_os = "macos")]
    {
        config::colors::rgba_to_bgr_u32(rgba)
    }

    #[cfg(target_os = "linux")]
    {
        config::colors::rgba_to_bgr_u32(rgba)
    }
}

pub fn native_border_color_to_rgb_f64(color: u32) -> (f64, f64, f64) {
    #[cfg(target_os = "macos")]
    {
        config::colors::bgr_u32_to_rgb_f64(color)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = color;
        (0.0, 0.0, 0.0)
    }
}

#[allow(dead_code)]
pub fn rgb_u32_to_colorref(color: u32) -> u32 {
    #[cfg(windows)]
    {
        config::colors::rgb_u32_to_colorref(color)
    }

    #[cfg(not(windows))]
    {
        let _ = color;
        0
    }
}
