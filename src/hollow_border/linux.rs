use crate::traits::BorderWindow;
use std::sync::Mutex;

#[derive(Default)]
pub struct HollowBorder {
    rect: Mutex<(i32, i32, i32, i32)>,
    border_width: Mutex<i32>,
}

pub fn set_allow_screen_capture(_allow: bool) {}

pub fn set_border_interaction_complete_callback<F>(_callback: F)
where
    F: Fn(i32, i32, i32, i32) + Send + Sync + 'static,
{
}

pub fn set_border_live_move_callback<F>(_callback: F)
where
    F: Fn(i32, i32, i32, i32) + Send + Sync + 'static,
{
}

pub fn is_border_interacting() -> bool {
    false
}

impl HollowBorder {
    pub fn new(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        border_width: i32,
        _border_color: u32,
    ) -> Option<Self> {
        Some(Self {
            rect: Mutex::new((x, y, width, height)),
            border_width: Mutex::new(border_width),
        })
    }

    pub fn get_rect(&self) -> (i32, i32, i32, i32) {
        *self.rect.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get_inner_rect(&self) -> (i32, i32, i32, i32) {
        let (x, y, w, h) = self.get_rect();
        let bw = *self.border_width.lock().unwrap_or_else(|e| e.into_inner());
        (x + bw, y + bw, (w - 2 * bw).max(1), (h - 2 * bw).max(1))
    }

    pub fn update_rect(&self, x: i32, y: i32, width: i32, height: i32) {
        if let Ok(mut rect) = self.rect.lock() {
            *rect = (x, y, width, height);
        }
    }

    pub fn update_color(&self, _color: u32) {}

    pub fn update_style(&self, width: i32, _color: u32) {
        if let Ok(mut bw) = self.border_width.lock() {
            *bw = width;
        }
    }

    pub fn prime_interaction_from_shortcut(&self) {}

    pub fn hide(&self) {}

    pub fn show(&self) {}

    pub fn hwnd_value(&self) -> isize {
        0
    }

    pub fn set_capture_mode(&mut self) {}

    pub fn set_preview_mode(&mut self) {}

    pub fn stop(&mut self) {}
}

impl BorderWindow for HollowBorder {
    fn new(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        border_width: i32,
        border_color: u32,
    ) -> Option<Self> {
        HollowBorder::new(x, y, width, height, border_width, border_color)
    }

    fn get_rect(&self) -> (i32, i32, i32, i32) {
        self.get_rect()
    }

    fn get_inner_rect(&self) -> (i32, i32, i32, i32) {
        self.get_inner_rect()
    }

    fn update_rect(&self, x: i32, y: i32, width: i32, height: i32) {
        self.update_rect(x, y, width, height);
    }

    fn update_color(&self, color: u32) {
        self.update_color(color);
    }

    fn update_style(&self, width: i32, color: u32) {
        self.update_style(width, color);
    }

    fn hide(&self) {
        self.hide();
    }

    fn show(&self) {
        self.show();
    }

    fn hwnd_value(&self) -> isize {
        self.hwnd_value()
    }

    fn set_capture_mode(&mut self) {
        self.set_capture_mode();
    }

    fn set_preview_mode(&mut self) {
        self.set_preview_mode();
    }

    fn stop(&mut self) {
        self.stop();
    }
}
