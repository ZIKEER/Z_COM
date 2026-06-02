use super::app_state::AppState;
use crate::app_core::config::DisplayMode;
use std::sync::Arc;

pub struct Handlers { state: Arc<AppState> }

impl Handlers {
    pub fn new(state: Arc<AppState>) -> Self { Handlers { state } }

    pub fn refresh_ports(&self) -> Vec<String> { vec!["COM1".to_string(), "COM2".to_string()] }

    pub fn toggle_connection(&self) -> Result<(), String> {
        if self.state.is_connected() {
            self.state.set_connected(false);
            self.state.logger.log_event("Disconnected");
        } else {
            self.state.set_connected(true);
            self.state.logger.log_event("Connected");
        }
        Ok(())
    }

    pub fn send_data(&self, data: &[u8]) -> Result<(), String> {
        if !self.state.is_connected() { return Err("Not connected".to_string()); }
        self.state.add_send_count(data.len() as u64);
        Ok(())
    }

    pub fn set_display_mode(&self, mode: DisplayMode) { self.state.set_display_mode(mode); }
    pub fn toggle_ansi(&self) { self.state.toggle_ansi(); }
    pub fn get_counts(&self) -> (u64, u64) { self.state.counts() }
}
