# macOS Window Exclusion (ScreenCaptureKit)

## Status
ScreenCaptureKit exclusion is implemented and wired into capture start for Exclude List mode. The preview window is excluded by default; include-only mode is not enforced yet.

## Implementation
- The macOS capture backend uses ScreenCaptureKit and can build SCContentFilter exclusions from WindowIdentifier data.
- The WindowIdentifier struct supports app/window matching and a preview window marker.

Relevant sources:
- src/capture/macos_sck.rs
- src/window_filter.rs
- src/main.rs (start_capture passes settings.window_filter exclusions)
