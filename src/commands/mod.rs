pub mod profiles;
pub mod locales;
pub mod settings;
pub mod system;
pub mod windowing;

pub fn handlers() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        system::is_dev_mode,
        system::get_platform_info,
        system::get_display_scale_factor,
        system::get_recommended_window_size,
        system::get_app_version,
        settings::get_settings,
        windowing::get_border_rect,
        profiles::get_capture_profiles,
        profiles::get_active_capture_profile,
        profiles::get_capture_profile_hints,
        profiles::set_active_capture_profile,
        profiles::get_local_profile_version,
        profiles::update_local_profile_version,
        profiles::check_profile_updates,
        profiles::download_profile,
        profiles::delete_profile,
        profiles::get_profile_details,
        locales::list_locales,
        locales::load_locales,
        locales::get_locales_path,
        locales::open_locales_folder,
        locales::download_locales,
        windowing::get_available_windows,
        settings::save_settings,
        settings::get_settings_path,
        settings::open_settings_folder,
        settings::open_logs_folder,
        settings::clear_old_logs,
        settings::export_settings,
        settings::import_settings,
        super::preview_border::show_preview_border,
        super::preview_border::hide_preview_border,
        super::preview_border::update_preview_border,
        super::preview_border::get_preview_border_rect,
        super::capture_controller::start_capture,
        super::capture_controller::stop_capture,
        super::capture_controller::cleanup_on_capture_failed,
        super::capture_controller::is_capturing,
        system::get_screen_dimensions,
        system::get_monitor_refresh_rate,
        system::get_monitors,
        super::preview_border::update_preview_border_style,
    ]
}
