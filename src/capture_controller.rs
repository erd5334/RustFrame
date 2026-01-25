use std::sync::Arc;
use tauri::State;

use crate::capture_deps::{CapturePlatform, RealCapturePlatform};
use crate::{hollow_border, platform::services, profiles, settings_io, AppState};
use crate::settings::{CaptureMethod, Settings};

use rustframe_capture::capture::CaptureRect;

// Platform-specific engine creation moved to platform::services

#[derive(Clone, Copy)]
struct CaptureStartOptions {
    spawn_render_thread: bool,
    from_shortcut: bool,
}

impl Default for CaptureStartOptions {
    fn default() -> Self {
        Self {
            spawn_render_thread: true,
            from_shortcut: false,
        }
    }
}

fn start_capture_with_platform(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    state: &AppState,
    app: Option<tauri::AppHandle>,
    platform: Arc<dyn CapturePlatform>,
    options: CaptureStartOptions,
) -> Result<(), String> {
    tracing::info!(
        x = x,
        y = y,
        width = width,
        height = height,
        "Starting capture"
    );
    log::info!(
        "Starting capture at ({}, {}) size {}x{}",
        x,
        y,
        width,
        height
    );

    // CRITICAL: Always close BOTH preview border and capture border first
    // PREVIEW_BORDER and HOLLOW_BORDER share global state (HOLLOW_HWND, HOLLOW_RECT, etc.)
    // and must be completely cleaned up before creating new border
    platform.cleanup_before_capture_start()?;

    // Clean up any previous capture session first (always, not just if capturing)

    // Stop capture engine if running
    if let Some(ref mut engine) = *state.capture_engine.lock().unwrap() {
        engine.stop();
    }

    // Stop render thread if running
    *state.render_thread_stop.lock().unwrap() = true;

    // Wait for render thread to finish to avoid dropping windows while it's still rendering
    if let Some(handle) = state.render_thread_handle.lock().unwrap().take() {
        let _ = handle.join();
    }

    // Clean up windows - this will trigger Drop which must be on main thread
    platform.clear_capture_windows();

    // Reset capturing state
    *state.is_capturing.lock().unwrap() = false;

    tracing::debug!("Waiting for cleanup to complete");
    // Give a moment for cleanup
    std::thread::sleep(std::time::Duration::from_millis(100));

    tracing::debug!("Loading settings for capture start");
    log::info!("[MAIN] About to load settings...");

    // Base settings + optional active profile overrides
    tracing::debug!("Acquiring base_settings lock");
    let base_settings = state.settings.lock().unwrap().clone();
    tracing::debug!("Acquiring active_profile lock");
    let active_profile = state.active_profile.lock().unwrap().clone();

    tracing::debug!(active_profile = ?active_profile, "Profile settings loaded");

    let settings = if let (Some(profiles_dir), Some(profile_id)) =
        (settings_io::rustframe_profiles_dir(), active_profile)
    {
        tracing::info!(profile_id = %profile_id, profiles_dir = ?profiles_dir, "Loading capture profile");
        match profiles::read_profile_overrides(&profiles_dir, &profile_id) {
            Some(overrides) => match settings_io::apply_profile_overrides(&base_settings, overrides) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!(
                        "Failed to apply profile '{}': {} (using base settings)",
                        profile_id,
                        e
                    );
                    base_settings
                }
            },
            None => base_settings,
        }
    } else {
        base_settings
    };

    tracing::debug!(
        show_rec_indicator = settings.show_rec_indicator,
        capture_clicks = settings.capture_clicks,
        preview_mode = ?settings.preview_mode,
        "Capture settings loaded"
    );

    // Create hollow border
    log::info!("[MAIN] Creating hollow border...");
    platform.create_and_store_hollow_border(x, y, width, height, &settings)?;
    log::info!("[MAIN] Hollow border created successfully");

    if options.from_shortcut {
        services::prime_capture_border_interaction_from_shortcut();
    }

    // Create REC indicator (separate window with screen sharing excluded)
    platform.create_and_store_rec_indicator(x, y, width, &settings);

    tracing::info!(preview_mode = ?settings.preview_mode, "Creating destination window");

    #[cfg(not(target_os = "windows"))]
    tracing::debug!(
        os = if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Linux"
        },
        "Creating native destination window"
    );

    platform.create_and_store_destination_window(x, y, width, height, &settings)?;
    platform.post_create_destination_window(x, y, width, height);

    // Create separation layer (RegionToShare approach)
    // Positioned between border and preview in z-order
    platform.create_separation_layer_for_capture(x, y, width, height);

    // Start capture engine
    tracing::debug!("Starting capture engine");
    let mut engine_lock = state.capture_engine.lock().unwrap();

    // (Re)create capture engine so users can change capture_method from Settings
    if let Some(ref mut existing) = *engine_lock {
        tracing::debug!("Stopping existing capture engine");
        existing.stop();
    }

    tracing::info!(
        capture_method = ?settings.capture_method,
        "Creating new capture engine"
    );
    *engine_lock = Some(platform.create_capture_engine_for_settings(&settings)?);

    if let Some(ref mut engine) = *engine_lock {
        // Offset capture region inward by border_width to exclude border from capture
        let border_offset = settings.border_width as i32;
        let region = CaptureRect {
            x: x + border_offset,
            y: y + border_offset,
            width: (width as i32 - border_offset * 2).max(1) as u32,
            height: (height as i32 - border_offset * 2).max(1) as u32,
        };

        // Note: Separation layer is already hidden from screen sharing via NSWindowSharingNone
        // No need to explicitly exclude it from capture
        let exclusion_list = platform.build_capture_exclusion_list(&settings);

        log::info!(
            "[MAIN] Calling engine.start() with region: {:?}, excluded: {} items",
            region,
            exclusion_list.len()
        );
        let exclusion_list = exclusion_list.clone();
        let start_result = engine.start(region, settings.show_cursor, Some(exclusion_list.clone()));
        if let Err(e) = start_result {
            tracing::error!(error = %e, "Capture engine start failed");
            if settings.capture_method == CaptureMethod::Wgc {
                tracing::warn!("WGC start failed, falling back to GDI Copy");
                let mut fallback_settings = settings.clone();
                fallback_settings.capture_method = CaptureMethod::GdiCopy;
                *engine_lock = Some(platform.create_capture_engine_for_settings(&fallback_settings)?);
                if let Some(ref mut fallback_engine) = *engine_lock {
                    fallback_engine
                        .start(region, settings.show_cursor, Some(exclusion_list))
                        .map_err(|e| {
                            tracing::error!(error = %e, "GDI fallback start failed");
                            e.to_string()
                        })?;
                } else {
                    return Err("Failed to initialize GDI fallback engine".to_string());
                }
            } else {
                return Err(e.to_string());
            }
        }
        tracing::info!("Capture engine started successfully");
    } else {
        tracing::error!("Capture engine is None after creation");
    }
    drop(engine_lock);

    // Set up cursor filtering: check if preview overlaps capture region
    platform.configure_preview_window_for_capture(x, y, width, height);

    tracing::debug!(
        capture_clicks = settings.capture_clicks,
        "Checking click capture setting"
    );

    // Start click capture if enabled
    if settings.capture_clicks {
        tracing::info!("Starting click capture");
        if let Err(e) = platform.start_click_capture() {
            tracing::error!(error = %e, "Failed to start click capture");
        } else {
            tracing::debug!("Click capture started successfully");
        }
    } else {
        tracing::debug!("Click capture disabled in settings");
    }

    *state.is_capturing.lock().unwrap() = true;
    *state.render_thread_stop.lock().unwrap() = false;

    // Register border callbacks for drag/resize interactions.
    if let Some(app_handle) = app {
        platform.register_border_callbacks(app_handle, state, settings.border_width as i32);
    }

    // Start frame rendering thread
    let engine_clone = state.capture_engine.clone();
    let settings_clone = state.settings.clone(); // Clone settings for GPU check
    let stop_flag = state.render_thread_stop.clone();
    let target_fps = settings.target_fps;
    let capture_clicks_enabled = settings.capture_clicks;
    let click_color = settings.click_highlight_color;
    let click_dissolve_ms = settings.click_dissolve_ms as u64;
    let click_radius = settings.click_highlight_radius;

    if options.spawn_render_thread {
        let platform_for_thread = Arc::clone(&platform);
        let render_handle = std::thread::spawn(move || {
            log::info!("Frame rendering thread started");
            let frame_duration = std::time::Duration::from_millis(1000 / target_fps as u64);

            loop {
                // Check stop flag
                if *stop_flag.lock().unwrap() {
                    break;
                }

                let frame_start = std::time::Instant::now();

                // User requested higher update frequency during drag/resize.
                // We keep capturing during interaction and bump FPS below.
                let is_interacting = hollow_border::is_border_interacting();

                // Get frame from capture engine
                let frame = {
                    let mut engine = engine_clone.lock().unwrap();
                    if let Some(ref mut eng) = *engine {
                        eng.get_frame()
                    } else {
                        None
                    }
                };

                // Render frame to destination window (use try_lock to avoid blocking)
                if let Some(frame) = frame {
                    // Check if GPU acceleration is available and enabled
                    let gpu_enabled = settings_clone.lock().unwrap().gpu_acceleration;
                    let use_gpu = gpu_enabled && frame.gpu_texture.is_some();

                    platform_for_thread.render_frame_to_destination_if_available(
                        is_interacting,
                        frame,
                        use_gpu,
                        capture_clicks_enabled,
                        click_color,
                        click_dissolve_ms,
                        click_radius,
                    );
                } // DESTINATION_WINDOW lock released here

                // Frame rate limiting
                let elapsed = frame_start.elapsed();

                // During border interaction (drag/resize), use faster update rate for Meet sync
                let is_interacting_for_fps = hollow_border::is_border_interacting();

                let min_frame_duration = if is_interacting_for_fps {
                    // 5ms during interaction = ~200 FPS max for smooth Meet updates
                    std::time::Duration::from_millis(5)
                } else {
                    frame_duration
                };

                if elapsed < min_frame_duration {
                    std::thread::sleep(min_frame_duration - elapsed);
                }
            }
        });

        *state.render_thread_handle.lock().unwrap() = Some(render_handle);
    } else {
        *state.render_thread_handle.lock().unwrap() = None;
    }

    log::info!("Capture started successfully");
    Ok(())
}

fn stop_capture_with_platform(
    state: &AppState,
    platform: Arc<dyn CapturePlatform>,
) -> Result<Settings, String> {
    tracing::info!("Stopping capture");
    log::info!("Stopping capture");

    // Save last region if remember_last_region is enabled
    let remember_last_region = state.settings.lock().unwrap().remember_last_region;
    if remember_last_region {
        if let Some(rect) = platform.get_capture_rect() {
            log::info!(
                "Read border position: x={}, y={}, w={}, h={}",
                rect.0,
                rect.1,
                rect.2,
                rect.3
            );

            let updated_settings = {
                let mut settings_guard = state.settings.lock().unwrap();
                settings_guard.last_region = Some([rect.0, rect.1, rect.2, rect.3]);
                settings_guard.clone()
            };

            if let Err(e) = settings_io::persist_settings_to_disk(&updated_settings) {
                log::error!("Failed to persist last_region: {}", e);
            } else {
                log::info!("Successfully saved last_region to disk");
            }
        }
    }

    // Signal render thread to stop
    *state.render_thread_stop.lock().unwrap() = true;

    // Join render thread to ensure it isn't still touching NSWindow-backed objects.
    if let Some(handle) = state.render_thread_handle.lock().unwrap().take() {
        let _ = handle.join();
    }

    // Stop capture engine
    let mut engine_lock = state.capture_engine.lock().unwrap();
    if let Some(ref mut engine) = *engine_lock {
        engine.stop();
        log::info!("Capture engine stopped");
    }
    drop(engine_lock);

    platform.cleanup_after_capture_stop();

    // Stop mouse hook completely and clear click capture data
    platform.stop_click_capture();
    platform.clear_clicks();

    *state.is_capturing.lock().unwrap() = false;

    // Return updated settings so frontend can sync
    let final_settings = state.settings.lock().unwrap().clone();
    log::info!("Capture stopped successfully");
    Ok(final_settings)
}

fn cleanup_on_capture_failed_with_platform(
    state: &AppState,
    platform: Arc<dyn CapturePlatform>,
) -> Result<(), String> {
    tracing::error!("Cleaning up after capture start failure");
    log::error!("Cleaning up after capture start failure");

    // Stop any capture engine that might have started
    let mut engine_lock = state.capture_engine.lock().unwrap();
    if let Some(ref mut engine) = *engine_lock {
        engine.stop();
    }
    drop(engine_lock);

    platform.cleanup_after_capture_failed();

    // Clear click capture data
    platform.clear_clicks();

    // Ensure capturing state is false
    *state.is_capturing.lock().unwrap() = false;

    log::info!("Cleanup completed after capture failure");
    Ok(())
}

#[tauri::command]
pub async fn start_capture(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    from_shortcut: Option<bool>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let platform: Arc<dyn CapturePlatform> = Arc::new(RealCapturePlatform);
    let options = CaptureStartOptions {
        from_shortcut: from_shortcut.unwrap_or(false),
        ..CaptureStartOptions::default()
    };
    start_capture_with_platform(
        x,
        y,
        width,
        height,
        state.inner(),
        Some(app),
        platform,
        options,
    )
}

#[tauri::command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<Settings, String> {
    let platform: Arc<dyn CapturePlatform> = Arc::new(RealCapturePlatform);
    stop_capture_with_platform(state.inner(), platform)
}

/// Cleanup borders and windows when capture fails to start
/// This is called from frontend when start_capture returns an error
#[tauri::command]
pub async fn cleanup_on_capture_failed(state: State<'_, AppState>) -> Result<(), String> {
    let platform: Arc<dyn CapturePlatform> = Arc::new(RealCapturePlatform);
    cleanup_on_capture_failed_with_platform(state.inner(), platform)
}

#[tauri::command]
pub async fn is_capturing(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.is_capturing.lock().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustframe_capture::capture::{CaptureEngine, CaptureFrame, CaptureRect};
    use rustframe_capture::window_filter::WindowIdentifier;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    };

    #[derive(Default)]
    struct MockEngineState {
        start_region: Mutex<Option<CaptureRect>>,
        start_show_cursor: AtomicBool,
        stop_calls: AtomicUsize,
    }

    struct MockCaptureEngine {
        state: Arc<MockEngineState>,
    }

    impl CaptureEngine for MockCaptureEngine {
        fn start(
            &mut self,
            region: CaptureRect,
            show_cursor: bool,
            _excluded_windows: Option<Vec<WindowIdentifier>>,
        ) -> anyhow::Result<()> {
            *self.state.start_region.lock().unwrap() = Some(region);
            self.state.start_show_cursor.store(show_cursor, Ordering::SeqCst);
            Ok(())
        }

        fn stop(&mut self) {
            self.state.stop_calls.fetch_add(1, Ordering::SeqCst);
        }

        fn is_active(&self) -> bool {
            false
        }

        fn has_new_frame(&self) -> bool {
            false
        }

        fn get_frame(&mut self) -> Option<CaptureFrame> {
            None
        }

        fn set_cursor_visible(&mut self, _visible: bool) -> anyhow::Result<()> {
            Ok(())
        }

        fn get_region(&self) -> Option<CaptureRect> {
            self.state.start_region.lock().unwrap().clone()
        }

        fn update_region(&mut self, _region: CaptureRect) -> anyhow::Result<()> {
            Ok(())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[derive(Default)]
    struct MockCapturePlatform {
        calls: Mutex<Vec<&'static str>>,
        engine_state: Arc<MockEngineState>,
    }

    impl MockCapturePlatform {
        fn record(&self, name: &'static str) {
            self.calls.lock().unwrap().push(name);
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl CapturePlatform for MockCapturePlatform {
        fn cleanup_before_capture_start(&self) -> Result<(), String> {
            self.record("cleanup_before_capture_start");
            Ok(())
        }

        fn clear_capture_windows(&self) {
            self.record("clear_capture_windows");
        }

        fn create_and_store_hollow_border(
            &self,
            _x: i32,
            _y: i32,
            _width: u32,
            _height: u32,
            _settings: &Settings,
        ) -> Result<(), String> {
            self.record("create_and_store_hollow_border");
            Ok(())
        }

        fn create_and_store_rec_indicator(
            &self,
            _x: i32,
            _y: i32,
            _width: u32,
            _settings: &Settings,
        ) {
            self.record("create_and_store_rec_indicator");
        }

        fn create_and_store_destination_window(
            &self,
            _x: i32,
            _y: i32,
            _width: u32,
            _height: u32,
            _settings: &Settings,
        ) -> Result<(), String> {
            self.record("create_and_store_destination_window");
            Ok(())
        }

        fn post_create_destination_window(
            &self,
            _x: i32,
            _y: i32,
            _width: u32,
            _height: u32,
        ) {
            self.record("post_create_destination_window");
        }

        fn create_separation_layer_for_capture(
            &self,
            _x: i32,
            _y: i32,
            _width: u32,
            _height: u32,
        ) {
            self.record("create_separation_layer_for_capture");
        }

        fn create_capture_engine_for_settings(
            &self,
            _settings: &Settings,
        ) -> Result<Box<dyn CaptureEngine>, String> {
            self.record("create_capture_engine_for_settings");
            Ok(Box::new(MockCaptureEngine {
                state: Arc::clone(&self.engine_state),
            }))
        }

        fn build_capture_exclusion_list(&self, _settings: &Settings) -> Vec<WindowIdentifier> {
            self.record("build_capture_exclusion_list");
            Vec::new()
        }

        fn configure_preview_window_for_capture(
            &self,
            _x: i32,
            _y: i32,
            _width: u32,
            _height: u32,
        ) {
            self.record("configure_preview_window_for_capture");
        }

        fn register_border_callbacks(
            &self,
            _app: tauri::AppHandle,
            _state: &AppState,
            _border_w: i32,
        ) {
            self.record("register_border_callbacks");
        }

        fn render_frame_to_destination_if_available(
            &self,
            _is_interacting: bool,
            _frame: CaptureFrame,
            _use_gpu: bool,
            _capture_clicks_enabled: bool,
            _click_color: [u8; 4],
            _click_dissolve_ms: u64,
            _click_radius: u32,
        ) {
        }

        fn get_capture_rect(&self) -> Option<(i32, i32, i32, i32)> {
            self.record("get_capture_rect");
            None
        }

        fn get_capture_inner_rect(&self) -> Option<(i32, i32, i32, i32)> {
            self.record("get_capture_inner_rect");
            None
        }

        fn cleanup_after_capture_stop(&self) {
            self.record("cleanup_after_capture_stop");
        }

        fn cleanup_after_capture_failed(&self) {
            self.record("cleanup_after_capture_failed");
        }

        fn start_click_capture(&self) -> anyhow::Result<()> {
            self.record("start_click_capture");
            Ok(())
        }

        fn stop_click_capture(&self) {
            self.record("stop_click_capture");
        }

        fn clear_clicks(&self) {
            self.record("clear_clicks");
        }
    }

    fn build_test_state(settings: Settings) -> AppState {
        AppState {
            capture_engine: Arc::new(Mutex::new(None)),
            settings: Arc::new(Mutex::new(settings)),
            active_profile: Arc::new(Mutex::new(None)),
            is_capturing: Arc::new(Mutex::new(false)),
            settings_modal_open: Arc::new(Mutex::new(false)),
            render_thread_stop: Arc::new(Mutex::new(false)),
            render_thread_handle: Arc::new(Mutex::new(None)),
            monitors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn set_existing_engine(state: &AppState, engine_state: Arc<MockEngineState>) {
        let engine = MockCaptureEngine { state: engine_state };
        *state.capture_engine.lock().unwrap() = Some(Box::new(engine));
    }

    fn call_index(calls: &[&'static str], name: &str) -> Option<usize> {
        calls.iter().position(|entry| *entry == name)
    }

    #[test]
    fn start_capture_uses_platform_services() {
        let mut settings = Settings::default();
        settings.capture_clicks = true;
        settings.border_width = 2;
        settings.remember_last_region = false;

        let state = build_test_state(settings);
        let platform = Arc::new(MockCapturePlatform::default());
        let platform_dyn: Arc<dyn CapturePlatform> = platform.clone();

        let options = CaptureStartOptions {
            spawn_render_thread: false,
            from_shortcut: false,
        };

        let result = start_capture_with_platform(
            10,
            20,
            200,
            100,
            &state,
            None,
            platform_dyn,
            options,
        );

        assert!(result.is_ok());

        let calls = platform.calls();
        assert!(calls.contains(&"cleanup_before_capture_start"));
        assert!(calls.contains(&"create_and_store_hollow_border"));
        assert!(calls.contains(&"create_capture_engine_for_settings"));
        assert!(calls.contains(&"start_click_capture"));

        let cleanup_index = call_index(&calls, "cleanup_before_capture_start").unwrap();
        let border_index = call_index(&calls, "create_and_store_hollow_border").unwrap();
        let engine_index = call_index(&calls, "create_capture_engine_for_settings").unwrap();
        assert!(cleanup_index < border_index);
        assert!(border_index < engine_index);

        let region = platform
            .engine_state
            .start_region
            .lock()
            .unwrap()
            .expect("engine start region missing");
        assert_eq!(region.x, 12);
        assert_eq!(region.y, 22);
        assert_eq!(region.width, 196);
        assert_eq!(region.height, 96);
    }

    #[test]
    fn stop_capture_invokes_cleanup_paths() {
        let mut settings = Settings::default();
        settings.remember_last_region = false;

        let state = build_test_state(settings);
        *state.is_capturing.lock().unwrap() = true;

        let platform = Arc::new(MockCapturePlatform::default());
        let platform_dyn: Arc<dyn CapturePlatform> = platform.clone();

        let result = stop_capture_with_platform(&state, platform_dyn);
        assert!(result.is_ok());

        let calls = platform.calls();
        assert!(calls.contains(&"cleanup_after_capture_stop"));
        assert!(calls.contains(&"stop_click_capture"));
        assert!(calls.contains(&"clear_clicks"));
        assert!(!calls.contains(&"get_capture_rect"));
        assert!(!*state.is_capturing.lock().unwrap());
    }

    #[test]
    fn start_capture_skips_click_capture_when_disabled() {
        let mut settings = Settings::default();
        settings.capture_clicks = false;
        settings.border_width = 4;

        let state = build_test_state(settings);
        let platform = Arc::new(MockCapturePlatform::default());
        let platform_dyn: Arc<dyn CapturePlatform> = platform.clone();

        let options = CaptureStartOptions {
            spawn_render_thread: false,
            from_shortcut: false,
        };

        let result = start_capture_with_platform(
            0,
            0,
            300,
            200,
            &state,
            None,
            platform_dyn,
            options,
        );

        assert!(result.is_ok());
        let calls = platform.calls();
        assert!(!calls.contains(&"start_click_capture"));
        assert!(*state.is_capturing.lock().unwrap());
        assert!(!*state.render_thread_stop.lock().unwrap());
    }

    #[test]
    fn start_capture_stops_existing_engine() {
        let mut settings = Settings::default();
        settings.capture_clicks = true;
        settings.border_width = 1;

        let state = build_test_state(settings);
        let existing_engine_state = Arc::new(MockEngineState::default());
        set_existing_engine(&state, Arc::clone(&existing_engine_state));

        let platform = Arc::new(MockCapturePlatform::default());
        let platform_dyn: Arc<dyn CapturePlatform> = platform.clone();

        let options = CaptureStartOptions {
            spawn_render_thread: false,
            from_shortcut: false,
        };

        let result = start_capture_with_platform(
            5,
            6,
            100,
            80,
            &state,
            None,
            platform_dyn,
            options,
        );

        assert!(result.is_ok());
        // start_capture stops the previous engine once before cleanup and once before recreate.
        assert_eq!(existing_engine_state.stop_calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn cleanup_on_capture_failed_clears_state() {
        let settings = Settings::default();
        let state = build_test_state(settings);
        *state.is_capturing.lock().unwrap() = true;

        let platform = Arc::new(MockCapturePlatform::default());
        let platform_dyn: Arc<dyn CapturePlatform> = platform.clone();

        let result = cleanup_on_capture_failed_with_platform(&state, platform_dyn);
        assert!(result.is_ok());

        let calls = platform.calls();
        assert!(calls.contains(&"cleanup_after_capture_failed"));
        assert!(calls.contains(&"clear_clicks"));
        assert!(!*state.is_capturing.lock().unwrap());
    }
}
