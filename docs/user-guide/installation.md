# Installation Guide

## System Requirements

### Windows
- Windows 10/11.
- Windows Graphics Capture (WGC) requires Windows 10 1903+.
- DirectX 11 compatible GPU recommended.

### macOS
- macOS 12.3 or newer (ScreenCaptureKit requirement).
- Screen Recording permission is required.

### Linux
- Not supported yet. The Linux capture engine is a stub and capture fails.

## Download

### Pre-built Binaries
1. Visit the releases page for your project distribution.
2. Download the Windows installer (MSI/NSIS) or macOS app bundle.

Linux binaries are not provided at this time.

## Installation Steps

### Windows
1. Run the installer.
2. Launch RustFrame.

### macOS
1. Move RustFrame.app to /Applications.
2. Launch RustFrame.
3. Grant Screen Recording permission in System Settings:
   - System Settings -> Privacy & Security -> Screen Recording
   - Enable RustFrame
4. Restart RustFrame after granting permission.

## Building from Source
See ../developer/building.md
