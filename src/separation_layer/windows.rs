//! Windows Separation Layer Window
//! 
//! Mimics RegionToShare's approach: A window between border and preview
//! in z-order that shows solid color when border is over desktop.

use crate::platform;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::thread;
use std::sync::mpsc;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, RECT, HINSTANCE};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, DeleteObject, EndPaint, GradientFill,
    SelectObject, SetBkMode, SetTextColor, TextOutW, HGDIOBJ, LOGFONTW, PAINTSTRUCT,
    TRIVERTEX, GRADIENT_RECT, GRADIENT_FILL_RECT_V, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, CreateWindowExW, DefWindowProcW, DrawIconEx, RegisterClassExW,
    SetWindowPos, DestroyIcon, HICON, IMAGE_FLAGS, DI_NORMAL,
    CS_HREDRAW, CS_VREDRAW, HWND_BOTTOM, SWP_NOACTIVATE, SWP_SHOWWINDOW, WNDCLASSEXW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, MSG, GetMessageW, TranslateMessage, DispatchMessageW,
    PostQuitMessage, SWP_ASYNCWINDOWPOS, WM_PAINT, WM_ERASEBKGND
};

const CLASS_NAME: &str = "RustFrameSeparationLayer";
const ICON_BYTES: &[u8] = include_bytes!("../../icons/icon.ico");

lazy_static! {
    static ref SEPARATION_HWND: Mutex<isize> = Mutex::new(0);
    static ref SEPARATION_ICON: Mutex<isize> = Mutex::new(0);
    static ref SEPARATION_COLOR: Mutex<u32> = Mutex::new(0);
}

pub struct SeparationLayer {
    hwnd: isize,
    #[allow(dead_code)] // Keep handle to prevent detach if we wanted to join, but we let it run
    thread_handle: Option<thread::JoinHandle<()>>,
}

unsafe impl Send for SeparationLayer {}
unsafe impl Sync for SeparationLayer {}

fn set_face_name(logfont: &mut LOGFONTW, name: &str) {
    let mut iter = name.encode_utf16();
    for slot in logfont.lfFaceName.iter_mut() {
        match iter.next() {
            Some(ch) => *slot = ch,
            None => break,
        }
    }
}

fn adjust_color(color: u32, factor: f32) -> u32 {
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;
    let r = (r * factor).max(0.0).min(255.0) as u8;
    let g = (g * factor).max(0.0).min(255.0) as u8;
    let b = (b * factor).max(0.0).min(255.0) as u8;
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn blend_color(color: u32, target: u32, t: f32) -> u32 {
    let r1 = ((color >> 16) & 0xFF) as f32;
    let g1 = ((color >> 8) & 0xFF) as f32;
    let b1 = (color & 0xFF) as f32;
    let r2 = ((target >> 16) & 0xFF) as f32;
    let g2 = ((target >> 8) & 0xFF) as f32;
    let b2 = (target & 0xFF) as f32;
    let r = (r1 + (r2 - r1) * t).max(0.0).min(255.0) as u8;
    let g = (g1 + (g2 - g1) * t).max(0.0).min(255.0) as u8;
    let b = (b1 + (b2 - b1) * t).max(0.0).min(255.0) as u8;
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn trivertex_color(color: u32) -> (u16, u16, u16) {
    let r = ((color >> 16) & 0xFF) as u16;
    let g = ((color >> 8) & 0xFF) as u16;
    let b = (color & 0xFF) as u16;
    ((r << 8) | r, (g << 8) | g, (b << 8) | b)
}

fn parse_best_icon(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 6 {
        return None;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let entry_size = 16;
    if bytes.len() < 6 + count * entry_size {
        return None;
    }

    let mut best: Option<(usize, usize, u32)> = None;
    for i in 0..count {
        let base = 6 + i * entry_size;
        let width = bytes[base];
        let height = bytes[base + 1];
        let width = if width == 0 { 256 } else { width as u32 };
        let height = if height == 0 { 256 } else { height as u32 };
        let bytes_in_res = u32::from_le_bytes([
            bytes[base + 8],
            bytes[base + 9],
            bytes[base + 10],
            bytes[base + 11],
        ]) as usize;
        let image_offset = u32::from_le_bytes([
            bytes[base + 12],
            bytes[base + 13],
            bytes[base + 14],
            bytes[base + 15],
        ]) as usize;
        if image_offset + bytes_in_res > bytes.len() {
            continue;
        }
        let score = width.saturating_mul(height);
        if best.map_or(true, |(_, _, best_score)| score > best_score) {
            best = Some((image_offset, bytes_in_res, score));
        }
    }
    best.map(|(offset, size, _)| (offset, size))
}

fn load_icon_from_bytes(bytes: &[u8], desired_size: i32) -> Option<HICON> {
    let (offset, size) = parse_best_icon(bytes)?;
    let icon_bytes = bytes.get(offset..offset + size)?;
    match unsafe {
        CreateIconFromResourceEx(
            icon_bytes,
            true,
            0x00030000,
            desired_size,
            desired_size,
            IMAGE_FLAGS(0),
        )
    } {
        Ok(icon) => Some(icon),
        Err(err) => {
            log::warn!("Separation layer icon load failed: {:?}", err);
            None
        }
    }
}

impl SeparationLayer {
    /// Create separation layer window
    /// Color format: 0xRRGGBB (e.g., 0x4682B4 for Steel Blue)
    pub fn new(x: i32, y: i32, width: i32, height: i32, color: u32) -> Option<Self> {
        let (tx, rx) = mpsc::channel();

        // Spawn a dedicated thread for the window to ensure it has a message loop
        // This prevents "Not Responding" and allows SetWindowPos to work correctly
        let thread_handle = thread::spawn(move || {
            unsafe {
                // Register window class
                let hinstance = match GetModuleHandleW(None) {
                    Ok(h) => h,
                    Err(_) => {
                        let _ = tx.send(None);
                        return;
                    }
                };
                
                let class_name_wide: Vec<u16> = CLASS_NAME
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                
                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(wndproc),
                    hInstance: hinstance.into(),
                    lpszClassName: windows::core::PCWSTR(class_name_wide.as_ptr()),
                    ..Default::default()
                };
                
                RegisterClassExW(&wc);
                
                if let Ok(mut color_lock) = SEPARATION_COLOR.lock() {
                    *color_lock = color;
                }

                if let Ok(mut icon_lock) = SEPARATION_ICON.lock() {
                    if *icon_lock == 0 {
                        if let Some(icon) = load_icon_from_bytes(ICON_BYTES, 32) {
                            *icon_lock = icon.0 as isize;
                        }
                    }
                }
                
                // Create window
                let hwnd = CreateWindowExW(
                    WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                    windows::core::PCWSTR(class_name_wide.as_ptr()),
                    w!(""),
                    WS_POPUP,
                    x,
                    y,
                    width,
                    height,
                    None,
                    None,
                    Some(HINSTANCE(hinstance.0)),
                    None,
                );

                if let Ok(hwnd) = hwnd {
                    let hwnd_val = hwnd.0 as isize;
                    *SEPARATION_HWND.lock().unwrap() = hwnd_val;
                    
                    // DO NOT exclude from capture - we WANT the blue screen to be visible in shared view
                    // The separation layer should appear in the capture stream so users see it in Meet/Discord
                    // Only the DestinationWindow (preview) and HollowBorder should be excluded
                    log::info!("✅ Separation layer created (will be visible in capture)");
                    
                    // Position at HWND_BOTTOM (above desktop, below everything else)
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_BOTTOM),
                        x,
                        y,
                        width,
                        height,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                    
                    log::info!("✅ Separation layer created at ({}, {}) {}x{} with color 0x{:06X}", x, y, width, height, color);
                    let _ = tx.send(Some(hwnd_val));
                } else {
                    let _ = tx.send(None);
                    return;
                }

                // Message loop
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).into() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        });

        // Wait for window creation
        match rx.recv() {
            Ok(Some(hwnd_val)) => Some(Self { 
                hwnd: hwnd_val,
                thread_handle: Some(thread_handle)
            }),
            _ => None,
        }
    }
    
    /// Update position and size (called when border moves/resizes)
    pub fn update_position(&self, x: i32, y: i32, width: i32, height: i32) {
        // Use SWP_ASYNCWINDOWPOS to prevent blocking if the window thread is busy
        // This decouples the caller (callback thread) from the window thread
        use windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER;
        
        // Remove HWND_BOTTOM to avoid pushing separation layer below the preview window
        // Use SWP_NOZORDER to maintain current z-order (Separation > Preview)
        let result = unsafe {
            SetWindowPos(
                HWND(self.hwnd as *mut _),
                None, // Previously Some(HWND_BOTTOM)
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_ASYNCWINDOWPOS | SWP_NOZORDER,
            )
        };
        
        if result.is_ok() {
            // Log less frequently to avoid spam
            // log::info!("✅ [SEP-LAYER] SetWindowPos succeeded"); 
        } else {
            log::error!("❌ [SEP-LAYER] SetWindowPos FAILED: {:?}", result.err());
        }
    }

    pub fn hwnd_value(&self) -> isize {
        self.hwnd
    }
}

impl Drop for SeparationLayer {
    fn drop(&mut self) {
        unsafe {
            let hwnd_val = self.hwnd;
            // Post WM_CLOSE or DestroyWindow? 
            // Since it's on another thread, we should post WM_CLOSE or quit message
            use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
            
            if hwnd_val != 0 {
                // Request window to close its thread loop
                let _ = PostMessageW(Some(HWND(hwnd_val as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        
        // We can't join the thread easily here because we want to return quickly,
        // but the thread should exit when it processes WM_CLOSE/WM_QUIT
    }
}


impl SeparationLayer {
    /// Show the separation layer
    pub fn show(&self) {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
            let _ = ShowWindow(HWND(self.hwnd as _), SW_SHOWNOACTIVATE);
        }
    }
    
    /// Hide the separation layer
    pub fn hide(&self) {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            let _ = ShowWindow(HWND(self.hwnd as _), SW_HIDE);
        }
    }
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            // Fully handled in WM_PAINT to reduce flicker.
            LRESULT(1)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            if width > 0 && height > 0 {
                let base_color = SEPARATION_COLOR
                    .lock()
                    .ok()
                    .and_then(|c| Some(*c))
                    .filter(|c| *c != 0)
                    .unwrap_or(0x2B78D6);
                let top_color = adjust_color(base_color, 1.12);
                let bottom_color = adjust_color(base_color, 0.82);
                let (top_r, top_g, top_b) = trivertex_color(top_color);
                let (bottom_r, bottom_g, bottom_b) = trivertex_color(bottom_color);

                let vertices = [
                    TRIVERTEX {
                        x: 0,
                        y: 0,
                        Red: top_r,
                        Green: top_g,
                        Blue: top_b,
                        Alpha: 0,
                    },
                    TRIVERTEX {
                        x: width,
                        y: height,
                        Red: bottom_r,
                        Green: bottom_g,
                        Blue: bottom_b,
                        Alpha: 0,
                    },
                ];
                let mesh = GRADIENT_RECT {
                    UpperLeft: 0,
                    LowerRight: 1,
                };
                let _ = GradientFill(
                    hdc,
                    &vertices,
                    &mesh as *const _ as *const core::ffi::c_void,
                    1,
                    GRADIENT_FILL_RECT_V,
                );

                let text_color = blend_color(base_color, 0xFFFFFF, 0.7);
                let text_colorref = platform::colors::rgb_u32_to_colorref(text_color);
                let _ = SetBkMode(hdc, TRANSPARENT);
                let _ = SetTextColor(
                    hdc,
                    windows::Win32::Foundation::COLORREF(text_colorref),
                );

                let mut logfont = LOGFONTW::default();
                logfont.lfHeight = -28;
                logfont.lfWeight = 600;
                logfont.lfEscapement = 330;
                logfont.lfOrientation = 330;
                set_face_name(&mut logfont, "Segoe UI Semibold");
                let font = CreateFontIndirectW(&logfont);
                let old_font = SelectObject(hdc, HGDIOBJ(font.0 as _));

                let text = "RustFrame";
                let text_w: Vec<u16> = text.encode_utf16().collect();

                let icon_handle = SEPARATION_ICON.lock().ok().map(|i| *i).unwrap_or(0);
                let icon_size = 32;
                let step_x = 200;
                let step_y = 160;
                let text_offset_x = icon_size + 10;
                let text_offset_y = 8;

                let mut row = -1;
                let max_row = height / step_y + 1;
                while row <= max_row {
                    let y = row * step_y;
                    let row_offset = (row * step_x) / 2;
                    let mut col = -1;
                    let max_col = width / step_x + 1;
                    while col <= max_col {
                        let x = col * step_x + row_offset;
                        if icon_handle != 0 {
                            let _ = DrawIconEx(
                                hdc,
                                x,
                                y,
                                HICON(icon_handle as _),
                                icon_size,
                                icon_size,
                                0,
                                None,
                                DI_NORMAL,
                            );
                        }
                        let _ = TextOutW(hdc, x + text_offset_x, y + text_offset_y, &text_w);
                        col += 1;
                    }
                    row += 1;
                }

                let _ = SelectObject(hdc, old_font);
                let _ = DeleteObject(HGDIOBJ(font.0 as _));
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => {
            PostQuitMessage(0);

            if let Ok(mut icon_lock) = SEPARATION_ICON.lock() {
                let icon = *icon_lock;
                if icon != 0 {
                    let _ = DestroyIcon(HICON(icon as _));
                    *icon_lock = 0;
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
