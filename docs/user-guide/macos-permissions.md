# macOS Permissions

RustFrame requires Screen Recording permission to capture the screen. It also benefits from Accessibility permission for click highlights.

## Screen Recording (Required)
1. System Settings -> Privacy & Security -> Screen Recording
2. Enable RustFrame (or your terminal if running via cargo)
3. Restart RustFrame

RustFrame checks permission and returns an error if it is missing; it does not request permission automatically.

## Accessibility (Optional, Click Highlights)
Click highlights on macOS use system event taps. If click highlights do not appear:
1. System Settings -> Privacy & Security -> Accessibility
2. Enable RustFrame
3. Restart RustFrame

## Automation (Single Instance Activation)
When a second instance starts, RustFrame tries to bring the first instance to the front using AppleScript (System Events). macOS may prompt for Automation permission. If denied, the app still runs but will not activate the existing window.
