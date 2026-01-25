use crate::monitors::MonitorInfo;
use crate::AppState;

pub fn get_tauri_monitors(
    window: tauri::Window,
    state: &AppState,
) -> Result<Vec<MonitorInfo>, String> {
    match window.available_monitors() {
        Ok(monitors) => {
            let mut result = Vec::new();
            for (idx, m) in monitors.into_iter().enumerate() {
                let scale_factor = m.scale_factor();
                let size = m.size().to_logical::<u32>(scale_factor);
                let position = m.position().to_logical::<i32>(scale_factor);
                let name = m
                    .name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Display {}", idx + 1));

                result.push(MonitorInfo {
                    id: idx,
                    name,
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                    scale_factor,
                    is_primary: position.x == 0 && position.y == 0, // Heuristic: (0,0) is usually primary
                    refresh_rate: 60, // Tauri doesn't always provide this, default to 60
                });
            }

            // Update the global monitors state
            *state.monitors.lock().unwrap() = result.clone();

            Ok(result)
        }
        Err(e) => Err(format!("Failed to list monitors: {}", e)),
    }
}
