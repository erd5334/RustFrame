use crate::app_state::AppState;
use crate::settings::Settings;
use rustframe_capture::capture::{CaptureEngine, CaptureFrame};
use rustframe_capture::window_filter::WindowIdentifier;

pub trait CapturePlatform: Send + Sync {
    fn cleanup_before_capture_start(&self) -> Result<(), String>;
    fn clear_capture_windows(&self);
    fn create_and_store_hollow_border(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        settings: &Settings,
    ) -> Result<(), String>;
    fn create_and_store_rec_indicator(&self, x: i32, y: i32, width: u32, settings: &Settings);
    fn create_and_store_destination_window(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        settings: &Settings,
    ) -> Result<(), String>;
    fn post_create_destination_window(&self, x: i32, y: i32, width: u32, height: u32);
    fn create_separation_layer_for_capture(&self, x: i32, y: i32, width: u32, height: u32);
    fn create_capture_engine_for_settings(
        &self,
        settings: &Settings,
    ) -> Result<Box<dyn CaptureEngine>, String>;
    fn build_capture_exclusion_list(&self, settings: &Settings) -> Vec<WindowIdentifier>;
    fn configure_preview_window_for_capture(&self, x: i32, y: i32, width: u32, height: u32);
    fn register_border_callbacks(&self, app: tauri::AppHandle, state: &AppState, border_w: i32);
    fn render_frame_to_destination_if_available(
        &self,
        is_interacting: bool,
        frame: CaptureFrame,
        use_gpu: bool,
        capture_clicks_enabled: bool,
        click_color: [u8; 4],
        click_dissolve_ms: u64,
        click_radius: u32,
    );
    fn get_capture_rect(&self) -> Option<(i32, i32, i32, i32)>;
    fn get_capture_inner_rect(&self) -> Option<(i32, i32, i32, i32)>;
    fn cleanup_after_capture_stop(&self);
    fn cleanup_after_capture_failed(&self);
    fn start_click_capture(&self) -> anyhow::Result<()>;
    fn stop_click_capture(&self);
    fn clear_clicks(&self);
}

pub struct RealCapturePlatform;

impl CapturePlatform for RealCapturePlatform {
    fn cleanup_before_capture_start(&self) -> Result<(), String> {
        crate::platform::services::cleanup_before_capture_start()
    }

    fn clear_capture_windows(&self) {
        crate::platform::services::clear_capture_windows();
    }

    fn create_and_store_hollow_border(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        settings: &Settings,
    ) -> Result<(), String> {
        let border = crate::platform::services::create_hollow_border_for_settings(
            x, y, width, height, settings,
        )?;
        crate::platform::services::store_hollow_border(border);
        Ok(())
    }

    fn create_and_store_rec_indicator(&self, x: i32, y: i32, width: u32, settings: &Settings) {
        if let Some(rec) = crate::platform::services::create_rec_indicator_for_settings(
            x, y, width, settings,
        ) {
            crate::platform::services::store_rec_indicator(rec);
        }
    }

    fn create_and_store_destination_window(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        settings: &Settings,
    ) -> Result<(), String> {
        let window = crate::platform::services::create_destination_window_for_settings(
            x, y, width, height, settings,
        )?;
        crate::platform::services::store_destination_window(window);
        Ok(())
    }

    fn post_create_destination_window(&self, x: i32, y: i32, width: u32, height: u32) {
        crate::platform::services::post_create_destination_window(x, y, width, height);
    }

    fn create_separation_layer_for_capture(&self, x: i32, y: i32, width: u32, height: u32) {
        crate::platform::services::create_separation_layer_for_capture(x, y, width, height);
    }

    fn create_capture_engine_for_settings(
        &self,
        settings: &Settings,
    ) -> Result<Box<dyn CaptureEngine>, String> {
        crate::platform::services::create_capture_engine_for_settings(settings)
    }

    fn build_capture_exclusion_list(&self, settings: &Settings) -> Vec<WindowIdentifier> {
        crate::platform::services::build_capture_exclusion_list(settings)
    }

    fn configure_preview_window_for_capture(&self, x: i32, y: i32, width: u32, height: u32) {
        crate::platform::services::configure_preview_window_for_capture(x, y, width, height);
    }

    fn register_border_callbacks(&self, app: tauri::AppHandle, state: &AppState, border_w: i32) {
        crate::platform::services::register_border_callbacks(app, state, border_w);
    }

    fn render_frame_to_destination_if_available(
        &self,
        is_interacting: bool,
        frame: CaptureFrame,
        use_gpu: bool,
        capture_clicks_enabled: bool,
        click_color: [u8; 4],
        click_dissolve_ms: u64,
        click_radius: u32,
    ) {
        crate::platform::services::render_frame_to_destination_if_available(
            is_interacting,
            frame,
            use_gpu,
            capture_clicks_enabled,
            click_color,
            click_dissolve_ms,
            click_radius,
        );
    }

    fn get_capture_rect(&self) -> Option<(i32, i32, i32, i32)> {
        crate::platform::services::get_capture_rect()
    }

    fn get_capture_inner_rect(&self) -> Option<(i32, i32, i32, i32)> {
        crate::platform::services::get_capture_inner_rect()
    }

    fn cleanup_after_capture_stop(&self) {
        crate::platform::services::cleanup_after_capture_stop();
    }

    fn cleanup_after_capture_failed(&self) {
        crate::platform::services::cleanup_after_capture_failed();
    }

    fn start_click_capture(&self) -> anyhow::Result<()> {
        crate::platform::input::start_click_capture()
    }

    fn stop_click_capture(&self) {
        crate::platform::input::stop_click_capture();
    }

    fn clear_clicks(&self) {
        crate::platform::input::clear_clicks();
    }
}
