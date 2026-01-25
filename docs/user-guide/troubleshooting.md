# Troubleshooting

## macOS: Screen Recording Permission
If capture fails or the preview is black:
1. System Settings -> Privacy & Security -> Screen Recording
2. Enable RustFrame (or your terminal if running via cargo).
3. Restart RustFrame.

The app does not request permission automatically; it checks and fails fast if permission is missing.

## Windows: Black Preview or Capture Failure
1. Open Settings -> Capture Method.
2. Switch to GDI Copy.
3. Restart capture.

## Preview Window Not Listed in Video Call App
- Start capture so the preview window exists.
- On Windows, some profiles may hide the window from the taskbar after a delay.
- On macOS, the preview window must be on-screen (not minimized).

## Zoom Window Sharing
Zoom window sharing is not supported at this time. Zoom may pause the share if the RustFrame preview window is obscured by the separation layer. Use screen/monitor sharing instead.

## Share Content Filters
- macOS: exclude list is applied; include-only mode is not enforced yet.
- Windows: Share Content is hidden and filters are ignored.

## Linux
Linux capture is not implemented; start_capture returns an error.

## Logs
Log directory:
- Windows: %LOCALAPPDATA%\RustFrame\logs
- macOS: ~/Library/Logs/RustFrame
- Linux: ~/.local/share/RustFrame/logs
