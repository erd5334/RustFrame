# Color Formats

This document summarizes how color formats are handled across platforms.

## Summary
- Windows capture frames are typically BGRA.
- Internal processing uses RGBA for click highlights and UI overlays.
- Conversions live in src/platform/colors.rs and capture/render paths.

## References
- src/platform/colors.rs
- src/platform/services/windows.rs
- docs/archive/2026-01-07_color_format_fix.md

## TODO
- Document exact format transitions per capture backend (WGC, GDI, SCK).
- Add a small table with per-platform input/output formats.
