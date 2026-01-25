use serde::{Deserialize, Serialize};

use rustframe_capture::config;
use rustframe_capture::window_filter::WindowFilterSettings;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PreviewMode {
    TauriCanvas, // Cross-platform, WebView overhead (not implemented on macOS/Linux)
    #[cfg(windows)]
    WinApiGdi, // Windows-only, lightweight native
    #[cfg(not(windows))]
    Native, // macOS/Linux native preview window
}

impl Default for PreviewMode {
    fn default() -> Self {
        #[cfg(windows)]
        {
            PreviewMode::WinApiGdi
        }
        #[cfg(not(windows))]
        {
            PreviewMode::Native
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CaptureMethod {
    /// Windows.Graphics.Capture (Windows only, modern, GPU-backed)
    #[cfg(windows)]
    Wgc,
    /// GDI screen copy (Windows only, broad compatibility)
    #[cfg(windows)]
    GdiCopy,
    /// macOS/Linux CoreGraphics-based capture
    #[cfg(not(windows))]
    CoreGraphics,
}

impl Default for CaptureMethod {
    fn default() -> Self {
        #[cfg(windows)]
        {
            CaptureMethod::Wgc
        }
        #[cfg(not(windows))]
        {
            CaptureMethod::CoreGraphics
        }
    }
}

impl std::fmt::Display for CaptureMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(windows)]
            CaptureMethod::Wgc => write!(f, "WGC"),
            #[cfg(windows)]
            CaptureMethod::GdiCopy => write!(f, "GdiCopy"),
            #[cfg(not(windows))]
            CaptureMethod::CoreGraphics => write!(f, "CoreGraphics"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutSettings {
    #[serde(default = "default_shortcut_start_capture")]
    pub start_capture: String,
    #[serde(default = "default_shortcut_stop_capture")]
    pub stop_capture: String,
    #[serde(default = "default_shortcut_zoom_in")]
    pub zoom_in: String,
    #[serde(default = "default_shortcut_zoom_out")]
    pub zoom_out: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            start_capture: default_shortcut_start_capture(),
            stop_capture: default_shortcut_stop_capture(),
            zoom_in: default_shortcut_zoom_in(),
            zoom_out: default_shortcut_zoom_out(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Mouse & Cursor
    pub show_cursor: bool,
    #[serde(default = "default_capture_clicks")]
    pub capture_clicks: bool,
    #[serde(default = "default_click_color")]
    pub click_highlight_color: [u8; 4],
    #[serde(default = "default_click_dissolve_ms")]
    pub click_dissolve_ms: u32,
    #[serde(default = "default_click_radius")]
    pub click_highlight_radius: u32,

    // Border
    pub show_border: bool,
    pub border_color: [u8; 4],
    pub border_width: u32,

    // Performance
    pub target_fps: u32,
    #[serde(default = "default_gpu_acceleration")]
    pub gpu_acceleration: bool,

    // Capture Method
    #[serde(default)]
    pub capture_method: CaptureMethod,

    // Preview Mode
    pub preview_mode: PreviewMode,
    #[serde(default)]
    pub capture_preview_window: bool,

    // Advanced (hidden) WinAPI Destination Window overrides (Windows-only behavior)
    // These are intentionally not exposed in the UI by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_alpha: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_topmost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_click_through: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_toolwindow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_layered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_appwindow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_noactivate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_overlapped: Option<bool>,

    // Optional post-start behavior (Windows-only UI behavior)
    // If set, after starting capture we will try to hide the preview window from the taskbar/Alt-Tab.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winapi_destination_hide_taskbar_after_ms: Option<u32>,

    // Region Memory
    pub remember_last_region: bool,
    pub last_region: Option<[i32; 4]>, // [x, y, width, height]

    // REC Indicator
    pub show_rec_indicator: bool,
    pub rec_indicator_size: String, // "small", "medium", "large"

    // Window Filtering (Exclusion/Inclusion)
    #[serde(default)]
    pub window_filter: WindowFilterSettings,

    // Logging
    #[serde(default = "default_log_level")]
    pub log_level: String, // "Off", "Error", "Warn", "Info", "Debug", "Trace"
    #[serde(default = "default_log_to_file")]
    pub log_to_file: bool,
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u32,

    // UI
    #[serde(default = "default_ui_zoom")]
    pub ui_zoom: f64,
    #[serde(default)]
    pub shortcuts: ShortcutSettings,

    // Debug/Advanced Features (hidden from UI)
    /// Allow preview/destination windows to be visible in screen capture tools (Snipping Tool, OBS, etc.)
    /// This is a hidden setting - not exposed in UI, only via settings.json manual edit
    /// Default: false (windows are excluded from capture for privacy/performance)
    #[serde(default)]
    pub debug_allow_screen_capture: Option<bool>,
}

// Default functions for serde
fn default_capture_clicks() -> bool {
    true // Enable click capture by default
}

fn default_click_color() -> [u8; 4] {
    [255, 255, 0, 180] // Yellow with alpha
}

fn default_click_radius() -> u32 {
    20 // Default radius in points (will be scaled for Retina)
}

fn default_click_dissolve_ms() -> u32 {
    300
}

fn default_log_level() -> String {
    "Error".to_string() // Default: only errors
}

fn default_log_to_file() -> bool {
    true // Enable file logging by default
}

fn default_log_retention_days() -> u32 {
    config::capture::LOG_RETENTION_DAYS as u32
}

fn default_gpu_acceleration() -> bool {
    true // GPU acceleration enabled with retained IOSurface
}

fn default_ui_zoom() -> f64 {
    1.0
}

fn default_shortcut_start_capture() -> String {
    "CmdOrCtrl+Shift+R".to_string()
}

fn default_shortcut_stop_capture() -> String {
    "CmdOrCtrl+Shift+S".to_string()
}

fn default_shortcut_zoom_in() -> String {
    "CmdOrCtrl+Shift+Equal".to_string()
}

fn default_shortcut_zoom_out() -> String {
    "CmdOrCtrl+Shift+Minus".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        let capture_preview_window = cfg!(target_os = "macos");
        Self {
            show_cursor: false, // Shadow cursor disabled by default to avoid double cursor in screen sharing
            capture_clicks: true, // Default to enabled for testing
            click_highlight_color: config::capture::DEFAULT_CLICK_HIGHLIGHT_COLOR,
            click_dissolve_ms: 300, // Reduced from 5000ms - 300ms is plenty for click feedback
            click_highlight_radius: 20,
            show_border: true,
            border_color: [255, 0, 0, 255],
            border_width: config::window::DEFAULT_BORDER_WIDTH as u32,
            target_fps: config::capture::DEFAULT_TARGET_FPS,
            gpu_acceleration: true,
            capture_method: CaptureMethod::default(),
            preview_mode: PreviewMode::default(),
            capture_preview_window,
            winapi_destination_alpha: None,
            winapi_destination_topmost: None,
            winapi_destination_click_through: None,
            winapi_destination_toolwindow: None,
            winapi_destination_layered: None,
            winapi_destination_appwindow: None,
            winapi_destination_noactivate: None,
            winapi_destination_overlapped: None,
            winapi_destination_hide_taskbar_after_ms: None,
            remember_last_region: true,
            last_region: Some([100, 100, 600, 400]),
            show_rec_indicator: true,
            rec_indicator_size: config::rec_indicator::DEFAULT_SIZE.to_string(),
            window_filter: WindowFilterSettings::default(),
            log_level: "Error".to_string(),
            log_to_file: true,
            log_retention_days: config::capture::LOG_RETENTION_DAYS as u32,
            ui_zoom: 1.0,
            shortcuts: ShortcutSettings::default(),
            debug_allow_screen_capture: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shadow_cursor_disabled() {
        let settings = Settings::default();
        assert!(!settings.show_cursor);
    }

    #[test]
    fn capture_preview_window_matches_platform() {
        let settings = Settings::default();
        assert_eq!(settings.capture_preview_window, cfg!(target_os = "macos"));
    }

    #[test]
    fn default_shortcuts_are_set() {
        let settings = Settings::default();
        assert!(!settings.shortcuts.start_capture.is_empty());
        assert!(!settings.shortcuts.stop_capture.is_empty());
        assert!(!settings.shortcuts.zoom_in.is_empty());
        assert!(!settings.shortcuts.zoom_out.is_empty());
    }
}

/// Check if screen capture visibility should be allowed for preview/destination windows
/// This checks both environment variable and hidden settings key
///
/// Priority order:
/// 1. Environment variable (RUSTFRAME_ALLOW_SCREEN_CAPTURE)
/// 2. Hidden settings key (debug_allow_screen_capture)
/// 3. Dev mode (debug builds always allow)
///
/// Returns true if windows should be visible in screen capture tools (Snipping Tool, OBS, etc.)
pub fn should_allow_screen_capture(settings: &Settings) -> bool {
    // 1. Check environment variable
    let env_result = std::env::var(config::debug::ALLOW_SCREEN_CAPTURE_ENV);
    tracing::info!("Environment variable check: {:?}", env_result);
    if let Ok(value) = env_result {
        // Parse value: "1", "true", "yes" = true, anything else = false
        let allow = value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes");
        if allow {
            tracing::info!("✅ Screen capture ALLOWED via environment variable (value: {})", value);
            log::info!("✅ Screen capture ALLOWED via environment variable (value: {})", value);
            return true;
        } else {
            tracing::info!("❌ Screen capture BLOCKED via environment variable (value: {})", value);
            log::info!("❌ Screen capture BLOCKED via environment variable (value: {})", value);
            return false;
        }
    }

    // 2. Check hidden settings key
    if let Some(allow) = settings.debug_allow_screen_capture {
        if allow {
            tracing::info!("✅ Screen capture ALLOWED via settings.json");
            log::info!("✅ Screen capture ALLOWED via settings.json");
            return true;
        }
    }

    // Default: exclude from capture
    tracing::info!("❌ Screen capture BLOCKED (default)");
    log::info!("❌ Screen capture BLOCKED (default)");
    false
}
