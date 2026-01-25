// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unexpected_cfgs)]

use tauri::Manager;

// Import modules
#[macro_use]
mod capture_controller;
mod destination_window;
mod display_info;
mod hollow_border;
mod logging;
mod commands;
mod app_bootstrap;
mod capture_deps;
mod app_state;
mod app_lifecycle;
mod monitors;
mod platform;
mod platform_info;
#[macro_use]
mod preview_border;
mod profiles;
mod rec_indicator;
mod separation_layer;
mod settings;
mod settings_io;
mod shortcuts;
mod single_instance;
mod traits; // Cross-platform trait definitions

pub(crate) use app_state::AppState;


// ============================================================================
// Main
// ============================================================================

fn main() {
    // Single instance, logging, panic hook (display info initialized in setup)
    app_bootstrap::acquire_single_instance_or_exit();
    let initial_settings = app_bootstrap::load_initial_settings_for_logging();
    app_bootstrap::init_logging_and_display(&initial_settings);
    app_bootstrap::install_panic_hook();

    // Load settings + active profile (we already loaded them above for logging, reuse them)
    let settings = initial_settings;
    let active_profile = app_bootstrap::load_active_profile();
    app_bootstrap::log_active_settings(&settings, &active_profile);
    let app_state = app_bootstrap::build_app_state(settings, active_profile);

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init());

    let builder = if shortcuts::SHORTCUTS_ENABLED {
        builder.plugin(tauri_plugin_global_shortcut::Builder::new().build())
    } else {
        builder
    };

    builder
        .setup(|app| {
            app_bootstrap::init_display_info();
            if shortcuts::SHORTCUTS_ENABLED {
                let app_handle = app.handle();
                let state = app_handle.state::<AppState>();
                let shortcuts = state.settings.lock().unwrap().shortcuts.clone();
                if let Err(e) = shortcuts::apply_shortcuts(&app_handle, &shortcuts, None) {
                    log::warn!("Failed to register shortcuts: {}", e);
                }
            }
            Ok(())
        })
        .manage(app_state.clone())
        .invoke_handler(commands::handlers())
        .on_window_event(move |window, event| {
            app_lifecycle::handle_window_event(&app_state, window, event);
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Final cleanup when app exits normally
    log::info!("Application exiting normally, performing final cleanup...");
    app_bootstrap::perform_cleanup();
}
