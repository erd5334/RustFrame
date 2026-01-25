use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::settings::Settings;
use rustframe_capture::config;

pub fn rustframe_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("RustFrame"))
}

pub fn rustframe_profiles_dir() -> Option<PathBuf> {
    rustframe_config_dir().map(|d| d.join("Profiles"))
}

pub fn get_os_profile_subdir() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

fn merge_json(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_obj), Value::Object(overlay_obj)) => {
            for (k, v) in overlay_obj {
                match base_obj.get_mut(&k) {
                    Some(existing) => merge_json(existing, v),
                    None => {
                        base_obj.insert(k, v);
                    }
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value;
        }
    }
}

pub fn sanitize_settings_json_for_platform(value: &mut Value) {
    let Value::Object(obj) = value else {
        return;
    };

    // Migrate legacy/bundled formats into the current Settings schema.
    // - border_color: u32 (ARGB) -> [r,g,b,a]
    // - capture_method: "auto" or an unsupported variant -> remove (let defaults apply)
    // - preview_mode: variant not available on this platform -> remove (let defaults apply)

    if let Some(border_color) = obj.get("border_color").and_then(|v| v.as_u64()) {
        if border_color <= u32::MAX as u64 {
            let rgba = config::colors::argb_to_rgba(border_color as u32);
            obj.insert("border_color".to_string(), serde_json::json!(rgba));
        }
    }

    if let Some(cm) = obj.get("capture_method").and_then(|v| v.as_str()) {
        let invalid_for_platform = cm == "auto" || {
            #[cfg(target_os = "windows")]
            {
                cm == "CoreGraphics"
            }
            #[cfg(not(target_os = "windows"))]
            {
                cm == "Wgc" || cm == "GdiCopy"
            }
        };

        if invalid_for_platform {
            obj.remove("capture_method");
        }
    }

    #[cfg(not(target_os = "windows"))]
    if let Some(pm) = obj.get("preview_mode").and_then(|v| v.as_str()) {
        if pm == "WinApiGdi" {
            obj.remove("preview_mode");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if obj.get("capture_preview_window").and_then(|v| v.as_bool()) != Some(true) {
            obj.insert(
                "capture_preview_window".to_string(),
                Value::Bool(true),
            );
        }
    }

    // Normalize window_filter section
    if let Some(window_filter) = obj.get_mut("window_filter").and_then(|v| v.as_object_mut()) {
        // Force auto_exclude_preview to true (checkbox removed in UI)
        window_filter.insert(
            "auto_exclude_preview".to_string(),
            Value::Bool(true),
        );

        // Normalize mode to snake_case expected by serde
        if let Some(mode_val) = window_filter.get("mode") {
            if let Some(mode_str) = mode_val.as_str() {
                let normalized_owned = mode_str.to_lowercase();
                let normalized = match normalized_owned.as_str() {
                    "none" => "none",
                    "exclude" | "exclude_list" => "exclude_list",
                    "include" | "include_only" => "include_only",
                    other => other,
                };
                window_filter.insert(
                    "mode".to_string(),
                    Value::String(normalized.to_string()),
                );
            }
        }

        // Ensure included_windows exists for include-only flow
        if !window_filter.contains_key("included_windows") {
            window_filter.insert(
                "included_windows".to_string(),
                Value::Array(vec![]),
            );
        }
    }

    // Normalize shortcut keys (legacy "+Plus" token -> "Equal")
    if let Some(shortcuts) = obj.get_mut("shortcuts").and_then(|v| v.as_object_mut()) {
        for key in ["start_capture", "stop_capture", "zoom_in", "zoom_out"] {
            if let Some(Value::String(raw)) = shortcuts.get(key) {
                let tokens = raw
                    .split('+')
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                    .map(|t| {
                        if t.eq_ignore_ascii_case("plus") || t == "+" {
                            "Equal".to_string()
                        } else {
                            t.to_string()
                        }
                    })
                    .collect::<Vec<_>>();

                if !tokens.is_empty() {
                    let normalized = tokens.join("+");
                    shortcuts.insert(key.to_string(), Value::String(normalized));
                }
            }
        }
    }
}

fn bundled_platform_default_settings_json() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/rustframe_default_settings.json"))
}

fn load_bundled_default_overrides() -> Value {
    serde_json::from_str::<Value>(bundled_platform_default_settings_json())
        .unwrap_or_else(|_| serde_json::json!({}))
}

pub fn bootstrap_settings_if_missing(config_dir: &Path) {
    let settings_path = config_dir.join("settings.json");
    if settings_path.exists() {
        return;
    }

    // Create a fully-populated, platform-aware settings.json.
    let mut merged =
        serde_json::to_value(Settings::default()).unwrap_or_else(|_| serde_json::json!({}));
    merge_json(&mut merged, load_bundled_default_overrides());
    let mut merged_obj = merged;
    sanitize_settings_json_for_platform(&mut merged_obj);

    let settings: Settings = serde_json::from_value(merged_obj).unwrap_or_default();
    if let Err(e) = persist_settings_to_disk(&settings) {
        log::warn!("Failed to bootstrap settings.json: {}", e);
    }
}

pub fn persist_settings_to_disk(settings: &Settings) -> Result<(), String> {
    let Some(rustframe_dir) = rustframe_config_dir() else {
        return Err("Could not find config directory".to_string());
    };
    let _ = std::fs::create_dir_all(&rustframe_dir);
    let settings_path = rustframe_dir.join("settings.json");

    fn merge_json_preserve_hidden(
        base: &mut Value,
        overlay: Value,
        hidden_keys: &[&str],
    ) {
        match (base, overlay) {
            (Value::Object(base_obj), Value::Object(overlay_obj)) => {
                for (k, v) in overlay_obj {
                    if hidden_keys.contains(&k.as_str()) && v.is_null() {
                        continue;
                    }

                    match base_obj.get_mut(&k) {
                        Some(existing) => merge_json_preserve_hidden(existing, v, hidden_keys),
                        None => {
                            base_obj.insert(k, v);
                        }
                    }
                }
            }
            (base_slot, overlay_value) => {
                *base_slot = overlay_value;
            }
        }
    }

    let mut existing_value: Value = match std::fs::read_to_string(&settings_path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };

    let new_value = serde_json::to_value(settings).map_err(|e| e.to_string())?;

    let hidden_keys = ["debug_allow_screen_capture"];
    merge_json_preserve_hidden(&mut existing_value, new_value, &hidden_keys);

    let pretty = serde_json::to_string_pretty(&existing_value).map_err(|e| e.to_string())?;
    std::fs::write(settings_path, pretty).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_converts_border_color_u32() {
        let mut value = json!({
            "border_color": 0xFF112233u32
        });
        sanitize_settings_json_for_platform(&mut value);
        assert_eq!(value["border_color"], json!([17, 34, 51, 255]));
    }

    #[test]
    fn sanitize_removes_auto_capture_method() {
        let mut value = json!({
            "capture_method": "auto"
        });
        sanitize_settings_json_for_platform(&mut value);
        assert!(value.get("capture_method").is_none());
    }

    #[test]
    fn sanitize_normalizes_window_filter_mode_and_preview_exclusion() {
        let mut value = json!({
            "window_filter": {
                "mode": "Exclude",
                "auto_exclude_preview": false
            }
        });
        sanitize_settings_json_for_platform(&mut value);
        assert_eq!(value["window_filter"]["mode"], json!("exclude_list"));
        assert_eq!(value["window_filter"]["auto_exclude_preview"], json!(true));
        assert!(value["window_filter"]["included_windows"].is_array());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn sanitize_removes_winapi_preview_mode_on_non_windows() {
        let mut value = json!({
            "preview_mode": "WinApiGdi"
        });
        sanitize_settings_json_for_platform(&mut value);
        assert!(value.get("preview_mode").is_none());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn sanitize_keeps_winapi_preview_mode_on_windows() {
        let mut value = json!({
            "preview_mode": "WinApiGdi"
        });
        sanitize_settings_json_for_platform(&mut value);
        assert_eq!(value["preview_mode"], json!("WinApiGdi"));
    }

    #[test]
    fn sanitize_rewrites_plus_shortcut_token() {
        let mut value = json!({
            "shortcuts": {
                "zoom_in": "CmdOrCtrl+Shift+Plus"
            }
        });
        sanitize_settings_json_for_platform(&mut value);
        assert_eq!(value["shortcuts"]["zoom_in"], json!("CmdOrCtrl+Shift+Equal"));
    }
}

pub fn load_settings_and_profile_from_disk(dir: &Path) -> (Settings, Option<String>) {
    let _ = std::fs::create_dir_all(dir);

    // First-run bootstrap: seed defaults only if missing.
    bootstrap_settings_if_missing(dir);

    let settings_path = dir.join("settings.json");

    let raw = std::fs::read_to_string(&settings_path).unwrap_or_else(|_| "{}".to_string());
    let mut value: Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));

    let active_profile = value
        .get("active_profile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    sanitize_settings_json_for_platform(&mut value);

    // Merge onto current defaults so missing keys don't break deserialization.
    let mut merged =
        serde_json::to_value(Settings::default()).unwrap_or_else(|_| serde_json::json!({}));
    merge_json(&mut merged, value);

    let settings: Settings = serde_json::from_value(merged).unwrap_or_default();

    // Ensure there is always a normalized, fully-populated settings.json on disk.
    // This prevents cases where stop_capture only writes last_region into an otherwise incomplete file.
    if let Err(e) = persist_settings_to_disk(&settings) {
        log::warn!("Failed to persist normalized settings: {}", e);
    }

    (settings, active_profile)
}

pub fn apply_profile_overrides(base: &Settings, overrides: Value) -> Result<Settings, String> {
    let mut merged = serde_json::to_value(base).map_err(|e| e.to_string())?;
    merge_json(&mut merged, overrides);
    serde_json::from_value::<Settings>(merged)
        .map_err(|e| format!("Invalid profile overrides: {}", e))
}

#[allow(dead_code)]
pub fn read_active_profile_from_settings_json(dir: &Path) -> Option<String> {
    let settings_path = dir.join("settings.json");
    let raw = std::fs::read_to_string(settings_path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    value
        .get("active_profile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub fn write_active_profile_to_settings_json(
    dir: &Path,
    profile: Option<String>,
) -> Result<(), String> {
    let _ = std::fs::create_dir_all(dir);
    let settings_path = dir.join("settings.json");
    let mut value: Value = match std::fs::read_to_string(&settings_path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };

    if !value.is_object() {
        value = serde_json::json!({});
    }

    if let Value::Object(ref mut obj) = value {
        match profile {
            Some(p) => {
                obj.insert("active_profile".to_string(), Value::String(p));
            }
            None => {
                obj.remove("active_profile");
            }
        }
    }

    let pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    std::fs::write(settings_path, pretty).map_err(|e| e.to_string())?;
    Ok(())
}
