# Technical Documentation

This section contains low-level implementation notes and platform-specific details.

## Contents
- gpu-optimization.md
- macos-window-exclusion.md
- windows-window-exclusion.md
- macos-window-visibility.md
- WINDOWS_LIMITATIONS.md

## Platform Summary

| Platform | Capture Engine | Preview Rendering | Notes |
| --- | --- | --- | --- |
| Windows | WGC or GDI | D3D11 (GPU) or GDI (CPU) | WGC requires Windows 10 1903+ |
| macOS | ScreenCaptureKit | CALayer/IOSurface | Requires macOS 12.3+ |
| Linux | Stub | Stub | Capture not implemented |
