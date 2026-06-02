use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Parity { None, Even, Odd, Mark, Space }
impl Default for Parity { fn default() -> Self { Parity::None } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FlowControl {
    None,
    #[serde(rename = "RTS/CTS")] RtsCts,
    #[serde(rename = "DTR/DSR")] DtrDsr,
    #[serde(rename = "XON/XOFF")] XonXoff,
}
impl Default for FlowControl { fn default() -> Self { FlowControl::None } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DisplayMode { Ascii, Hex, Mixed }
impl Default for DisplayMode { fn default() -> Self { DisplayMode::Ascii } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SendMode { Ascii, Hex }
impl Default for SendMode { fn default() -> Self { SendMode::Ascii } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)] pub port: String,
    #[serde(default = "default_baudrate")] pub baudrate: String,
    #[serde(default = "default_databits")] pub databits: String,
    #[serde(default = "default_stopbits")] pub stopbits: String,
    #[serde(default)] pub parity: Parity,
    #[serde(default)] pub flowcontrol: FlowControl,
    #[serde(default = "default_frame_timeout")] pub frame_timeout: u32,
    #[serde(default)] pub display_mode: DisplayMode,
    #[serde(default)] pub send_mode: SendMode,
    #[serde(default = "default_true")] pub auto_scroll: bool,
    #[serde(default = "default_auto_send_interval")] pub auto_send_interval: u32,
    #[serde(default)] pub display_ansi: bool,
    #[serde(default)] pub rtt_chip: String,
    #[serde(default = "default_rtt_speed")] pub rtt_speed: u32,
    #[serde(default)] pub rtt_reset: bool,
    #[serde(default)] pub rtt_start_address: String,
    #[serde(default)] pub rtt_range_size: String,
    #[serde(default)] pub rtt_chip_history: Vec<String>,
    #[serde(default = "default_frame_timeout")] pub rtt_frame_timeout: u32,
    #[serde(default = "default_main_splitter")] pub main_splitter_sizes: Vec<i32>,
    #[serde(default = "default_top_splitter")] pub top_splitter_sizes: Vec<i32>,
    #[serde(default)] pub preset_panel_visible: bool,
}

fn default_baudrate() -> String { "115200".to_string() }
fn default_databits() -> String { "8".to_string() }
fn default_stopbits() -> String { "1".to_string() }
fn default_frame_timeout() -> u32 { 50 }
fn default_true() -> bool { true }
fn default_auto_send_interval() -> u32 { 1000 }
fn default_rtt_speed() -> u32 { 4000 }
fn default_main_splitter() -> Vec<i32> { vec![590, 92] }
fn default_top_splitter() -> Vec<i32> { vec![700, 320] }

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            port: String::new(), baudrate: default_baudrate(), databits: default_databits(),
            stopbits: default_stopbits(), parity: Parity::default(), flowcontrol: FlowControl::default(),
            frame_timeout: default_frame_timeout(), display_mode: DisplayMode::default(),
            send_mode: SendMode::default(), auto_scroll: default_true(),
            auto_send_interval: default_auto_send_interval(), display_ansi: false,
            rtt_chip: String::new(), rtt_speed: default_rtt_speed(), rtt_reset: false,
            rtt_start_address: String::new(), rtt_range_size: String::new(),
            rtt_chip_history: Vec::new(), rtt_frame_timeout: default_frame_timeout(),
            main_splitter_sizes: default_main_splitter(), top_splitter_sizes: default_top_splitter(),
            preset_panel_visible: false,
        }
    }
}

pub struct ConfigManager {
    config: Arc<Mutex<AppConfig>>,
    path: PathBuf,
    save_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl ConfigManager {
    pub fn new(config_dir: &Path) -> Self {
        let path = config_dir.join("settings.json");
        let config = Self::load_from_file(&path);
        ConfigManager {
            config: Arc::new(Mutex::new(config)),
            path,
            save_handle: Arc::new(Mutex::new(None)),
        }
    }

    fn load_from_file(path: &Path) -> AppConfig {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    }

    pub fn get(&self) -> AppConfig { self.config.lock().clone() }

    pub fn set(&self, config: AppConfig) {
        *self.config.lock() = config;
        self.save_immediate();
    }

    pub fn update<F: FnOnce(&mut AppConfig)>(&self, updater: F) {
        { updater(&mut self.config.lock()); }
        self.save_debounced();
    }

    fn save_immediate(&self) {
        let config = self.config.lock().clone();
        if let Some(parent) = self.path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Ok(json) = serde_json::to_string_pretty(&config) { let _ = std::fs::write(&self.path, json); }
    }

    fn save_debounced(&self) {
        { let mut handle = self.save_handle.lock(); if let Some(h) = handle.take() { let _ = h; } }
        let config = self.config.lock().clone();
        let path = self.path.clone();
        let save_handle = self.save_handle.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
            if let Ok(json) = serde_json::to_string_pretty(&config) { let _ = std::fs::write(&path, json); }
            *save_handle.lock() = None;
        });
        *self.save_handle.lock() = Some(handle);
    }

    pub fn add_rtt_chip_history(&self, chip: &str) {
        let mut config = self.config.lock();
        config.rtt_chip_history.retain(|c| c != chip);
        config.rtt_chip_history.insert(0, chip.to_string());
        config.rtt_chip_history.truncate(20);
        config.rtt_chip = chip.to_string();
        drop(config);
        self.save_debounced();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.baudrate, "115200");
        assert_eq!(config.frame_timeout, 50);
        assert!(config.auto_scroll);
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let config2: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.baudrate, config2.baudrate);
    }

    #[test]
    fn test_partial_json() {
        let json = r#"{"port": "COM3", "baudrate": "9600"}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, "COM3");
        assert_eq!(config.baudrate, "9600");
        assert_eq!(config.frame_timeout, 50);
    }
}
