mod logger;
mod models;
mod segger;
mod storage;
mod transport;

use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Mutex,
};

use fs2::FileExt;
use logger::Logger;
use models::{AppConfig, BootstrapData, ConnectRequest, DeviceEntry, ExtendedSendConfig};
use probe_rs::probe::list::Lister;
use tauri::{AppHandle, Manager, State};
use transport::TransportManager;

pub(crate) struct AppState {
    pub(crate) config_directory: PathBuf,
    instance_id: u32,
    _instance_lock: File,
    config: Mutex<AppConfig>,
    extended: Mutex<ExtendedSendConfig>,
    transport: Mutex<TransportManager>,
    pub(crate) logger: Mutex<Logger>,
}

#[tauri::command]
fn bootstrap(state: State<'_, AppState>) -> Result<BootstrapData, String> {
    Ok(BootstrapData {
        config: state.config.lock().map_err(lock_error)?.clone(),
        extended: state.extended.lock().map_err(lock_error)?.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
        data_directory: state.config_directory.display().to_string(),
        probe_target_directory: state
            .config_directory
            .join("probe_rs_targets")
            .display()
            .to_string(),
        instance_id: state.instance_id,
    })
}

#[tauri::command]
fn list_custom_probe_targets(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    transport::list_custom_probe_targets(&state.config_directory.join("probe_rs_targets"))
}

#[tauri::command]
fn list_devices(state: State<'_, AppState>) -> Vec<DeviceEntry> {
    let (include_probes, show_generic_jtag_adapters) = state
        .config
        .lock()
        .map(|value| (value.support_probes, value.show_generic_jtag_adapters))
        .unwrap_or((true, false));
    let mut devices = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|port| DeviceEntry {
            id: port.port_name.clone(),
            label: port.port_name,
            transport: "serial".into(),
            probe_kind: None,
            serial_number: None,
        })
        .collect::<Vec<_>>();

    if include_probes {
        devices.extend(segger::list_devices().unwrap_or_default());
        devices.extend(Lister::new().list_all().into_iter().filter_map(|probe| {
            if is_jlink_probe(&probe.identifier, &probe.probe_type())
                || (!show_generic_jtag_adapters
                    && is_generic_jtag_adapter(
                        probe.vendor_id,
                        &probe.identifier,
                        &probe.probe_type(),
                    ))
            {
                return None;
            }
            let interface = probe
                .interface
                .map(|value| format!("-{value}"))
                .unwrap_or_default();
            let serial = probe.serial_number.clone().unwrap_or_default();
            let id = format!(
                "{:04x}:{:04x}{interface}:{serial}",
                probe.vendor_id, probe.product_id
            );
            let kind = probe.probe_type();
            let label = if serial.is_empty() {
                format!("{} ({kind})", probe.identifier)
            } else {
                format!("{} ({kind}, SN={serial})", probe.identifier)
            };
            Some(DeviceEntry {
                id,
                label,
                transport: "probe".into(),
                probe_kind: Some(kind),
                serial_number: probe.serial_number,
            })
        }));
    }
    devices
}

fn is_jlink_probe(identifier: &str, kind: &str) -> bool {
    let description = format!("{identifier} {kind}").to_ascii_lowercase();
    description.contains("j-link")
        || description.contains("jlink")
        || description.contains("segger")
}

fn is_generic_jtag_adapter(vendor_id: u16, identifier: &str, kind: &str) -> bool {
    let description = format!("{identifier} {kind}").to_ascii_lowercase();
    vendor_id == 0x0403 || description.contains("ftdi") || description.contains("ft2232")
}

#[cfg(test)]
mod tests {
    use super::{is_generic_jtag_adapter, is_jlink_probe};

    #[test]
    fn separates_jlink_from_probe_rs_devices() {
        assert!(is_jlink_probe("SEGGER J-Link", "J-Link"));
        assert!(!is_jlink_probe("CMSIS-DAP", "CmsisDap"));
    }

    #[test]
    fn hides_ftdi_adapters_by_default() {
        assert!(is_generic_jtag_adapter(
            0x0403,
            "YNUIC USB Serial Converter",
            "FTDI"
        ));
        assert!(!is_generic_jtag_adapter(0x0483, "ST-Link", "STLink"));
    }
}

#[tauri::command]
fn connect_transport(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ConnectRequest,
) -> Result<(), String> {
    state
        .transport
        .lock()
        .map_err(lock_error)?
        .connect(app, request);
    Ok(())
}

#[tauri::command]
fn disconnect_transport(state: State<'_, AppState>) -> Result<(), String> {
    state.transport.lock().map_err(lock_error)?.disconnect();
    Ok(())
}

#[tauri::command]
fn send_bytes(state: State<'_, AppState>, bytes: Vec<u8>) -> Result<(), String> {
    state.transport.lock().map_err(lock_error)?.send(bytes)
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    storage::save_config(&state.config_directory, &config).map_err(|error| error.to_string())?;
    *state.config.lock().map_err(lock_error)? = config;
    Ok(())
}

#[tauri::command]
fn save_extended(state: State<'_, AppState>, extended: ExtendedSendConfig) -> Result<(), String> {
    storage::save_extended(&state.config_directory, &extended)
        .map_err(|error| error.to_string())?;
    *state.extended.lock().map_err(lock_error)? = extended;
    Ok(())
}

#[tauri::command]
fn read_extended_file(path: String) -> Result<ExtendedSendConfig, String> {
    storage::read_extended_file(std::path::Path::new(&path))
        .map_err(|error| format!("读取扩展发送文件失败: {error}"))
}

#[tauri::command]
fn write_extended_file(path: String, extended: ExtendedSendConfig) -> Result<(), String> {
    storage::write_extended_file(std::path::Path::new(&path), &extended)
        .map_err(|error| format!("导出扩展发送文件失败: {error}"))
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    storage::write_text_file(std::path::Path::new(&path), &content)
        .map_err(|error| format!("保存显示内容失败: {error}"))
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("应用状态锁定失败: {error}")
}

struct InstanceAllocation {
    id: u32,
    data_root: PathBuf,
    lock: File,
}

fn allocate_instance() -> Result<InstanceAllocation, String> {
    let executable = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|error| format!("无法确定当前程序路径: {error}"))?;
    let mut hasher = DefaultHasher::new();
    executable.hash(&mut hasher);
    let executable_hash = format!("{:016x}", hasher.finish());
    let executable_root = executable
        .parent()
        .ok_or_else(|| "无法确定当前程序所在目录".to_string())?
        .to_path_buf();
    let lock_directory = executable_root.join("locks");
    fs::create_dir_all(&lock_directory).map_err(|error| format!("无法创建实例锁目录: {error}"))?;

    for id in 1..=128 {
        let lock_path = lock_directory.join(format!("{executable_hash}_instance_{id}.lock"));
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|error| format!("无法打开实例锁文件: {error}"))?;
        if FileExt::try_lock_exclusive(&lock).is_ok() {
            let data_root = if id == 1 {
                executable_root.clone()
            } else {
                executable_root.join(format!("instance_{id}"))
            };
            fs::create_dir_all(&data_root)
                .map_err(|error| format!("无法创建实例数据目录: {error}"))?;
            return Ok(InstanceAllocation {
                id,
                data_root,
                lock,
            });
        }
    }
    Err("已达到最大并行实例数 128".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let instance = allocate_instance()?;
            let config_directory = instance.data_root.join("config");
            fs::create_dir_all(config_directory.join("probe_rs_targets"))
                .map_err(|error| format!("无法创建自定义 MCU 目录: {error}"))?;
            let config = storage::load_config(&config_directory);
            storage::save_config(&config_directory, &config)
                .map_err(|error| format!("无法初始化设置文件: {error}"))?;
            let extended = storage::load_extended(&config_directory);
            storage::save_extended(&config_directory, &extended)
                .map_err(|error| format!("无法初始化扩展发送文件: {error}"))?;
            let logger = Logger::new(&instance.data_root)
                .map_err(|error| format!("无法创建日志: {error}"))?;
            app.manage(AppState {
                config: Mutex::new(config),
                extended: Mutex::new(extended),
                config_directory,
                instance_id: instance.id,
                _instance_lock: instance.lock,
                transport: Mutex::new(TransportManager::default()),
                logger: Mutex::new(logger),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            list_custom_probe_targets,
            list_devices,
            connect_transport,
            disconnect_transport,
            send_bytes,
            save_config,
            save_extended,
            read_extended_file,
            write_extended_file,
            write_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
