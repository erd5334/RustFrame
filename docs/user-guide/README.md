# RustFrame User Guide

RustFrame is a screen-region capture tool that creates a shareable preview window. You select a region, start capture, and share the preview window in your video call app.

## Contents
- Installation: installation.md
- Quick Start: quick-start.md
- Features: features.md
- Settings: settings.md
- Troubleshooting: troubleshooting.md
- FAQ: faq.md
- macOS Permissions: macos-permissions.md

## Supported Platforms
- Windows 10/11: Supported. Capture methods: Windows Graphics Capture (WGC) or GDI.
- macOS 12.3+: Supported. Capture method: ScreenCaptureKit.
- Linux: Not supported yet. The capture engine is a stub and capture will fail.

## What RustFrame Does
- Lets you capture a specific region of your screen.
- Displays a shareable preview window for video call apps.
- Supports multi-monitor setups.
- Optional click highlights and recording indicator overlays.
- Profiles let you apply platform-specific window behavior overrides.

## Share Content Filters
On macOS, the Share Content tab can exclude selected windows during capture; the preview window is always excluded. Include-only mode is not enforced yet. On Windows, the Share Content UI is hidden and filters are ignored.

Next: quick-start.md
