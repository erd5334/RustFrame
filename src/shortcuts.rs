use serde::Serialize;

use tauri::Emitter;
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::settings::ShortcutSettings;

pub const SHORTCUTS_ENABLED: bool = false;

#[derive(Clone, Copy)]
pub enum ShortcutAction {
    StartCapture,
    StopCapture,
    ZoomIn,
    ZoomOut,
}

impl ShortcutAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShortcutAction::StartCapture => "start_capture",
            ShortcutAction::StopCapture => "stop_capture",
            ShortcutAction::ZoomIn => "zoom_in",
            ShortcutAction::ZoomOut => "zoom_out",
        }
    }
}

#[derive(Clone, Serialize)]
struct ShortcutEvent {
    action: &'static str,
}

fn register_one(
    app: &tauri::AppHandle,
    accelerator: &str,
    action: ShortcutAction,
) -> Result<(), String> {
    if accelerator.trim().is_empty() {
        return Ok(());
    }

    let action_name = action.as_str();
    #[cfg(target_os = "macos")]
    let focus_on_trigger = matches!(action, ShortcutAction::StartCapture);
    app.global_shortcut()
        .on_shortcut(accelerator, move |app_handle, _shortcut, _event| {
            #[cfg(target_os = "macos")]
            if focus_on_trigger {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.set_focus();
                }
            }
            let _ = app_handle.emit("shortcut-action", ShortcutEvent { action: action_name });
        })
        .map_err(|e| format!("Failed to register {}: {}", action_name, e))
}

fn register_all(app: &tauri::AppHandle, shortcuts: &ShortcutSettings) -> Result<(), String> {
    register_one(app, &shortcuts.start_capture, ShortcutAction::StartCapture)?;
    register_one(app, &shortcuts.stop_capture, ShortcutAction::StopCapture)?;
    register_one(app, &shortcuts.zoom_in, ShortcutAction::ZoomIn)?;
    register_one(app, &shortcuts.zoom_out, ShortcutAction::ZoomOut)?;
    Ok(())
}

pub fn apply_shortcuts(
    app: &tauri::AppHandle,
    shortcuts: &ShortcutSettings,
    previous: Option<&ShortcutSettings>,
) -> Result<(), String> {
    if !SHORTCUTS_ENABLED {
        return Ok(());
    }

    if let Err(e) = app.global_shortcut().unregister_all() {
        log::warn!("Failed to clear global shortcuts: {}", e);
    }

    if let Err(e) = register_all(app, shortcuts) {
        log::warn!("Shortcut registration failed, rolling back: {}", e);
        if let Err(err) = app.global_shortcut().unregister_all() {
            log::warn!("Failed to clear shortcuts after rollback: {}", err);
        }

        if let Some(prev) = previous {
            if let Err(err) = register_all(app, prev) {
                log::warn!("Failed to restore previous shortcuts: {}", err);
            }
        }
        return Err(e);
    }

    Ok(())
}
