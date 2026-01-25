#![allow(dead_code)]

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "../src/settings.rs"]
mod settings;
#[path = "../src/settings_io.rs"]
mod settings_io;
#[path = "../src/profiles.rs"]
mod profiles;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("rustframe_test_{}_{}_{}", label, std::process::id(), nanos));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_json(path: &Path, value: serde_json::Value) {
    let data = serde_json::to_string_pretty(&value).unwrap();
    fs::write(path, data).unwrap();
}

#[test]
fn scan_capture_profiles_includes_os_and_root() {
    let temp = TempDir::new("profiles_scan");
    let os_dir = temp
        .path
        .join(settings_io::get_os_profile_subdir());
    fs::create_dir_all(&os_dir).unwrap();

    write_json(&os_dir.join("discord.json"), json!({"ok": true}));
    write_json(&temp.path.join("profile_zoom.json"), json!({"ok": true}));
    fs::write(temp.path.join("ignore.txt"), "nope").unwrap();
    fs::write(temp.path.join("bad.json"), "\"string\"").unwrap();

    let profiles_list = profiles::scan_capture_profiles(&temp.path);
    let ids: Vec<_> = profiles_list.iter().map(|p| p.id.as_str()).collect();

    assert!(ids.contains(&"discord"));
    assert!(ids.contains(&"zoom"));
    assert_eq!(profiles_list.len(), 2);
}

#[test]
fn read_profile_overrides_prefers_os_dir() {
    let temp = TempDir::new("profile_read_os");
    let os_dir = temp
        .path
        .join(settings_io::get_os_profile_subdir());
    fs::create_dir_all(&os_dir).unwrap();

    write_json(&os_dir.join("demo.json"), json!({"border_width": 7}));
    write_json(&temp.path.join("demo.json"), json!({"border_width": 3}));

    let value = profiles::read_profile_overrides(&temp.path, "demo").unwrap();
    assert_eq!(value["border_width"], json!(7));
}

#[test]
fn read_profile_overrides_falls_back_to_legacy_names() {
    let temp = TempDir::new("profile_read_legacy");
    write_json(
        &temp.path.join("profile_legacy.json"),
        json!({"border_width": 11}),
    );

    let value = profiles::read_profile_overrides(&temp.path, "legacy").unwrap();
    assert_eq!(value["border_width"], json!(11));
}

#[test]
fn apply_profile_overrides_merges_settings() {
    let base = settings::Settings::default();
    let overrides = json!({
        "border_width": 12,
        "show_border": false,
        "capture_clicks": false
    });

    let merged = settings_io::apply_profile_overrides(&base, overrides).unwrap();
    assert_eq!(merged.border_width, 12);
    assert!(!merged.show_border);
    assert!(!merged.capture_clicks);
}

#[test]
fn write_and_read_active_profile() {
    let temp = TempDir::new("active_profile");

    settings_io::write_active_profile_to_settings_json(&temp.path, Some("demo".to_string()))
        .unwrap();
    let active = settings_io::read_active_profile_from_settings_json(&temp.path);
    assert_eq!(active.as_deref(), Some("demo"));

    settings_io::write_active_profile_to_settings_json(&temp.path, None).unwrap();
    let cleared = settings_io::read_active_profile_from_settings_json(&temp.path);
    assert!(cleared.is_none());
}
