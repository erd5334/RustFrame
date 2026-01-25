use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::{display_info, logging, platform, profiles, settings_io, single_instance};
use crate::settings::Settings;
use crate::AppState;
use rustframe_capture::capture::create_capture_engine;

lazy_static! {
    // Global flag to track if cleanup has been performed
    static ref CLEANUP_PERFORMED: AtomicBool = AtomicBool::new(false);
    // Single instance lock - prevents multiple instances from running
    static ref SINGLE_INSTANCE_LOCK: Mutex<Option<single_instance::SingleInstanceLock>> =
        Mutex::new(None);
}

/// Acquire the single instance lock or exit if another instance is running.
pub(crate) fn acquire_single_instance_or_exit() {
    let instance_lock = match single_instance::SingleInstanceLock::acquire() {
        Ok(lock) => lock,
        Err(_e) => {
            eprintln!("RustFrame is already running!");
            eprintln!("Attempting to activate existing window...");

            // Try to bring the existing window to foreground
            single_instance::SingleInstanceLock::activate_existing_instance();

            std::process::exit(1);
        }
    };

    // Store the lock in global state so it's held for the entire application lifetime
    *SINGLE_INSTANCE_LOCK.lock().unwrap() = Some(instance_lock);
}

/// Load settings early to get log level configuration.
pub(crate) fn load_initial_settings_for_logging() -> Settings {
    if let Some(dir) = settings_io::rustframe_config_dir() {
        profiles::bootstrap_profiles_if_missing(&dir);
        settings_io::load_settings_and_profile_from_disk(&dir).0
    } else {
        Settings::default()
    }
}

/// Initialize logging system.
pub(crate) fn init_logging_and_display(initial_settings: &Settings) {
    let log_level = initial_settings
        .log_level
        .parse::<logging::LogLevel>()
        .unwrap_or(logging::LogLevel::Error);

    if let Err(e) = logging::init_logging(log_level, initial_settings.log_to_file) {
        eprintln!("Failed to initialize logging: {}", e);
    } else {
        // Log startup header with visual markers for easy identification
        tracing::info!("***********************************************************************");
        tracing::info!("*                        RUSTFRAME STARTUP                            *");
        tracing::info!("***********************************************************************");
        tracing::info!(
            version = env!("CARGO_PKG_VERSION"),
            platform = std::env::consts::OS,
            log_level = %log_level.to_string(),
            "Application started"
        );
        tracing::info!("***********************************************************************");

        // Log system information for debugging
        tracing::debug!("");
        tracing::debug!("=== SYSTEM INFORMATION ===");
        tracing::debug!(
            os = std::env::consts::OS,
            arch = std::env::consts::ARCH,
            "Platform details"
        );
    }

    // Auto-cleanup old logs in background
    if initial_settings.log_to_file {
        logging::auto_cleanup_old_logs(initial_settings.log_retention_days);
    }
}

/// Initialize display information after the app runtime is ready.
pub(crate) fn init_display_info() {
    if display_info::is_initialized() {
        return;
    }

    if let Err(e) = display_info::initialize() {
        tracing::warn!(error = %e, "Failed to initialize display info");
        eprintln!("Warning: Failed to initialize display info: {}", e);
    } else {
        tracing::debug!("Display info initialized successfully");

        // Log display configuration for debugging
        tracing::debug!("");
        tracing::debug!("=== DISPLAY CONFIGURATION ===");
        let display_config = display_info::get();
        tracing::debug!(
            scale_factor = display_config.scale_factor,
            width_points = display_config.width_points,
            height_points = display_config.height_points,
            width_pixels = display_config.width_pixels,
            height_pixels = display_config.height_pixels,
            "Display details"
        );
    }
}

/// Set up panic hook for cleanup on crash.
pub(crate) fn install_panic_hook() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(
            ?panic_info,
            "Application panic detected! Performing emergency cleanup"
        );
        log::error!("Application panic detected! Performing emergency cleanup...");
        perform_cleanup();
        default_panic(panic_info);
    }));
}

/// Load the active profile (separately from initial settings load).
pub(crate) fn load_active_profile() -> Option<String> {
    if let Some(dir) = settings_io::rustframe_config_dir() {
        settings_io::load_settings_and_profile_from_disk(&dir).1
    } else {
        None
    }
}

/// Log active settings for debugging.
pub(crate) fn log_active_settings(settings: &Settings, active_profile: &Option<String>) {
    tracing::debug!("");
    tracing::debug!("=== ACTIVE SETTINGS ===");
    tracing::debug!(
        capture_method = ?settings.capture_method,
        target_fps = settings.target_fps,
        show_cursor = settings.show_cursor,
        show_border = settings.show_border,
        border_width = settings.border_width,
        border_color = ?settings.border_color,
        show_rec_indicator = settings.show_rec_indicator,
        rec_indicator_size = ?settings.rec_indicator_size,
        remember_last_region = settings.remember_last_region,
        active_profile = ?active_profile,
        log_level = ?settings.log_level,
        log_to_file = settings.log_to_file,
        log_retention_days = settings.log_retention_days,
        "Settings configuration"
    );
    tracing::debug!("");
    tracing::debug!("***********************************************************************");
    tracing::debug!("*                   INITIALIZATION COMPLETE                           *");
    tracing::debug!("***********************************************************************");
    tracing::debug!("");
}

/// Initialize capture engine and app state.
pub(crate) fn build_app_state(
    settings: Settings,
    active_profile: Option<String>,
) -> AppState {
    tracing::info!(
        capture_method = %settings.capture_method.to_string(),
        "Initializing capture engine"
    );

    let capture_engine = platform::services::create_capture_engine_for_settings(&settings)
        .or_else(|e| {
            tracing::warn!(error = %e, "Failed to create capture engine with settings, using default");
            create_capture_engine().map_err(|e| e.to_string())
        })
        .expect("Failed to initialize capture engine");

    tracing::debug!("Capture engine created successfully");

    AppState {
        capture_engine: Arc::new(Mutex::new(Some(capture_engine))),
        settings: Arc::new(Mutex::new(settings)),
        active_profile: Arc::new(Mutex::new(active_profile)),
        is_capturing: Arc::new(Mutex::new(false)),
        settings_modal_open: Arc::new(Mutex::new(false)),
        render_thread_stop: Arc::new(Mutex::new(false)),
        render_thread_handle: Arc::new(Mutex::new(None)),
        monitors: Arc::new(Mutex::new(Vec::new())),
    }
}

/// Perform cleanup of all capture resources.
/// This function is safe to call multiple times - it will only execute once.
pub(crate) fn perform_cleanup() {
    // Check if cleanup has already been performed
    if CLEANUP_PERFORMED.swap(true, Ordering::SeqCst) {
        tracing::debug!("Cleanup already performed, skipping");
        return;
    }

    tracing::info!("Performing cleanup of all capture resources");

    // Stop mouse hook first (before destroying windows)
    platform::input::stop_click_capture();
    tracing::debug!("Mouse hook stopped");

    platform::services::drop_capture_windows_in_background();

    // Clear click capture data
    platform::input::clear_clicks();
    tracing::debug!("Click capture data cleared");

    // Release single instance lock
    if let Ok(mut lock) = SINGLE_INSTANCE_LOCK.try_lock() {
        if lock.is_some() {
            *lock = None;
            tracing::debug!("Single instance lock released");
        }
    }

    tracing::info!("Cleanup completed successfully");
}

/// Reset cleanup flag (for testing or restart scenarios).
#[allow(dead_code)]
pub(crate) fn reset_cleanup_flag() {
    CLEANUP_PERFORMED.store(false, Ordering::SeqCst);
}
