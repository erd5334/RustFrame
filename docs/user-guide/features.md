# Features

This document describes features that are implemented in the current codebase.

## Capture Region
- The hollow border window shows the capture region.
- You can drag/resize the border in preview mode and while capturing.
- The capture region excludes the border thickness.
- Multi-monitor support is implemented by detecting the border center point and restarting capture on monitor changes.

## Preview Window
- A native preview window is created and rendered by the Rust backend.
- This is the window you share in your video call app.

## Cursor and Click Highlights
- Show Shadow Cursor draws a second cursor inside the preview frame (off by default to avoid double cursor in screen share).
- Click highlights are supported via global input capture (Windows) and event tap (macOS).
- Click highlights can be toggled and customized (color, radius, dissolve time).

## Recording Indicator
- Optional REC indicator window follows the capture border.
- Size options: small, medium, large.

## Capture Methods
- Windows: WGC (Windows Graphics Capture) or GDI Copy.
- macOS: ScreenCaptureKit only (macOS 12.3+ required).
- Linux: Not supported yet.

## Preview Modes
- Windows:
  - WinAPI GDI: implemented (native preview window).
  - Tauri Canvas: not implemented; selecting it returns an error.
- macOS/Linux: native preview window only.

## Profiles
- Profiles are JSON overrides stored under the RustFrame config directory in a Profiles/<os> subfolder.
- Built-in profiles are copied from resources/profiles/<os> on first run.

## Share Content Filters
- The UI can enumerate windows and save include/exclude lists.
- On macOS, excluded windows are passed into ScreenCaptureKit filtering; the preview window is always excluded.
- Include-only mode is not enforced yet; Windows ignores Share Content filters and hides the tab.
