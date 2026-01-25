# FAQ

## What is RustFrame?
A screen-region capture tool that creates a shareable preview window for video calls.

## Which platforms are supported?
- Windows 10/11
- macOS 12.3+
Linux capture is not implemented yet.

## Does RustFrame send data anywhere?
No. Capture and rendering are local only.

## How do I share my capture?
Start capture, then share the preview window in your video call app.

## Can I exclude specific windows?
On macOS, the Share Content tab can exclude selected windows; the preview window is always excluded. Include-only mode is not enforced yet. On Windows, Share Content is hidden and filters are ignored.

## Where are settings stored?
- Windows: %APPDATA%\RustFrame\settings.json
- macOS: ~/Library/Application Support/RustFrame/settings.json
- Linux: ~/.config/RustFrame/settings.json
