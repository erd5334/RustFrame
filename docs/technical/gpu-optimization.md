# GPU Rendering Notes

This document summarizes the current GPU/CPU rendering paths.

## macOS
- Capture uses ScreenCaptureKit and provides IOSurface-backed frames.
- The preview window uses CALayer with IOSurface for GPU rendering when available.
- If GPU rendering is disabled or a GPU handle is unavailable, the preview window renders CPU pixel buffers.

## Windows
- WGC capture provides D3D11 textures.
- The preview window uses a D3D11 renderer when available.
- If GPU rendering is disabled or unavailable, the preview window renders CPU pixel buffers via GDI.

## Notes
- GPU usage depends on both capture_method and gpu_acceleration.
- The rendering path is selected per frame based on the presence of a GPU texture handle.
