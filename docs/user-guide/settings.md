# Settings Reference

This is a concise reference for settings used by the Rust backend. Some settings are exposed in the UI, and some are hidden in settings.json.

## Settings File Location
- Windows: %APPDATA%\RustFrame\settings.json
- macOS: ~/Library/Application Support/RustFrame/settings.json
- Linux: ~/.config/RustFrame/settings.json

## Mouse and Clicks
- show_cursor: boolean, default false. Show Shadow Cursor (draws a second cursor in the preview; may appear as double cursor in screen share).
- capture_clicks: boolean, default true. Enable click highlights.
- click_highlight_color: [R,G,B,A], default [255, 255, 0, 180].
- click_dissolve_ms: integer, default 300.
- click_highlight_radius: integer, default 20.

## Border
- show_border: boolean, default true.
- border_color: [R,G,B,A], default [255, 0, 0, 255].
- border_width: integer, default 4.

## Performance
- target_fps: integer, default 60.
- gpu_acceleration: boolean, default true. Controls GPU rendering in the preview window.
- capture_method:
  - Windows: Wgc or GdiCopy.
  - macOS: CoreGraphics (used as a label; capture uses ScreenCaptureKit).

## Preview
- preview_mode: WinApiGdi (Windows) or TauriCanvas (not implemented).
- capture_preview_window: boolean, default true on macOS. Hidden setting used to control preview window capture visibility.

## Recording Indicator
- show_rec_indicator: boolean, default true.
- rec_indicator_size: string, default "medium".

## Region Memory
- remember_last_region: boolean, default true.
- last_region: [x, y, width, height] or null.

## Window Filter (Share Content)
- window_filter.mode: "none", "exclude_list", or "include_only".
- window_filter.excluded_windows: list of { app_id, window_name }.
- window_filter.included_windows: list of { app_id, window_name }.
- window_filter.auto_exclude_preview: boolean, default true.
- window_filter.dev_mode: boolean, derived from RUSTFRAME_DEV_MODE env var.

Note: on macOS, excluded windows are applied during capture; include-only mode is not enforced yet. On Windows, the Share Content UI is hidden and filters are ignored.

## Logging
- log_level: Off, Error, Warn, Info, Debug, Trace. Default: Error.
- log_to_file: boolean, default true.
- log_retention_days: integer, default 30.

Log directory:
- Windows: %LOCALAPPDATA%\RustFrame\logs
- macOS: ~/Library/Logs/RustFrame
- Linux: ~/.local/share/RustFrame/logs

## Advanced Windows Settings (hidden)
These are optional overrides for the preview window. When unset, internal defaults are used.
- winapi_destination_alpha
- winapi_destination_topmost
- winapi_destination_click_through
- winapi_destination_toolwindow
- winapi_destination_layered
- winapi_destination_appwindow
- winapi_destination_noactivate
- winapi_destination_overlapped
- winapi_destination_hide_taskbar_after_ms

## Debug Settings (hidden)
- debug_allow_screen_capture: boolean. Allows preview/border windows to be visible in screen capture tools when set.
- RUSTFRAME_ALLOW_SCREEN_CAPTURE env var overrides this.
