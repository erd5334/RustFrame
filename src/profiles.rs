use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::settings_io;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureProfileInfo {
    /// Profile id (derived from filename), e.g. "discord" for profile_discord.json
    pub id: String,
    /// Filename, e.g. "profile_discord.json"
    pub file_name: String,
    /// Display name from profile JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureProfileHints {
    /// If present in the selected profile, the preview window will be hidden from taskbar/Alt-Tab after start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_taskbar_after_ms: Option<u32>,
}

#[allow(clippy::redundant_static_lifetimes)]
mod bundled_profiles {
    include!(concat!(env!("OUT_DIR"), "/rustframe_bundled_profiles.rs"));
}

fn bundled_profiles_for_platform() -> &'static [(&'static str, &'static str)] {
    bundled_profiles::PROFILES
}

pub fn bootstrap_profiles_if_missing(config_dir: &Path) {
    let profiles_base_dir = config_dir.join("Profiles");
    let profiles_dir = profiles_base_dir.join(settings_io::get_os_profile_subdir());
    let _ = std::fs::create_dir_all(&profiles_dir);

    // Bootstrap version.json if missing
    let version_json_path = profiles_base_dir.join("version.json");
    if !version_json_path.exists() {
        // Use bundled version.json from resources
        const BUNDLED_VERSION_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/profiles/version.json"));
        if let Err(e) = std::fs::write(&version_json_path, BUNDLED_VERSION_JSON) {
            log::warn!("Failed to seed version.json: {}", e);
        }
    }

    // Bootstrap profile files
    for (file_name, contents) in bundled_profiles_for_platform() {
        let dst = profiles_dir.join(file_name);
        if dst.exists() {
            continue;
        }
        if let Err(e) = std::fs::write(&dst, contents) {
            log::warn!("Failed to seed profile {}: {}", dst.display(), e);
        }
    }
}

pub fn scan_capture_profiles(dir: &Path) -> Vec<CaptureProfileInfo> {
    let mut profiles = Vec::new();

    // First, check OS-specific subdirectory
    let os_dir = dir.join(settings_io::get_os_profile_subdir());
    if os_dir.exists() {
        scan_profiles_from_dir(&os_dir, &mut profiles);
    }

    // Also scan root directory for backward compatibility
    scan_profiles_from_dir(dir, &mut profiles);

    profiles.sort_by(|a, b| a.id.cmp(&b.id));
    profiles.dedup_by(|a, b| a.id == b.id);
    profiles
}

fn scan_profiles_from_dir(dir: &Path, profiles: &mut Vec<CaptureProfileInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip version.json - it's not a profile
        if file_name == "version.json" {
            continue;
        }

        // Support both "profile_xyz.json" (old) and "xyz.json" (new) formats
        let id = if file_name.starts_with("profile_") {
            file_name
                .trim_start_matches("profile_")
                .trim_end_matches(".json")
                .to_string()
        } else {
            file_name.trim_end_matches(".json").to_string()
        };

        if id.is_empty() {
            continue;
        }

        // Only include valid JSON object profiles
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        if !value.is_object() {
            continue;
        }

        // Extract name from JSON if available
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        profiles.push(CaptureProfileInfo {
            id,
            file_name,
            name,
        });
    }
}

pub fn read_profile_overrides(dir: &Path, profile_id: &str) -> Option<serde_json::Value> {
    // Try new format first: Profiles/os/profilename.json
    let os_subdir = dir.join(settings_io::get_os_profile_subdir());
    let new_format_path = os_subdir.join(format!("{}.json", profile_id));

    if new_format_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&new_format_path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                if value.is_object() {
                    return Some(value);
                }
            }
        }
    }

    // Try old format: profile_profilename.json in root
    let old_format_name = format!("profile_{}.json", profile_id);
    let old_format_path = dir.join(&old_format_name);

    if old_format_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&old_format_path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                if value.is_object() {
                    return Some(value);
                }
            }
        }
    }

    // Try simple format in root: profilename.json
    let simple_format_path = dir.join(format!("{}.json", profile_id));
    if simple_format_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&simple_format_path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                if value.is_object() {
                    return Some(value);
                }
            }
        }
    }

    None
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProfileVersionInfo {
    pub version: String,
    pub file: String,
    pub last_updated: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProfileVersionData {
    pub version: String,
    pub last_updated: String,
    pub profiles:
        std::collections::HashMap<String, std::collections::HashMap<String, ProfileVersionInfo>>,
}

#[derive(Serialize)]
pub struct ProfileDetails {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub file_name: String,
    pub settings: serde_json::Value,
}

pub const PROFILE_VERSION_URL: &str =
    "https://raw.githubusercontent.com/salihcantekin/RustFrame/master/resources/profiles/version.json";

pub fn get_profile_download_url(platform: &str, filename: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/salihcantekin/RustFrame/master/resources/profiles/{}/{}",
        platform, filename
    )
}
