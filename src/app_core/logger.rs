use chrono::Local;
use parking_lot::Mutex;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_LOG_FILE_SIZE: u64 = 50 * 1024 * 1024;
const MAX_BUFFER_ENTRIES: usize = 10_000;
static GLOBAL_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct Logger {
    buffer: Mutex<Vec<String>>,
    log_dir: PathBuf,
    current_file: Mutex<Option<PathBuf>>,
    dropped_entries: Mutex<u64>,
}

impl Logger {
    pub fn new(log_dir: &Path) -> Self {
        let _ = fs::create_dir_all(log_dir);
        Logger {
            buffer: Mutex::new(Vec::new()),
            log_dir: log_dir.to_path_buf(),
            current_file: Mutex::new(None),
            dropped_entries: Mutex::new(0),
        }
    }

    pub fn log(&self, timestamp: &str, direction: &str, hex_str: &str, ascii_str: &str) {
        let arrow = if direction == "recv" { "←" } else { "→" };
        self.append_entry(format!("[{}] {} {} | {}", timestamp, arrow, hex_str, ascii_str));
    }

    pub fn log_event(&self, text: &str) {
        self.append_entry(format!("[{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), text));
    }

    fn append_entry(&self, entry: String) {
        let mut buffer = self.buffer.lock();
        if buffer.len() >= MAX_BUFFER_ENTRIES {
            buffer.remove(0);
            *self.dropped_entries.lock() += 1;
        }
        buffer.push(entry);
    }

    pub fn flush(&self) {
        let data: Vec<String> = { let mut buffer = self.buffer.lock(); std::mem::take(&mut *buffer) };
        if data.is_empty() { return; }

        let file_path = self.get_or_create_file();
        let drop_count = { let mut dropped = self.dropped_entries.lock(); std::mem::take(&mut *dropped) };

        let mut content = String::new();
        if drop_count > 0 {
            content.push_str(&format!("[{}] *** {} entries dropped ***\n", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), drop_count));
        }
        for entry in &data { content.push_str(entry); content.push('\n'); }

        match OpenOptions::new().create(true).append(true).open(&file_path) {
            Ok(mut file) => {
                if file.write_all(content.as_bytes()).is_err() {
                    let mut buffer = self.buffer.lock();
                    for entry in data.into_iter().rev() { buffer.insert(0, entry); }
                    if buffer.len() > MAX_BUFFER_ENTRIES { buffer.truncate(MAX_BUFFER_ENTRIES); }
                } else if let Ok(metadata) = fs::metadata(&file_path) {
                    if metadata.len() >= MAX_LOG_FILE_SIZE { *self.current_file.lock() = None; }
                }
            }
            Err(_) => {
                let mut buffer = self.buffer.lock();
                for entry in data.into_iter().rev() { buffer.insert(0, entry); }
                if buffer.len() > MAX_BUFFER_ENTRIES { buffer.truncate(MAX_BUFFER_ENTRIES); }
            }
        }
    }

    fn get_or_create_file(&self) -> PathBuf {
        let mut current = self.current_file.lock();
        if let Some(ref path) = *current { return path.clone(); }
        let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = Local::now().format("%Y-%m-%d_%H%M%S").to_string();
        let filename = if counter == 0 { format!("log_{}.txt", timestamp) } else { format!("log_{}_{}.txt", timestamp, counter) };
        let path = self.log_dir.join(filename);
        *current = Some(path.clone());
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_log_creates_entry() {
        let dir = tempdir().unwrap();
        let logger = Logger::new(dir.path());
        logger.log_event("test event");
        let buffer = logger.buffer.lock();
        assert_eq!(buffer.len(), 1);
        assert!(buffer[0].contains("test event"));
    }

    #[test]
    fn test_buffer_overflow() {
        let dir = tempdir().unwrap();
        let logger = Logger::new(dir.path());
        for i in 0..MAX_BUFFER_ENTRIES + 100 { logger.log_event(&format!("entry {}", i)); }
        let buffer = logger.buffer.lock();
        assert_eq!(buffer.len(), MAX_BUFFER_ENTRIES);
        assert_eq!(*logger.dropped_entries.lock(), 100);
    }

    #[test]
    fn test_flush_creates_file() {
        let dir = tempdir().unwrap();
        let logger = Logger::new(dir.path());
        logger.log_event("test");
        logger.flush();
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }
}
