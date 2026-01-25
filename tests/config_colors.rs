use rustframe_capture::config::colors;

#[test]
fn rgba_argb_roundtrip() {
    let rgba = [255, 128, 64, 200];
    let argb = colors::rgba_to_argb(rgba);
    let converted = colors::argb_to_rgba(argb);

    assert_eq!(rgba, converted);
}

#[test]
fn bgr_to_rgb_components() {
    let (r, g, b) = colors::bgr_u32_to_rgb_f64(0x000000FF);
    assert!((r - 1.0).abs() < 0.01);
    assert!((g - 0.0).abs() < 0.01);
    assert!((b - 0.0).abs() < 0.01);
}
