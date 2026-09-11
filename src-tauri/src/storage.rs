use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::models::{AppConfig, ExtendedSendConfig};

pub fn load_config(dir: &Path) -> AppConfig {
    load_json(&dir.join("settings.json")).unwrap_or_default()
}

pub fn load_extended(dir: &Path) -> ExtendedSendConfig {
    load_json(&dir.join("extended_send.json")).unwrap_or_default()
}

pub fn save_config(dir: &Path, value: &AppConfig) -> io::Result<()> {
    atomic_write_json(&dir.join("settings.json"), value)
}

pub fn save_extended(dir: &Path, value: &ExtendedSendConfig) -> io::Result<()> {
    atomic_write_json(&dir.join("extended_send.json"), value)
}

pub fn read_extended_file(path: &Path) -> io::Result<ExtendedSendConfig> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

pub fn write_extended_file(path: &Path, value: &ExtendedSendConfig) -> io::Result<()> {
    atomic_write_json(path, value)
}

fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path);
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp)?;
    use io::Write;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    replace_file(&tmp, path)
}

fn temp_path(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut value = path.as_os_str().to_os_string();
    value.push(format!(
        ".tmp-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    PathBuf::from(value)
}

fn replace_file(source: &Path, target: &Path) -> io::Result<()> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(target)?;
            fs::rename(source, target)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_jlink_keys_are_accepted() {
        let json = r#"{"support_jlink":true,"rtt_chip":"nRF52840_xxAA","rtt_speed":4000}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(config.support_probes);
        assert_eq!(config.probe_chip, "nRF52840_xxAA");
    }
}
