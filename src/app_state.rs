use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

use rustframe_capture::capture::CaptureEngine;

use crate::destination_window::DestinationWindow;
use crate::hollow_border::HollowBorder;
use crate::monitors::MonitorInfo;
use crate::rec_indicator::RecIndicator;
use crate::settings::Settings;
#[cfg(target_os = "windows")]
use crate::separation_layer::SeparationLayer;

lazy_static! {
    pub(crate) static ref HOLLOW_BORDER: Mutex<Option<HollowBorder>> = Mutex::new(None);
    pub(crate) static ref DESTINATION_WINDOW: Mutex<Option<DestinationWindow>> = Mutex::new(None);
    pub(crate) static ref REC_INDICATOR: Mutex<Option<RecIndicator>> = Mutex::new(None);
}

#[cfg(target_os = "windows")]
lazy_static! {
    pub(crate) static ref SEPARATION_LAYER: Mutex<Option<SeparationLayer>> = Mutex::new(None);
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) capture_engine: Arc<Mutex<Option<Box<dyn CaptureEngine>>>>,
    pub(crate) settings: Arc<Mutex<Settings>>,
    pub(crate) active_profile: Arc<Mutex<Option<String>>>,
    pub(crate) is_capturing: Arc<Mutex<bool>>,
    #[allow(dead_code)]
    pub(crate) settings_modal_open: Arc<Mutex<bool>>,
    pub(crate) render_thread_stop: Arc<Mutex<bool>>,
    pub(crate) render_thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    pub(crate) monitors: Arc<Mutex<Vec<MonitorInfo>>>,
}
