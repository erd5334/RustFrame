# macOS Permissions

RustFrame requires Screen Recording and Accessibility permissions to function correctly on macOS. File system access is also required for settings, profiles, and exports.

## Screen Recording (Required)
1. System Settings -> Privacy & Security -> Screen Recording
2. Enable RustFrame (or your terminal if running via cargo)
3. Restart RustFrame

RustFrame checks permission and returns an error if it is missing; it does not request permission automatically.

## Accessibility (Required)
On macOS use system event taps and RustFrame relies on Accessibility permission for core functionality.
1. System Settings -> Privacy & Security -> Accessibility
2. Enable RustFrame
3. Restart RustFrame

## Open Anyway (Gatekeeper)
If macOS blocks RustFrame because it was downloaded from the internet:
1. System Settings -> Privacy & Security
2. Scroll to the security section and click "Open Anyway" for RustFrame
3. Launch RustFrame again

## Automation (Single Instance Activation)
When a second instance starts, RustFrame tries to bring the first instance to the front using AppleScript (System Events). macOS may prompt for Automation permission. If denied, the app still runs but will not activate the existing window.
