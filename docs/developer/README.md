# Developer Guide

This guide describes the current RustFrame architecture and code layout.

## Project Layout
- src/main.rs: Tauri entry point, command handlers, render loop.
- src/lib.rs: capture library exports.
- src/capture/: platform capture engines (Windows, macOS, Linux stub).
- src/destination_window/: native preview window per platform.
- src/hollow_border/: border window per platform.
- src/rec_indicator.rs: recording indicator window.
- src/platform.rs + src/platform/window_enumerator.rs: input utilities and window enumeration.
- src/window_filter.rs: window include/exclude data types and capture filtering helpers.
- ui/: React frontend.

## Capture Engine Interface
The capture engine trait lives in src/capture/mod.rs:

- start(region, show_cursor, excluded_windows) // show_cursor = shadow cursor overlay in preview frames
- stop()
- has_new_frame(), get_frame()
- update_region(), set_cursor_visible(), set_scale_factor()

macOS uses ScreenCaptureKit; Windows uses WGC or GDI; Linux is a stub.

## Preview Window
The preview window is created from src/destination_window/. It supports:
- CPU rendering with pixel buffers.
- GPU rendering when gpu_texture handles are available.

## Window Filters
The Share Content UI stores selections in settings.window_filter. On macOS, these selections are passed into ScreenCaptureKit exclusion filtering; on Windows the Share Content UI is hidden and filters are ignored.

## Build
See building.md

## Tests
The workspace includes an app binary that links Tauri and platform UI frameworks.
For fast test runs without the app binary, use core tests only:

- RustRover run configuration:
  - Command: `test`
  - Arguments: `--no-default-features --lib --tests`

To include app-level tests (runs `src/main.rs` and Tauri-linked code):

- RustRover run configuration:
  - Command: `test`
  - Arguments: `--no-default-features --features app --bin rustframe`
