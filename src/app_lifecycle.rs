use crate::{app_bootstrap, platform, AppState};

pub(crate) fn handle_window_event(
    app_state: &AppState,
    window: &tauri::Window,
    event: &tauri::WindowEvent,
) {
    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            log::info!("Window close requested, performing cleanup...");

            if *app_state.is_capturing.lock().unwrap() {
                log::info!("Capture active, preventing window close");
                api.prevent_close();
                let _ = window.minimize();
                return;
            }

            // Immediately hide/destroy hollow border to unblock its message loop
            platform::services::clear_hollow_border_for_shutdown();

            // Stop capture if running
            if *app_state.is_capturing.lock().unwrap() {
                log::info!("Capture is running, stopping before close...");

                // Signal render thread to stop
                *app_state.render_thread_stop.lock().unwrap() = true;

                // Join render thread to avoid dropping windows while it's rendering
                if let Some(handle) = app_state.render_thread_handle.lock().unwrap().take() {
                    let _ = handle.join();
                }

                // Stop capture engine
                if let Ok(mut engine_lock) = app_state.capture_engine.lock() {
                    if let Some(ref mut engine) = *engine_lock {
                        engine.stop();
                        log::info!("Capture engine stopped");
                    }
                }

                *app_state.is_capturing.lock().unwrap() = false;
            }

            // Perform global cleanup
            app_bootstrap::perform_cleanup();

            // Allow the window to close
            // api.prevent_close(); // Uncomment if you want to prevent close
        }
        tauri::WindowEvent::Destroyed => {
            log::info!("Window destroyed, ensuring cleanup...");
            app_bootstrap::perform_cleanup();
        }
        _ => {}
    }
}
