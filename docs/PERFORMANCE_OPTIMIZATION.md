# Performance Notes

This document summarizes current performance-related behavior without hard numbers.

## Key Factors
- Target FPS directly impacts CPU/GPU usage.
- Capture region size impacts memory bandwidth.
- Click highlights add extra rendering work.

## Current Pipeline
- Capture engines provide CPU frames and, when available, GPU texture handles.
- The preview window uses GPU rendering when gpu_acceleration is enabled and a GPU handle is present.
- If GPU rendering is disabled or unavailable, the preview window falls back to CPU rendering.

## Practical Tips
- Lower FPS if CPU usage is high.
- Reduce capture region size if performance is poor.
- Disable click highlights if you do not need them.
