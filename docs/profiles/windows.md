# Windows Profile Parameters

These settings override preview window behavior on Windows. They map to Win32 window styles and are optional. When omitted, internal defaults are used.

## Settings
- winapi_destination_overlapped: Use WS_OVERLAPPEDWINDOW instead of WS_POPUP.
- winapi_destination_toolwindow: Apply WS_EX_TOOLWINDOW (hide from taskbar/Alt-Tab).
- winapi_destination_appwindow: Apply WS_EX_APPWINDOW (show in taskbar/window pickers).
- winapi_destination_layered: Apply WS_EX_LAYERED (enables per-window alpha).
- winapi_destination_topmost: Apply WS_EX_TOPMOST.
- winapi_destination_click_through: Apply WS_EX_TRANSPARENT.
- winapi_destination_noactivate: Apply WS_EX_NOACTIVATE.
- winapi_destination_alpha: Alpha value for layered windows (0-255).
- winapi_destination_hide_taskbar_after_ms: If set, hides the window from the taskbar after the delay.

See src/destination_window/windows.rs for the exact behavior.
