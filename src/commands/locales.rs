use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::settings_io;
use rustframe_capture::config;

#[derive(Serialize)]
pub struct LocaleEntry {
    pub code: String,
    pub data: Value,
}

#[derive(Serialize)]
pub struct LocaleDownloadResult {
    pub downloaded: usize,
    pub skipped: usize,
}

#[derive(Deserialize)]
struct GithubContentItem {
    name: String,
    download_url: Option<String>,
    #[serde(rename = "type")]
    item_type: String,
}

fn migrate_locales_from_profiles(dest_dir: &Path) {
    let Some(old_dir) = settings_io::rustframe_profiles_dir().map(|d| d.join("locales")) else {
        return;
    };

    if !old_dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(&old_dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("Failed to read legacy locales directory {:?}: {}", old_dir, e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let dest_path = dest_dir.join(file_name);
        if dest_path.exists() {
            continue;
        }
        if let Err(e) = std::fs::copy(&path, &dest_path) {
            log::warn!("Failed to migrate locale {:?}: {}", path, e);
        }
    }
}

fn locales_dir() -> Result<PathBuf, String> {
    let dir = settings_io::rustframe_locales_dir()
        .ok_or_else(|| "Could not find locales directory".to_string())?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create locales directory: {}", e))?;
    migrate_locales_from_profiles(&dir);
    Ok(dir)
}

fn is_valid_locale_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 16
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[tauri::command]
pub async fn list_locales() -> Result<Vec<String>, String> {
    let dir = locales_dir()?;

    let mut locales = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("Failed to read locales directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(code) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if is_valid_locale_code(code) {
            locales.push(code.to_string());
        }
    }

    locales.sort();
    locales.dedup();
    Ok(locales)
}

#[tauri::command]
pub async fn load_locales() -> Result<Vec<LocaleEntry>, String> {
    let dir = locales_dir()?;

    let entries = std::fs::read_dir(&dir).map_err(|e| format!("Failed to read locales directory: {}", e))?;
    let mut locales = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(code) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !is_valid_locale_code(code) {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(data) => data,
            Err(e) => {
                log::warn!("Failed to read locale file {:?}: {}", path, e);
                continue;
            }
        };

        let data: Value = match serde_json::from_str(&content) {
            Ok(value) => value,
            Err(e) => {
                log::warn!("Invalid locale JSON in {:?}: {}", path, e);
                continue;
            }
        };

        locales.push(LocaleEntry {
            code: code.to_string(),
            data,
        });
    }

    locales.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(locales)
}

#[tauri::command]
pub async fn download_locales(requested: Option<Vec<String>>) -> Result<LocaleDownloadResult, String> {
    let dir = locales_dir()?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(config::remote::LOCALES_DIR_API_URL)
        .header("User-Agent", "RustFrame")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch locales list: {}", e))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("No language files found.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!(
            "Locales manifest download failed (status {})",
            response.status()
        ));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read locales list: {}", e))?;

    let entries: Vec<GithubContentItem> = serde_json::from_str(&response_text)
        .map_err(|e| format!("Invalid locales list JSON: {}", e))?;

    let requested_set: Option<std::collections::HashSet<String>> = requested
        .filter(|list| !list.is_empty())
        .map(|list| list.into_iter().map(|s| s.to_lowercase()).collect());

    let mut downloaded = 0;
    let mut skipped = 0;

    for entry in entries {
        if entry.item_type != "file" {
            continue;
        }

        if !entry.name.ends_with(".json") {
            continue;
        }

        let code = entry.name.trim_end_matches(".json");
        if code.eq_ignore_ascii_case("en") {
            continue;
        }

        if let Some(ref set) = requested_set {
            if !set.contains(&code.to_lowercase()) {
                continue;
            }
        }

        if !is_valid_locale_code(code) {
            skipped += 1;
            continue;
        }

        let Some(url) = entry.download_url else {
            skipped += 1;
            continue;
        };

        let response = match client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                log::warn!("Failed to download locale {}: {}", code, e);
                skipped += 1;
                continue;
            }
        };

        if !response.status().is_success() {
            log::warn!(
                "Locale download failed for {} (status {})",
                code,
                response.status()
            );
            skipped += 1;
            continue;
        }

        let content = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                log::warn!("Failed to read locale {}: {}", code, e);
                skipped += 1;
                continue;
            }
        };

        if serde_json::from_str::<Value>(&content).is_err() {
            log::warn!("Invalid locale JSON for {}", code);
            skipped += 1;
            continue;
        }

        let filename = format!("{}.json", code);
        let dest_path = dir.join(&filename);
        if std::fs::write(&dest_path, content).is_err() {
            log::warn!("Failed to write locale {}", code);
            skipped += 1;
            continue;
        }

        downloaded += 1;
    }

    Ok(LocaleDownloadResult { downloaded, skipped })
}

#[tauri::command]
pub fn get_locales_path() -> Result<String, String> {
    let dir = locales_dir()?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_locales_folder() -> Result<(), String> {
    let dir = locales_dir()?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
