mod logger;
mod models;
mod segger;
mod storage;
mod transport;
mod update;
mod update_apply;
mod update_download;

use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Mutex,
    sync::atomic::Ordering,
};

use fs2::FileExt;
use logger::Logger;
use models::{
    AppConfig, BootstrapData, ConnectRequest, DeviceEntry, ExtendedSendConfig, SerialSettings,
};
use probe_rs::probe::list::Lister;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use transport::TransportManager;
use update::{UpdateInfo, UpdateState};
use update_download::DownloadedUpdate;

pub(crate) struct AppState {
    pub(crate) config_directory: PathBuf,
    instance_id: u32,
    instance_lock_path: PathBuf,
    _instance_lock: File,
    config: Mutex<AppConfig>,
    extended: Mutex<ExtendedSendConfig>,
    transport: Mutex<TransportManager>,
    pub(crate) logger: Mutex<Logger>,
    update: Mutex<UpdateState>,
}

#[tauri::command]
fn bootstrap(state: State<'_, AppState>) -> Result<BootstrapData, String> {
    let relative_data_directory = if state.instance_id == 1 {
        "./config".to_string()
    } else {
        format!("./instance_{}/config", state.instance_id)
    };
    Ok(BootstrapData {
        config: state.config.lock().map_err(lock_error)?.clone(),
        extended: state.extended.lock().map_err(lock_error)?.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
        build_timestamp: env!("Z_COM_BUILD_TIMESTAMP").parse().unwrap_or_default(),
        data_directory: relative_data_directory.clone(),
        probe_target_directory: format!("{relative_data_directory}/probe_rs_targets"),
        instance_id: state.instance_id,
        update_notice: update_apply::take_startup_error(),
    })
}

#[tauri::command]
fn open_app_directory(
    app: AppHandle,
    state: State<'_, AppState>,
    directory: String,
) -> Result<(), String> {
    let path = match directory.as_str() {
        "config" => state.config_directory.clone(),
        "probe_targets" => state.config_directory.join("probe_rs_targets"),
        _ => return Err("不支持打开该目录".into()),
    };
    app.opener()
        .open_path(
            dunce::simplified(&path).to_string_lossy().into_owned(),
            None::<String>,
        )
        .map_err(|error| format!("打开目录失败: {error}"))
}

#[tauri::command]
async fn list_custom_probe_targets(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let directory = state.config_directory.join("probe_rs_targets");
    tauri::async_runtime::spawn_blocking(move || transport::list_custom_probe_targets(&directory))
        .await
        .map_err(|error| format!("自定义目标后台任务失败: {error}"))?
}

#[tauri::command]
async fn list_local_ipv4_addresses() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(collect_local_ipv4_addresses)
        .await
        .map_err(|error| format!("网卡枚举后台任务失败: {error}"))
}

fn collect_local_ipv4_addresses() -> Vec<String> {
    let mut addresses = vec!["0.0.0.0".to_string(), "127.0.0.1".to_string()];
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        addresses.extend(
            interfaces
                .into_iter()
                .filter_map(|interface| match interface.addr {
                    if_addrs::IfAddr::V4(address) => Some(address.ip.to_string()),
                    if_addrs::IfAddr::V6(_) => None,
                }),
        );
    }
    addresses.sort_by_key(|address| {
        let priority = match address.as_str() {
            "0.0.0.0" => 0,
            "127.0.0.1" => 1,
            _ => 2,
        };
        (priority, address.clone())
    });
    addresses.dedup();
    addresses
}

#[tauri::command]
async fn list_devices(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<DeviceEntry>, String> {
    let (include_probes, show_generic_jtag_adapters, jlink_sdk_path) = state
        .config
        .lock()
        .map(|value| {
            (
                value.support_probes,
                value.show_generic_jtag_adapters,
                value.jlink_sdk_path.clone(),
            )
        })
        .unwrap_or((true, false, String::new()));
    let (devices, warnings) = tauri::async_runtime::spawn_blocking(move || {
        collect_devices(include_probes, show_generic_jtag_adapters, &jlink_sdk_path)
    })
    .await
    .map_err(|error| format!("设备扫描后台任务失败: {error}"))?;
    for warning in warnings {
        transport::emit(&app, "warning", "system", Vec::new(), warning);
    }
    Ok(devices)
}

#[tauri::command]
async fn check_jlink_sdk(configured_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || segger::sdk_path(&configured_path))
        .await
        .map_err(|error| format!("J-Link SDK 检查后台任务失败: {error}"))?
}

#[tauri::command]
async fn check_for_updates(state: State<'_, AppState>) -> Result<UpdateInfo, String> {
    let result = tauri::async_runtime::spawn_blocking(|| update::check(env!("CARGO_PKG_VERSION")))
        .await
        .map_err(|error| format!("更新检查后台任务失败: {error}"))?;
    let (info, candidate) = match result {
        Ok(result) => result,
        Err(error) => {
            if let Ok(mut logger) = state.logger.lock() {
                logger.log_event(&format!("软件更新检查失败: {error}"));
            }
            return Err(error);
        }
    };
    if !info.source_warning.is_empty()
        && let Ok(mut logger) = state.logger.lock()
    {
        logger.log_event(&format!("软件更新源提醒: {}", info.source_warning));
    }
    let mut update = state.update.lock().map_err(lock_error)?;
    update.candidate = candidate;
    update.downloaded_path = None;
    Ok(info)
}

#[tauri::command]
async fn download_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadedUpdate, String> {
    let candidate = state
        .update
        .lock()
        .map_err(lock_error)?
        .candidate
        .clone()
        .ok_or_else(|| "请先检查更新".to_string())?;
    let cancel = {
        let update = state.update.lock().map_err(lock_error)?;
        update.cancel_download.store(false, Ordering::Relaxed);
        update.cancel_download.clone()
    };
    let download_app = app.clone();
    let candidate_for_download = candidate.clone();
    let (downloaded, path) = tauri::async_runtime::spawn_blocking(move || {
        update_download::download(&download_app, &candidate_for_download, &cancel)
    })
    .await
    .map_err(|error| format!("更新下载后台任务失败: {error}"))??;
    let mut update = state.update.lock().map_err(lock_error)?;
    if update
        .candidate
        .as_ref()
        .is_some_and(|current| current.version == candidate.version)
    {
        update.downloaded_path = Some(path);
    }
    Ok(downloaded)
}

#[tauri::command]
fn cancel_update_download(state: State<'_, AppState>) -> Result<(), String> {
    state
        .update
        .lock()
        .map_err(lock_error)?
        .cancel_download
        .store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("开发模式不允许替换程序，请使用绿色版验证自动更新".into());
    }
    ensure_no_other_instances(&state)?;
    let (candidate, downloaded_path) = {
        let update = state.update.lock().map_err(lock_error)?;
        (
            update
                .candidate
                .clone()
                .ok_or_else(|| "请先检查更新".to_string())?,
            update
                .downloaded_path
                .clone()
                .ok_or_else(|| "请先下载并校验更新".to_string())?,
        )
    };
    state.transport.lock().map_err(lock_error)?.disconnect();
    if let Ok(mut logger) = state.logger.lock() {
        logger.log_event(&format!(
            "开始安装 Z_COM v{}，应用即将重启",
            candidate.version
        ));
    }
    update_apply::launch(&candidate, &downloaded_path)?;
    app.exit(0);
    Ok(())
}

fn collect_devices(
    include_probes: bool,
    show_generic_jtag_adapters: bool,
    jlink_sdk_path: &str,
) -> (Vec<DeviceEntry>, Vec<String>) {
    let (serial_ports, warnings) = match serialport::available_ports() {
        Ok(ports) => (ports, Vec::new()),
        Err(error) => (
            Vec::new(),
            vec![transport::serial_access_error("枚举串口", "", &error)],
        ),
    };
    let mut devices = serial_ports
        .into_iter()
        .map(|port| DeviceEntry {
            id: port.port_name.clone(),
            label: serial_device_label(&port),
            transport: "serial".into(),
            probe_kind: None,
            serial_number: None,
        })
        .collect::<Vec<_>>();

    if include_probes {
        devices.extend(segger::list_devices(jlink_sdk_path).unwrap_or_default());
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
    (devices, warnings)
}

fn serial_device_label(port: &serialport::SerialPortInfo) -> String {
    let description = match &port.port_type {
        serialport::SerialPortType::UsbPort(info) => info
            .product
            .as_deref()
            .and_then(|value| clean_serial_description(value, &port.port_name))
            .or_else(|| {
                info.manufacturer
                    .as_deref()
                    .and_then(|value| clean_serial_description(value, &port.port_name))
            })
            .unwrap_or_else(|| format!("USB {:04X}:{:04X}", info.vid, info.pid)),
        serialport::SerialPortType::BluetoothPort => "蓝牙串口".into(),
        serialport::SerialPortType::PciPort => "PCI 串口".into(),
        serialport::SerialPortType::Unknown => return port.port_name.clone(),
    };
    format!("{}  {description}", port.port_name)
}

fn clean_serial_description(value: &str, port_name: &str) -> Option<String> {
    let value = value.trim();
    let port_suffix = format!(" ({port_name})");
    let value = value
        .get(..value.len().saturating_sub(port_suffix.len()))
        .filter(|_| {
            value
                .to_ascii_lowercase()
                .ends_with(&port_suffix.to_ascii_lowercase())
        })
        .unwrap_or(value)
        .trim_end();
    (!value.is_empty() && !value.eq_ignore_ascii_case(port_name)).then(|| value.to_string())
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
    use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};

    use super::{is_generic_jtag_adapter, is_jlink_probe, serial_device_label};

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

    #[test]
    fn appends_usb_product_to_serial_port_label() {
        let port = SerialPortInfo {
            port_name: "COM3".into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x1a86,
                pid: 0x7523,
                serial_number: None,
                manufacturer: Some("wch.cn".into()),
                product: Some("USB-SERIAL CH340 (COM3)".into()),
                interface: None,
            }),
        };

        assert_eq!(serial_device_label(&port), "COM3  USB-SERIAL CH340");
    }

    #[test]
    fn keeps_unknown_serial_port_label_compact() {
        let port = SerialPortInfo {
            port_name: "/dev/ttyS0".into(),
            port_type: SerialPortType::Unknown,
        };

        assert_eq!(serial_device_label(&port), "/dev/ttyS0");
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
fn reconfigure_serial(state: State<'_, AppState>, settings: SerialSettings) -> Result<(), String> {
    state
        .transport
        .lock()
        .map_err(lock_error)?
        .reconfigure_serial(settings)
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

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("应用状态锁定失败: {error}")
}

struct InstanceAllocation {
    id: u32,
    data_root: PathBuf,
    lock_path: PathBuf,
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
    fs::create_dir_all(&lock_directory).map_err(|error| {
        format!(
            "无法创建实例锁目录 {}: {error}。绿色版程序所在目录必须允许当前用户写入",
            lock_directory.display()
        )
    })?;

    for id in 1..=128 {
        let lock_path = lock_directory.join(format!("{executable_hash}_instance_{id}.lock"));
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
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
                lock_path,
                lock,
            });
        }
    }
    Err("已达到最大并行实例数 128".into())
}

fn ensure_no_other_instances(state: &AppState) -> Result<(), String> {
    let lock_directory = state
        .instance_lock_path
        .parent()
        .ok_or_else(|| "无法确定实例锁目录".to_string())?;
    let current_name = state
        .instance_lock_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "实例锁文件名无效".to_string())?;
    let prefix = current_name
        .split_once("_instance_")
        .map(|(value, _)| value)
        .ok_or_else(|| "实例锁文件名格式无效".to_string())?;
    for entry in
        fs::read_dir(lock_directory).map_err(|error| format!("无法检查其他实例: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("无法读取实例锁目录: {error}"))?
            .path();
        if path == state.instance_lock_path {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.starts_with(prefix) || !name.contains("_instance_") {
            continue;
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("无法检查实例锁 {}: {error}", path.display()))?;
        if FileExt::try_lock_exclusive(&lock).is_err() {
            return Err("检测到其他 Z_COM 实例，请关闭其他实例后再安装更新".into());
        }
    }
    Ok(())
}

pub fn run_update_mode() -> Option<Result<(), String>> {
    update_apply::run_update_mode()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    update_apply::schedule_cleanup();
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
                instance_lock_path: instance.lock_path,
                _instance_lock: instance.lock,
                transport: Mutex::new(TransportManager::default()),
                logger: Mutex::new(logger),
                update: Mutex::new(UpdateState::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            open_app_directory,
            list_custom_probe_targets,
            list_local_ipv4_addresses,
            list_devices,
            check_jlink_sdk,
            check_for_updates,
            download_update,
            cancel_update_download,
            install_update,
            connect_transport,
            disconnect_transport,
            send_bytes,
            reconfigure_serial,
            save_config,
            save_extended,
            read_extended_file,
            write_extended_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
