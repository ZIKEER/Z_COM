use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendItem {
    pub id: u32, pub data: String, pub is_hex: bool,
    pub comment: String, pub delay: u32, pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedSendConfig {
    pub items: Vec<SendItem>,
    #[serde(default)] pub loop_send: bool,
    #[serde(default = "default_true")] pub multi_send: bool,
    #[serde(default = "default_delay")] pub default_delay: u32,
}
fn default_true() -> bool { true }
fn default_delay() -> u32 { 1000 }
impl Default for ExtendedSendConfig {
    fn default() -> Self { ExtendedSendConfig { items: Vec::new(), loop_send: false, multi_send: true, default_delay: 1000 } }
}

pub struct ExtendedSendManager {
    config: Arc<Mutex<ExtendedSendConfig>>,
    path: PathBuf,
    is_sending: Arc<Mutex<bool>>,
    next_id: Arc<Mutex<u32>>,
}

impl ExtendedSendManager {
    pub fn new(config_dir: &Path) -> Self {
        let path = config_dir.join("extended_send.json");
        let config = Self::load_from_file(&path);
        let max_id = config.items.iter().map(|i| i.id).max().unwrap_or(0);
        ExtendedSendManager {
            config: Arc::new(Mutex::new(config)), path,
            is_sending: Arc::new(Mutex::new(false)), next_id: Arc::new(Mutex::new(max_id + 1)),
        }
    }

    fn load_from_file(path: &Path) -> ExtendedSendConfig {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => ExtendedSendConfig::default(),
        }
    }

    pub fn items(&self) -> Vec<SendItem> { self.config.lock().items.clone() }

    pub fn sorted_items(&self) -> Vec<SendItem> {
        let mut items: Vec<SendItem> = self.config.lock().items.iter().filter(|i| i.sort_order > 0).cloned().collect();
        items.sort_by_key(|i| i.sort_order);
        items
    }

    pub fn add_item(&self, data: &str, is_hex: bool, comment: &str, delay: u32) -> u32 {
        let id = { let mut next_id = self.next_id.lock(); let id = *next_id; *next_id += 1; id };
        self.config.lock().items.push(SendItem { id, data: data.to_string(), is_hex, comment: comment.to_string(), delay, sort_order: 0 });
        self.save();
        id
    }

    pub fn remove_item(&self, item_id: u32) {
        self.config.lock().items.retain(|i| i.id != item_id);
        self.save();
    }

    pub fn update_item(&self, item_id: u32, data: Option<&str>, is_hex: Option<bool>, comment: Option<&str>, delay: Option<u32>, sort_order: Option<u32>) {
        let mut config = self.config.lock();
        if let Some(item) = config.items.iter_mut().find(|i| i.id == item_id) {
            if let Some(d) = data { item.data = d.to_string(); }
            if let Some(h) = is_hex { item.is_hex = h; }
            if let Some(c) = comment { item.comment = c.to_string(); }
            if let Some(d) = delay { item.delay = d; }
            if let Some(s) = sort_order { item.sort_order = s; }
        }
        drop(config);
        self.save();
    }

    pub fn clear_items(&self) { self.config.lock().items.clear(); self.save(); }

    pub fn import_from_file(&self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
        let imported: ExtendedSendConfig = serde_json::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
        *self.config.lock() = imported;
        self.save();
        Ok(())
    }

    pub fn export_to_file(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&*self.config.lock()).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(path, json).map_err(|e| format!("Write error: {}", e))
    }

    fn save(&self) {
        let config = self.config.lock().clone();
        if let Some(parent) = self.path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Ok(json) = serde_json::to_string_pretty(&config) { let _ = std::fs::write(&self.path, json); }
    }
}

pub fn decode_ascii_escapes(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('r') => result.push('\r'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('0') => result.push('\0'),
                Some('\\') => result.push('\\'),
                Some('x') => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) { result.push(byte as char); }
                    else { result.push('\\'); result.push('x'); result.push_str(&hex); }
                }
                Some(c) => { result.push('\\'); result.push(c); }
                None => result.push('\\'),
            }
        } else { result.push(c); }
    }
    result
}

pub fn encode_ascii_for_display(text: &str) -> String {
    let mut result = String::new();
    for c in text.chars() {
        match c {
            '\r' => result.push_str("\\r"), '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"), '\0' => result.push_str("\\0"),
            '\\' => result.push_str("\\\\"),
            c if c.is_control() => result.push_str(&format!("\\x{:02X}", c as u8)),
            c => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_ascii_escapes() {
        assert_eq!(decode_ascii_escapes("hello"), "hello");
        assert_eq!(decode_ascii_escapes("a\\r\\nb"), "a\r\nb");
        assert_eq!(decode_ascii_escapes("\\x4F\\x4B"), "OK");
        assert_eq!(decode_ascii_escapes("\\\\"), "\\");
    }

    #[test]
    fn test_encode_ascii_for_display() {
        assert_eq!(encode_ascii_for_display("hello"), "hello");
        assert_eq!(encode_ascii_for_display("a\r\nb"), "a\\r\\nb");
    }

    #[test]
    fn test_manager_crud() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ExtendedSendManager::new(dir.path());
        let id = manager.add_item("test", false, "comment", 100);
        assert_eq!(manager.items().len(), 1);
        manager.update_item(id, Some("updated"), None, None, None, None);
        assert_eq!(manager.items()[0].data, "updated");
        manager.remove_item(id);
        assert_eq!(manager.items().len(), 0);
    }
}
