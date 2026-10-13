mod egui_app;
mod event_sink;
mod logger;
mod models;
mod native_events;
mod native_plot;
mod native_receive;
mod native_tasks;
mod native_terminal;
mod panic_hook;
mod segger;
mod storage;
mod transport;
mod update;
mod update_apply;
mod update_download;

use fs2::FileExt;
use models::DeviceEntry;
use probe_rs::probe::list::Lister;
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    path::PathBuf,
};

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

struct InstanceAllocation {
    id: u32,
    data_root: PathBuf,
    lock_path: PathBuf,
    _lock: File,
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
            .truncate(false)
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
                _lock: lock,
            });
        }
    }
    Err("已达到最大并行实例数 128".into())
}

fn ensure_instance_lock_is_exclusive(instance_lock_path: &std::path::Path) -> Result<(), String> {
    let lock_directory = instance_lock_path
        .parent()
        .ok_or_else(|| "无法确定实例锁目录".to_string())?;
    let current_name = instance_lock_path
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
        if path == instance_lock_path {
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
pub fn install_panic_hook() {
    panic_hook::init();
}
pub fn run() {
    update_apply::schedule_cleanup();
    egui_app::run();
}
