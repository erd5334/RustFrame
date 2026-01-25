# Windows Window Filtering Limitations

## Summary
Windows does not provide a native API to exclude arbitrary windows from a region capture. RustFrame therefore does not apply user-selected include/exclude lists on Windows.

## Current Behavior
- The capture pipeline excludes the preview window by default to avoid feedback loops.
- User-selected window filters are stored in settings but are ignored during capture on Windows.

## Why This Is Hard on Windows
Windows Graphics Capture can capture a monitor or a window, but it does not expose per-window exclusion for region capture. Any exclusion would need to happen after capture (CPU masking), which produces visible artifacts in video call apps.

## macOS Contrast
ScreenCaptureKit supports exclusions, and RustFrame wires user-selected exclusions into capture start on macOS.
