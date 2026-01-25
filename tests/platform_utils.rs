use rustframe_capture::platform_utils::{
    bgr_to_rgba, calculate_corner_thickness, calculate_inner_rect, rgba_to_bgr,
    validate_window_size,
};

#[test]
fn color_roundtrip_matches_expected() {
    let rgba = [10, 20, 30, 200];
    let bgr = rgba_to_bgr(rgba);
    let roundtrip = bgr_to_rgba(bgr);

    assert_eq!(roundtrip[0], rgba[0]);
    assert_eq!(roundtrip[1], rgba[1]);
    assert_eq!(roundtrip[2], rgba[2]);
    assert_eq!(roundtrip[3], 255);
}

#[test]
fn inner_rect_subtracts_border() {
    let (ix, iy, iw, ih) = calculate_inner_rect(100, 100, 200, 150, 4);
    assert_eq!(ix, 104);
    assert_eq!(iy, 104);
    assert_eq!(iw, 192);
    assert_eq!(ih, 142);
}

#[test]
fn window_size_validation_limits() {
    assert!(validate_window_size(100, 100).is_ok());
    assert!(validate_window_size(49, 100).is_err());
    assert!(validate_window_size(100, 49).is_err());
    assert!(validate_window_size(7681, 100).is_err());
}

#[test]
fn corner_thickness_respects_minimum() {
    let min = rustframe_capture::config::window::MIN_CORNER_THICKNESS;
    assert_eq!(calculate_corner_thickness(2), min);
    assert_eq!(calculate_corner_thickness(4), 8);
}
