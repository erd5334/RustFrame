# Shortcut Border Drag/Resize Issue (macOS)

Status: Unresolved. Feature is disabled (shortcuts hidden + no hook).

## Summary
When capture is started via global shortcut, the hollow border becomes
non-interactive after a short time. Dragging/resizing works only in
some sessions. If the border "refreshes" (briefly disappears/reappears)
right after start, drag/resize tends to work. If that refresh does not
happen, drag/resize fails even after waiting.

Button-based start does not show this issue.

## Repro Steps
1) Launch app.
2) Start capture via shortcut.
3) Wait a few seconds (or not) and attempt edge drag/resize.
4) Repeat without app restart.

Observed:
- Sometimes the border briefly disappears and returns (a refresh).
- If refresh happens, drag/resize usually works.
- If refresh does not happen, drag/resize fails even after waiting.

## Notable Behaviors
- Cursor position (inside/outside border) does not guarantee success.
- Border can become click-through and stay that way.
- No consistent correlation with immediate drag vs delayed drag.

## Suspected Areas
1) macOS mouse hover detection for border edges (poller).
2) Interaction between "prime" logic and poller state (desync).
3) Coordinate system mismatch (AppKit vs CGEvent, points vs pixels).
4) Border window losing/never gaining correct mouse event state.

## Recent Changes Attempted
- Centralized CGEvent coordinate conversion with runtime detection.
- Poller hit-test tries multiple coordinate modes (top/bottom, points/pixels).
- Removed prime logic toggling ignoresMouseEvents directly; poller owns it.
- Fixed last_region saving to use outer rect (prevents size shrink).

Files touched (recent):
- src/hollow_border/macos.rs
- src/display_info.rs
- src/platform/coords.rs
- src/platform.rs
- src/capture_controller.rs
- src/platform/services/mod.rs

## What We Need To Measure Next
1) Poller debug:
   - Log whether "should_interact" ever flips true after shortcut start.
   - Log candidate coordinate hits vs border rect.
2) Window event state:
   - Log ignoresMouseEvents changes on main thread.
   - Log when window becomes key (if at all).
3) Border refresh trigger:
   - Identify what causes the brief disappear/reappear.
   - Compare state transitions between "refresh" vs "no refresh" runs.

## Proposed Debug Instrumentation
- Add a short-lived debug trace window (first 3s after start):
  - mouse location (raw CGEvent + derived points)
  - border rect + hit result
  - ignoresMouseEvents flag
- Add a one-time log in set_capture_mode and set_preview_mode.

## Temporary Mitigation
Shortcuts are disabled until root cause is identified.

### Where Disabled
Rust:
- src/shortcuts.rs: SHORTCUTS_ENABLED = false
- src/main.rs: plugin + registration gated
- src/commands/settings.rs: apply_shortcuts gated

UI:
- ui/src/featureFlags.ts: SHORTCUTS_ENABLED = false
- ui/src/components/SettingsDialog.tsx: Shortcuts tab hidden
- ui/src/components/HelpOverlay.tsx: shortcuts section hidden
- ui/src/App.tsx: shortcut-action listener gated

## Re-enable Steps (later)
1) Set SHORTCUTS_ENABLED = true in:
   - src/shortcuts.rs
   - ui/src/featureFlags.ts
2) Rebuild.
3) Confirm shortcuts tab is visible and hook registers.

