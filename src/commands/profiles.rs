use tauri::State;
use std::path::Path;

use crate::profiles::{
    CaptureProfileHints, CaptureProfileInfo, ProfileDetails, ProfileVersionData, PROFILE_VERSION_URL,
};
use crate::{profiles, settings_io, AppState};

#[tauri::command]
pub async fn get_capture_profiles() -> Result<Vec<CaptureProfileInfo>, String> {
    let Some(profiles_dir) = settings_io::rustframe_profiles_dir() else {
        return Ok(vec![]);
    };

    // Ensure Profiles directory exists
    let _ = std::fs::create_dir_all(&profiles_dir);

    // Scan profiles from the Profiles directory
    Ok(profiles::scan_capture_profiles(&profiles_dir))
}

#[tauri::command]
pub async fn get_active_capture_profile(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.active_profile.lock().unwrap().clone())
}

#[tauri::command]
pub async fn get_capture_profile_hints(profile: String) -> Result<CaptureProfileHints, String> {
    let Some(profiles_dir) = settings_io::rustframe_profiles_dir() else {
        return Ok(CaptureProfileHints {
            hide_taskbar_after_ms: None,
        });
    };

    let Some(overrides) = profiles::read_profile_overrides(&profiles_dir, &profile) else {
        return Ok(CaptureProfileHints {
            hide_taskbar_after_ms: None,
        });
    };

    let hide_taskbar_after_ms = overrides
        .get("winapi_destination_hide_taskbar_after_ms")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());

    Ok(CaptureProfileHints {
        hide_taskbar_after_ms,
    })
}

#[tauri::command]
pub async fn set_active_capture_profile(
    profile: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    *state.active_profile.lock().unwrap() = profile.clone();
    if let Some(dir) = settings_io::rustframe_config_dir() {
        settings_io::write_active_profile_to_settings_json(&dir, profile)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_local_profile_version() -> Result<ProfileVersionData, String> {
    let Some(profiles_dir) = settings_io::rustframe_profiles_dir() else {
        return Err("Could not find profiles directory".to_string());
    };

    let version_path = profiles_dir.parent().unwrap().join("version.json");

    if !version_path.exists() {
        return Err("Local version.json not found".to_string());
    }

    let content = std::fs::read_to_string(&version_path)
        .map_err(|e| format!("Failed to read local version.json: {}", e))?;

    let version_data: ProfileVersionData = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse local version.json: {}", e))?;

    Ok(version_data)
}

#[tauri::command]
pub async fn check_profile_updates() -> Result<ProfileVersionData, String> {
    tracing::info!("Checking for profile updates from: {}", PROFILE_VERSION_URL);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| {
            let error_msg = format!("Failed to create HTTP client: {}", e);
            tracing::error!("{}", error_msg);
            "Network error: Could not initialize update check".to_string()
        })?;

    let response = client
        .get(PROFILE_VERSION_URL)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to fetch profile versions from GitHub: {}", e);
            tracing::error!("{}", error_msg);
            "Network error: Could not connect to GitHub. Please check your internet connection".to_string()
        })?;

    let status = response.status();
    tracing::info!("GitHub response status: {}", status);

    if !status.is_success() {
        let error_msg = format!("GitHub returned non-success status: {}", status);
        tracing::error!("{}", error_msg);
        return Err(format!("Update server error: GitHub returned status {}", status));
    }

    // Get response text first for better error reporting
    let response_text = response
        .text()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to read response body: {}", e);
            tracing::error!("{}", error_msg);
            "Network error: Could not read server response".to_string()
        })?;

    tracing::debug!("Response body preview: {}", &response_text[..response_text.len().min(200)]);

    // Try to parse JSON
    let version_data: ProfileVersionData = serde_json::from_str(&response_text)
        .map_err(|e| {
            let error_msg = format!("Failed to parse version.json: {}. Response: {}", e, &response_text[..response_text.len().min(500)]);
            tracing::error!("{}", error_msg);
            "Data error: Server returned invalid profile data. Please try again later".to_string()
        })?;

    tracing::info!("Successfully loaded profile version data: v{}", version_data.version);
    Ok(version_data)
}

#[tauri::command]
pub async fn update_local_profile_version(version_data: ProfileVersionData) -> Result<(), String> {
    let Some(profiles_dir) = settings_io::rustframe_profiles_dir() else {
        return Err("Could not find profiles directory".to_string());
    };

    let version_path = profiles_dir.parent().unwrap().join("version.json");

    let content = serde_json::to_string_pretty(&version_data)
        .map_err(|e| format!("Failed to serialize version data: {}", e))?;

    std::fs::write(&version_path, content)
        .map_err(|e| format!("Failed to write version.json: {}", e))?;

    tracing::info!("Updated local version.json");
    Ok(())
}

#[tauri::command]
pub async fn download_profile(profile_id: String, file_name: Option<String>) -> Result<(), String> {
    let platform = settings_io::get_os_profile_subdir();
    let Some(profiles_dir) = settings_io::rustframe_profiles_dir() else {
        return Err("Could not find profiles directory".to_string());
    };

    let platform_dir = profiles_dir.join(platform);
    let _ = std::fs::create_dir_all(&platform_dir);

    let requested_filename = file_name.unwrap_or_else(|| format!("{}.json", profile_id));
    let filename = Path::new(&requested_filename)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid profile file name".to_string())?
        .to_string();
    let url = profiles::get_profile_download_url(platform, &filename);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download profile: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        if status.as_u16() == 404 {
            return Err("Profile file not found on server".to_string());
        }
        return Err(format!("Profile download failed (status {})", status));
    }

    let content = response
        .text()
        .await
        .map_err(|e| format!("Failed to read profile content: {}", e))?;

    // Validate JSON before saving
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Downloaded profile has invalid JSON: {}", e))?;

    let dest_path = platform_dir.join(&filename);
    std::fs::write(&dest_path, content).map_err(|e| format!("Failed to save profile: {}", e))?;

    tracing::info!("Downloaded profile '{}' to {:?}", profile_id, dest_path);
    Ok(())
}

#[tauri::command]
pub async fn delete_profile(profile_id: String) -> Result<(), String> {
    let platform = settings_io::get_os_profile_subdir();
    let Some(profiles_dir) = settings_io::rustframe_profiles_dir() else {
        return Err("Could not find profiles directory".to_string());
    };

    let platform_dir = profiles_dir.join(platform);
    let filename = format!("{}.json", profile_id);
    let profile_path = platform_dir.join(&filename);

    if !profile_path.exists() {
        return Err(format!("Profile '{}' not found", profile_id));
    }

    std::fs::remove_file(&profile_path)
        .map_err(|e| format!("Failed to delete profile: {}", e))?;

    tracing::info!("Deleted profile '{}' from {:?}", profile_id, profile_path);
    Ok(())
}

#[tauri::command]
pub async fn get_profile_details(profile_id: String) -> Result<ProfileDetails, String> {
    let platform = settings_io::get_os_profile_subdir();
    let Some(profiles_dir) = settings_io::rustframe_profiles_dir() else {
        return Err("Could not find profiles directory".to_string());
    };

    let platform_dir = profiles_dir.join(platform);
    let filename = format!("{}.json", profile_id);
    let profile_path = platform_dir.join(&filename);

    if !profile_path.exists() {
        return Err(format!("Profile '{}' not found", profile_id));
    }

    let content = std::fs::read_to_string(&profile_path)
        .map_err(|e| format!("Failed to read profile: {}", e))?;

    let settings: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profile: {}", e))?;

    let name = settings
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&profile_id)
        .to_string();

    let description = settings
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(ProfileDetails {
        id: profile_id.clone(),
        name,
        description,
        version: "1.0.0".to_string(), // TODO: Get from version.json
        file_name: filename,
        settings,
    })
}
