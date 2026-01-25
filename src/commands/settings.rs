use tauri::State;

use crate::{logging, settings_io, shortcuts, AppState};
use crate::settings::Settings;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().unwrap();
    Ok(settings.clone())
}

#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if !shortcuts::SHORTCUTS_ENABLED {
        let _ = app;
    }

    let previous_settings = state.settings.lock().unwrap().clone();
    let mut effective_settings = settings.clone();

    {
        let mut app_settings = state.settings.lock().unwrap();
        *app_settings = effective_settings.clone();
    }

    let mut shortcut_error: Option<String> = None;
    if shortcuts::SHORTCUTS_ENABLED {
        if let Err(e) = shortcuts::apply_shortcuts(
            &app,
            &effective_settings.shortcuts,
            Some(&previous_settings.shortcuts),
        ) {
            effective_settings.shortcuts = previous_settings.shortcuts.clone();
            let mut app_settings = state.settings.lock().unwrap();
            *app_settings = effective_settings.clone();
            shortcut_error = Some(e);
        }
    }

    // Save to disk (merge with existing JSON to preserve unknown/manual keys)
    let _ = settings_io::persist_settings_to_disk(&effective_settings);

    // If logging settings changed, reinitialize logger
    if effective_settings.log_level != previous_settings.log_level
        || effective_settings.log_to_file != previous_settings.log_to_file
    {
        tracing::info!(
            old_level = %previous_settings.log_level,
            new_level = %effective_settings.log_level,
            old_file = previous_settings.log_to_file,
            new_file = effective_settings.log_to_file,
            "Logging settings changed, reinitializing logger"
        );

        let log_level = effective_settings
            .log_level
            .parse::<logging::LogLevel>()
            .unwrap_or(logging::LogLevel::Error);

        if let Err(e) = logging::init_logging(log_level, effective_settings.log_to_file) {
            tracing::error!(error = %e, "Failed to reinitialize logging");
        } else {
            tracing::info!(
                log_level = %log_level.to_string(),
                log_to_file = effective_settings.log_to_file,
                "Logging reinitialized successfully"
            );
        }

        // If retention days changed, trigger cleanup
        if effective_settings.log_to_file {
            logging::auto_cleanup_old_logs(effective_settings.log_retention_days);
        }
    }

    if let Some(err) = shortcut_error {
        return Err(err);
    }

    Ok(())
}

#[tauri::command]
pub fn get_settings_path() -> Result<String, String> {
    if let Some(config_dir) = dirs::config_dir() {
        let settings_path = config_dir.join("RustFrame").join("settings.json");
        Ok(settings_path.to_string_lossy().to_string())
    } else {
        Err("Could not find config directory".to_string())
    }
}

#[tauri::command]
pub fn open_settings_folder() -> Result<(), String> {
    if let Some(config_dir) = dirs::config_dir() {
        let rustframe_dir = config_dir.join("RustFrame");
        let _ = std::fs::create_dir_all(&rustframe_dir);

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(&rustframe_dir)
                .spawn()
                .map_err(|e| e.to_string())?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&rustframe_dir)
                .spawn()
                .map_err(|e| e.to_string())?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&rustframe_dir)
                .spawn()
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    } else {
        Err("Could not find config directory".to_string())
    }
}

#[tauri::command]
pub fn open_logs_folder() -> Result<(), String> {
    let logs_dir = logging::get_logs_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(&logs_dir);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&logs_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&logs_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&logs_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn clear_old_logs(keep_days: u32) -> Result<usize, String> {
    let logs_dir = logging::get_logs_dir().map_err(|e| e.to_string())?;
    logging::cleanup_old_logs(&logs_dir, keep_days).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_settings(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.settings.lock().unwrap();
    let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn import_settings(
    path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Settings, String> {
    if !shortcuts::SHORTCUTS_ENABLED {
        let _ = app;
    }

    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    let imported: Settings =
        serde_json::from_str(&json).map_err(|e| format!("Invalid settings file: {}", e))?;

    let previous_settings = state.settings.lock().unwrap().clone();
    let mut effective_settings = imported.clone();
    {
        let mut app_settings = state.settings.lock().unwrap();
        *app_settings = effective_settings.clone();
    }

    if shortcuts::SHORTCUTS_ENABLED {
        if let Err(e) = shortcuts::apply_shortcuts(
            &app,
            &effective_settings.shortcuts,
            Some(&previous_settings.shortcuts),
        ) {
            effective_settings.shortcuts = previous_settings.shortcuts.clone();
            let mut app_settings = state.settings.lock().unwrap();
            *app_settings = effective_settings.clone();
            log::warn!("Shortcut registration failed during import: {}", e);
        }
    }

    // Also save to default location (preserve any extra keys in the imported JSON)
    if let Some(config_dir) = dirs::config_dir() {
        let rustframe_dir = config_dir.join("RustFrame");
        let _ = std::fs::create_dir_all(&rustframe_dir);

        let value: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|_| {
            serde_json::to_value(&imported).unwrap_or_else(|_| serde_json::json!({}))
        });
        if let Ok(pretty) = serde_json::to_string_pretty(&value) {
            let _ = std::fs::write(rustframe_dir.join("settings.json"), pretty);
        }
    }

    Ok(effective_settings)
}
