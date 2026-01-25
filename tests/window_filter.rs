use rustframe_capture::window_filter::{WindowFilterMode, WindowFilterSettings, WindowIdentifier};

fn base_settings(mode: WindowFilterMode) -> WindowFilterSettings {
    WindowFilterSettings {
        mode,
        excluded_windows: Vec::new(),
        included_windows: Vec::new(),
        auto_exclude_preview: true,
        dev_mode: false,
    }
}

#[test]
fn preview_is_excluded_by_default() {
    let preview = WindowIdentifier::preview_window();
    let settings = base_settings(WindowFilterMode::None);

    assert!(!settings.should_capture(&preview, Some(&preview)));
}

#[test]
fn exclude_list_filters_target() {
    let target = WindowIdentifier::app_window("com.test.app", "Target");
    let mut settings = base_settings(WindowFilterMode::ExcludeList);
    settings.excluded_windows.push(target.clone());

    assert!(!settings.should_capture(&target, None));
}

#[test]
fn include_only_allows_listed_window() {
    let target = WindowIdentifier::app_window("com.test.app", "Target");
    let other = WindowIdentifier::app_window("com.test.app", "Other");
    let mut settings = base_settings(WindowFilterMode::IncludeOnly);
    settings.included_windows.push(target.clone());

    assert!(settings.should_capture(&target, None));
    assert!(!settings.should_capture(&other, None));
}
