use crate::app_core::config::{ConfigManager, DisplayMode};
use crate::app_core::logger::Logger;
use crate::app_core::extended_send::ExtendedSendManager;
use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoMode { Serial, Rtt, Socket }

pub struct AppState {
    pub config: ConfigManager,
    pub logger: Logger,
    pub ext_send: ExtendedSendManager,
    pub io_mode: Mutex<IoMode>,
    pub display_mode: Mutex<DisplayMode>,
    pub display_ansi: Mutex<bool>,
    pub send_count: Mutex<u64>,
    pub receive_count: Mutex<u64>,
    pub connected: Mutex<bool>,
}

impl AppState {
    pub fn new(config_dir: &Path, log_dir: &Path) -> Self {
        AppState {
            config: ConfigManager::new(config_dir),
            logger: Logger::new(log_dir),
            ext_send: ExtendedSendManager::new(config_dir),
            io_mode: Mutex::new(IoMode::Serial),
            display_mode: Mutex::new(DisplayMode::Ascii),
            display_ansi: Mutex::new(false),
            send_count: Mutex::new(0),
            receive_count: Mutex::new(0),
            connected: Mutex::new(false),
        }
    }

    pub fn add_send_count(&self, count: u64) { *self.send_count.lock() += count; }
    pub fn add_receive_count(&self, count: u64) { *self.receive_count.lock() += count; }
    pub fn counts(&self) -> (u64, u64) { (*self.send_count.lock(), *self.receive_count.lock()) }
    pub fn set_connected(&self, connected: bool) { *self.connected.lock() = connected; }
    pub fn is_connected(&self) -> bool { *self.connected.lock() }
    pub fn set_display_mode(&self, mode: DisplayMode) { *self.display_mode.lock() = mode; }
    pub fn toggle_ansi(&self) { let mut a = self.display_ansi.lock(); *a = !*a; }
}
