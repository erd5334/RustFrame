# Multi-Monitor Handling

This document describes how RustFrame detects and handles multiple displays.

## Overview
- Display metadata (bounds, scale, refresh) is gathered at startup.
- The capture region is updated when the border crosses monitor boundaries.
- DPI scaling is applied to keep logical points consistent with pixels.

## References
- src/display_info.rs
- src/platform/services/windows.rs
- src/capture_controller.rs

## TODO
- Add platform-specific notes for macOS and Linux.
- Document edge cases (mixed DPI, negative coordinates, ultrawide).
