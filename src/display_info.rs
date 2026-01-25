#![allow(deprecated)]
//! Display Information Manager
//!
//! Central management of display properties (scale factor, resolution, bounds).
//! All coordinate conversions should use this module to ensure consistency.
//!
//! # Cross-Platform Support
//!
//! ## macOS
//! Uses NSScreen.mainScreen to get accurate display information including:
//! - backingScaleFactor (2.0 for Retina displays)
//! - Frame bounds in points
//! - Automatically handles coordinate system (bottom-left origin)
//!
//! ## Windows  
//! Uses Win32 GetDeviceCaps API to query:
//! - LOGPIXELSX for DPI (scale factor = DPI / 96.0)
//! - HORZRES, VERTRES for screen resolution
//! - Handles high-DPI displays correctly
//!
//! ## Linux
//! Detects X11 or Wayland and provides sensible defaults:
//! - Checks DISPLAY environment variable for X11
//! - Checks WAYLAND_DISPLAY for Wayland
//! - Uses 1.0 scale factor by default
//! - TODO: Integrate with X11/Wayland APIs for accurate detection
//!
//! # Usage
//!
//! ```rust
//! # use rustframe_capture::display_info;
//! // Initialize once at application startup
//! display_info::initialize().ok();
//!
//! // Get display info anywhere
//! let info = display_info::get();
//! println!("Scale: {}x", info.scale_factor);
//!
//! // Convert coordinates
//! let pixels = info.points_to_pixels(100.0);
//! let (x_pt, y_pt) = (100, 200);
//! let (x_px, y_px) = info.point_to_pixel_coords(x_pt, y_pt);
//! ```

use lazy_static::lazy_static;
use log::info;
use std::sync::{Arc, RwLock};
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU8, Ordering};
#[cfg(target_os = "macos")]
use core_graphics::geometry::CGPoint;

lazy_static! {
    /// Global display information singleton
    static ref DISPLAY_INFO: Arc<RwLock<DisplayInfo>> = Arc::new(RwLock::new(DisplayInfo::default()));
}

#[cfg(target_os = "macos")]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum CGEventCoordMode {
    TopLeftPoints = 1,
    TopLeftPixels = 2,
    BottomLeftPoints = 3,
    BottomLeftPixels = 4,
}

#[cfg(target_os = "macos")]
static CG_EVENT_COORD_MODE: AtomicU8 = AtomicU8::new(0);

/// Display information for coordinate system management
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// Backing scale factor (2.0 for Retina, 1.0 for standard)
    pub scale_factor: f64,
    /// Screen width in points (logical coordinates)
    pub width_points: f64,
    /// Screen height in points (logical coordinates)  
    pub height_points: f64,
    /// Screen width in pixels (physical coordinates)
    pub width_pixels: u32,
    /// Screen height in pixels (physical coordinates)
    pub height_pixels: u32,
    /// Whether display info has been initialized
    pub initialized: bool,
}

impl Default for DisplayInfo {
    fn default() -> Self {
        Self {
            scale_factor: 1.0,
            width_points: 0.0,
            height_points: 0.0,
            width_pixels: 0,
            height_pixels: 0,
            initialized: false,
        }
    }
}

impl DisplayInfo {
    /// Convert points to pixels
    #[allow(dead_code)]
    pub fn points_to_pixels(&self, points: f64) -> i32 {
        (points * self.scale_factor).round() as i32
    }

    /// Convert pixels to points
    #[allow(dead_code)]
    pub fn pixels_to_points(&self, pixels: i32) -> f64 {
        pixels as f64 / self.scale_factor
    }

    /// Convert point coordinates to pixel coordinates
    #[allow(dead_code)]
    pub fn point_to_pixel_coords(&self, x_points: i32, y_points: i32) -> (i32, i32) {
        (
            self.points_to_pixels(x_points as f64),
            self.points_to_pixels(y_points as f64),
        )
    }

    /// Convert pixel coordinates to point coordinates  
    #[allow(dead_code)]
    pub fn pixel_to_point_coords(&self, x_pixels: i32, y_pixels: i32) -> (i32, i32) {
        (
            self.pixels_to_points(x_pixels) as i32,
            self.pixels_to_points(y_pixels) as i32,
        )
    }

    #[cfg(target_os = "macos")]
    pub fn cg_event_to_screen_points(&self, x: f64, y: f64) -> (f64, f64) {
        let mode = cg_event_coord_mode();
        let scale = if self.scale_factor > 0.0 {
            self.scale_factor
        } else {
            1.0
        };
        let height_points = if self.height_points > 0.0 {
            self.height_points
        } else {
            0.0
        };
        let flip_y = |val: f64| if height_points > 0.0 { height_points - val } else { val };

        match mode {
            CGEventCoordMode::TopLeftPoints => (x, y),
            CGEventCoordMode::TopLeftPixels => (x / scale, y / scale),
            CGEventCoordMode::BottomLeftPoints => (x, flip_y(y)),
            CGEventCoordMode::BottomLeftPixels => (x / scale, flip_y(y / scale)),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn cg_event_to_screen_pixels(&self, x: f64, y: f64) -> (i32, i32) {
        let (x_points, y_points) = self.cg_event_to_screen_points(x, y);
        (
            self.points_to_pixels(x_points),
            self.points_to_pixels(y_points),
        )
    }

    /// Convert macOS CGEvent coordinates to screen capture pixel coordinates.
    /// Uses a detected coordinate mode to align CGEvent with capture space.
    #[cfg(target_os = "macos")]
    pub fn macos_event_to_screen_pixels(&self, x: f64, y: f64) -> (i32, i32) {
        self.cg_event_to_screen_pixels(x, y)
    }

    /// Convert screen pixels (top-left origin) to macOS NSEvent coordinates (bottom-left origin, points)
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    pub fn screen_pixels_to_macos_event(&self, x_pixels: i32, y_pixels: i32) -> (f64, f64) {
        // Convert to points
        let x_points = x_pixels as f64 / self.scale_factor;
        let y_points = y_pixels as f64 / self.scale_factor;

        // Flip Y coordinate (macOS uses bottom-left origin)
        let y_flipped_points = self.height_points - y_points;

        (x_points, y_flipped_points)
    }

    /// Convert AppKit/NSWindow coordinates (bottom-left origin, points) to CGDisplay/SCK coordinates (top-left origin, pixels)
    ///
    /// Used for:
    /// - Converting border window position to ScreenCaptureKit sourceRect
    /// - Any conversion from NSWindow/NSView coordinates to screen capture coordinates
    ///
    /// AppKit coordinate system:
    /// - Origin: BOTTOM-LEFT of screen
    /// - Units: POINTS
    ///
    /// CGDisplay/ScreenCaptureKit coordinate system:
    /// - Origin: TOP-LEFT of screen  
    /// - Units: PIXELS
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    pub fn appkit_to_cgdisplay(
        &self,
        x_points: f64,
        y_points: f64,
        width_points: f64,
        height_points: f64,
    ) -> (f64, f64, f64, f64) {
        // Scale to pixels
        let x_px = x_points * self.scale_factor;
        let y_px = y_points * self.scale_factor;
        let w_px = width_points * self.scale_factor;
        let h_px = height_points * self.scale_factor;

        // Flip Y coordinate: bottom-left → top-left
        let y_flipped_px = (self.height_pixels as f64) - y_px - h_px;

        (x_px, y_flipped_px, w_px, h_px)
    }
}

#[cfg(target_os = "macos")]
impl CGEventCoordMode {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::TopLeftPoints),
            2 => Some(Self::TopLeftPixels),
            3 => Some(Self::BottomLeftPoints),
            4 => Some(Self::BottomLeftPixels),
            _ => None,
        }
    }
}

#[cfg(target_os = "macos")]
fn cg_event_coord_mode() -> CGEventCoordMode {
    let stored = CG_EVENT_COORD_MODE.load(Ordering::SeqCst);
    if let Some(mode) = CGEventCoordMode::from_u8(stored) {
        return mode;
    }

    let detected = detect_cg_event_coord_mode();
    CG_EVENT_COORD_MODE.store(detected as u8, Ordering::SeqCst);
    detected
}

#[cfg(target_os = "macos")]
fn detect_cg_event_coord_mode() -> CGEventCoordMode {
    unsafe {
        if pthread_main_np() != 0 {
            return detect_cg_event_coord_mode_on_main();
        }

        struct Ctx {
            mode: CGEventCoordMode,
        }

        extern "C" fn detect(ctx_ptr: *mut std::ffi::c_void) {
            let ctx = unsafe { &mut *(ctx_ptr as *mut Ctx) };
            ctx.mode = unsafe { detect_cg_event_coord_mode_on_main() };
        }

        let mut ctx = Ctx {
            mode: CGEventCoordMode::TopLeftPoints,
        };

        dispatch_sync_f(
            &_dispatch_main_q,
            &mut ctx as *mut _ as *mut std::ffi::c_void,
            detect,
        );

        ctx.mode
    }
}

#[cfg(target_os = "macos")]
unsafe fn detect_cg_event_coord_mode_on_main() -> CGEventCoordMode {
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSPoint, NSRect};
    use objc::{class, msg_send, sel, sel_impl};

    let screen: id = msg_send![class!(NSScreen), mainScreen];
    if screen == nil {
        return CGEventCoordMode::TopLeftPoints;
    }

    let screen_frame: NSRect = msg_send![screen, frame];
    let screen_height_points = screen_frame.size.height;
    let scale: f64 = msg_send![screen, backingScaleFactor];
    let scale = if scale > 0.0 { scale } else { 1.0 };

    let ns_loc: NSPoint = msg_send![class!(NSEvent), mouseLocation];

    let event = CGEventCreate(std::ptr::null_mut());
    if event.is_null() {
        return CGEventCoordMode::TopLeftPoints;
    }
    let cg_loc: CGPoint = CGEventGetLocation(event);
    CFRelease(event);

    let candidates = [
        (
            CGEventCoordMode::TopLeftPoints,
            cg_loc.x,
            screen_height_points - cg_loc.y,
        ),
        (
            CGEventCoordMode::TopLeftPixels,
            cg_loc.x / scale,
            screen_height_points - (cg_loc.y / scale),
        ),
        (
            CGEventCoordMode::BottomLeftPoints,
            cg_loc.x,
            cg_loc.y,
        ),
        (
            CGEventCoordMode::BottomLeftPixels,
            cg_loc.x / scale,
            cg_loc.y / scale,
        ),
    ];

    let mut best = CGEventCoordMode::TopLeftPoints;
    let mut best_err = f64::INFINITY;

    for (mode, x, y) in candidates {
        let dx = x - ns_loc.x;
        let dy = y - ns_loc.y;
        let err = dx * dx + dy * dy;
        if err < best_err {
            best_err = err;
            best = mode;
        }
    }

    log::debug!(
        "[DISPLAY_INFO] CGEvent coord mode {:?} (error {:.3})",
        best,
        best_err
    );

    best
}

#[cfg(target_os = "macos")]
extern "C" {
    static _dispatch_main_q: std::ffi::c_void;

    fn dispatch_sync_f(
        queue: *const std::ffi::c_void,
        context: *mut std::ffi::c_void,
        work: extern "C" fn(*mut std::ffi::c_void),
    );

    fn CGEventCreate(source: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn CGEventGetLocation(event: *mut std::ffi::c_void) -> CGPoint;
    fn CFRelease(cf: *mut std::ffi::c_void);

    fn pthread_main_np() -> i32;
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn flip_y_point_with_height(y_points: f64, screen_height_points: f64) -> f64 {
    screen_height_points - y_points
}

#[cfg(target_os = "macos")]
pub fn flip_y_rect_with_height(y_points: f64, height_points: f64, screen_height_points: f64) -> f64 {
    screen_height_points - y_points - height_points
}

/// Initialize display information from the operating system
#[cfg(target_os = "macos")]
pub fn initialize() -> anyhow::Result<()> {
    use cocoa::base::id;
    use cocoa::foundation::NSRect;
    use objc::*;

    unsafe {
        let screen: id = msg_send![class!(NSScreen), mainScreen];
        if screen.is_null() {
            return Err(anyhow::anyhow!("Failed to get main screen"));
        }

        let frame: NSRect = msg_send![screen, frame];
        let scale_factor: f64 = msg_send![screen, backingScaleFactor];

        let width_points = frame.size.width;
        let height_points = frame.size.height;
        let width_pixels = (width_points * scale_factor).round() as u32;
        let height_pixels = (height_points * scale_factor).round() as u32;

        let mut display_info = DISPLAY_INFO
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to lock display info: {}", e))?;

        *display_info = DisplayInfo {
            scale_factor,
            width_points,
            height_points,
            width_pixels,
            height_pixels,
            initialized: true,
        };

        info!(
            "[DISPLAY_INFO] Initialized: {}x{} points ({}x{} pixels) @ {:.1}x scale",
            width_points as u32, height_points as u32, width_pixels, height_pixels, scale_factor
        );

        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn initialize() -> anyhow::Result<()> {
    use std::ptr::null_mut;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        GetDC, GetDeviceCaps, ReleaseDC, HORZRES, LOGPIXELSX, VERTRES,
    };

    unsafe {
        let hdc = GetDC(Some(HWND(null_mut())));
        if hdc.0.is_null() {
            return Err(anyhow::anyhow!("Failed to get device context"));
        }

        let dpi = GetDeviceCaps(Some(hdc), LOGPIXELSX);
        let scale_factor = (dpi as f64 / 96.0).max(1.0); // 96 DPI is baseline, min 1.0

        let width_pixels = GetDeviceCaps(Some(hdc), HORZRES) as u32;
        let height_pixels = GetDeviceCaps(Some(hdc), VERTRES) as u32;

        let _ = ReleaseDC(Some(HWND(null_mut())), hdc);

        let width_points = width_pixels as f64 / scale_factor;
        let height_points = height_pixels as f64 / scale_factor;

        let mut display_info = DISPLAY_INFO
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to lock display info: {}", e))?;

        *display_info = DisplayInfo {
            scale_factor,
            width_points,
            height_points,
            width_pixels,
            height_pixels,
            initialized: true,
        };

        info!(
            "[DISPLAY_INFO] Initialized: {}x{} points ({}x{} pixels) @ {:.1}x scale",
            width_points as u32, height_points as u32, width_pixels, height_pixels, scale_factor
        );

        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub fn initialize() -> anyhow::Result<()> {
    // Try to get display info from environment variables (set by X11/Wayland)
    let (width_pixels, height_pixels, scale_factor) = if let Ok(display) = std::env::var("DISPLAY")
    {
        // X11 is available
        info!("[DISPLAY_INFO] X11 display detected: {}", display);
        // TODO: Use X11 APIs to get actual resolution
        // For now, use common default
        (1920, 1080, 1.0)
    } else if std::env::var("WAYLAND_DISPLAY").is_ok() {
        // Wayland is available
        info!("[DISPLAY_INFO] Wayland display detected");
        // TODO: Use Wayland APIs to get actual resolution
        (1920, 1080, 1.0)
    } else {
        warn!("[DISPLAY_INFO] No display server detected, using defaults");
        (1920, 1080, 1.0)
    };

    let width_points = width_pixels as f64 / scale_factor;
    let height_points = height_pixels as f64 / scale_factor;

    let mut display_info = DISPLAY_INFO
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to lock display info: {}", e))?;

    *display_info = DisplayInfo {
        scale_factor,
        width_points,
        height_points,
        width_pixels,
        height_pixels,
        initialized: true,
    };

    info!(
        "[DISPLAY_INFO] Initialized: {}x{} points ({}x{} pixels) @ {:.1}x scale",
        width_points as u32, height_points as u32, width_pixels, height_pixels, scale_factor
    );

    Ok(())
}

/// Get the current display information
pub fn get() -> DisplayInfo {
    DISPLAY_INFO
        .read()
        .map(|info| info.clone())
        .unwrap_or_default()
}

/// Get the scale factor (convenience function)
pub fn scale_factor() -> f64 {
    get().scale_factor
}

/// Check if display info has been initialized
pub fn is_initialized() -> bool {
    DISPLAY_INFO
        .read()
        .map(|info| info.initialized)
        .unwrap_or(false)
}
