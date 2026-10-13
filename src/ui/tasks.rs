use crate::{
    models::{AppConfig, DeviceEntry, ExtendedSendConfig, SerialSettings},
    storage,
    transport::TransportManager,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) enum Task {
    Scan(AppConfig, PathBuf),
    Save(PathBuf, AppConfig, ExtendedSendConfig),
    Apply(AppConfig, bool),
    CheckSdk(String),
    Import,
    Export(ExtendedSendConfig),
    PickSdk(bool),
}

pub(crate) enum TaskResult {
    Scanned(Vec<DeviceEntry>, Vec<String>, Vec<String>, Vec<String>),
    Saved(Result<(), String>),
    Applied(Result<AppConfig, String>),
    Sdk(Result<String, String>),
    Imported(Result<Option<ExtendedSendConfig>, String>),
    Exported(Result<(), String>),
    SdkPicked(Option<String>),
}

pub(crate) struct Tasks {
    pub sender: crossbeam_channel::Sender<Task>,
    pub results: crossbeam_channel::Receiver<TaskResult>,
}

impl Tasks {
    pub fn new(context: eframe::egui::Context, transport: Arc<Mutex<TransportManager>>) -> Self {
        let (sender, commands) = crossbeam_channel::unbounded();
        let (responses, results) = crossbeam_channel::unbounded();
        std::thread::Builder::new()
            .name("native-tasks".into())
            .spawn(move || {
                while let Ok(task) = commands.recv() {
                    let result = match task {
                        Task::Scan(config, directory) => {
                            let (devices, mut warnings) = crate::collect_devices(
                                config.support_probes,
                                config.show_generic_jtag_adapters,
                                &config.jlink_sdk_path,
                            );
                            let targets = crate::transport::list_custom_probe_targets(
                                &directory.join("probe_rs_targets"),
                            )
                            .unwrap_or_else(|error| {
                                warnings.push(error);
                                Vec::new()
                            });
                            TaskResult::Scanned(devices, warnings, targets, ipv4_addresses())
                        }
                        Task::Save(directory, config, extended) => TaskResult::Saved(
                            storage::save_config(&directory, &config)
                                .and_then(|_| storage::save_extended(&directory, &extended))
                                .map_err(|error| error.to_string()),
                        ),
                        Task::Apply(config, connected_serial) => {
                            let result = if connected_serial {
                                config
                                    .baudrate
                                    .parse::<u32>()
                                    .map_err(|_| "波特率无效".to_string())
                                    .and_then(|baud_rate| {
                                        if (1..=10_000_000).contains(&baud_rate) {
                                            Ok(baud_rate)
                                        } else {
                                            Err("波特率必须在 1 到 10000000 之间".to_string())
                                        }
                                    })
                                    .and_then(|baud_rate| {
                                        transport
                                            .lock()
                                            .unwrap_or_else(|error| error.into_inner())
                                            .reconfigure_serial(SerialSettings {
                                                baud_rate,
                                                data_bits: config.databits,
                                                stop_bits: config.stopbits,
                                                parity: config.parity.clone(),
                                                flow_control: config.flowcontrol.clone(),
                                                frame_timeout: config.frame_timeout,
                                            })
                                    })
                            } else {
                                Ok(())
                            };
                            TaskResult::Applied(result.map(|_| config))
                        }
                        Task::CheckSdk(path) => TaskResult::Sdk(crate::segger::sdk_path(&path)),
                        Task::Import => TaskResult::Imported(
                            rfd::FileDialog::new()
                                .add_filter("JSON", &["json"])
                                .pick_file()
                                .map(|path| {
                                    storage::read_extended_file(&path)
                                        .map_err(|error| error.to_string())
                                })
                                .transpose(),
                        ),
                        Task::Export(extended) => {
                            let result = rfd::FileDialog::new()
                                .add_filter("JSON", &["json"])
                                .set_file_name("extended_send.json")
                                .save_file()
                                .map(|path| {
                                    storage::write_extended_file(&path, &extended)
                                        .map_err(|error| error.to_string())
                                })
                                .unwrap_or(Ok(()));
                            TaskResult::Exported(result)
                        }
                        Task::PickSdk(directory) => {
                            let dialog = rfd::FileDialog::new().set_title("选择 SEGGER J-Link SDK");
                            let path = if directory {
                                dialog.pick_folder()
                            } else {
                                dialog
                                    .add_filter("动态库", &["dll", "so", "dylib"])
                                    .pick_file()
                            };
                            TaskResult::SdkPicked(
                                path.map(|path| path.to_string_lossy().into_owned()),
                            )
                        }
                    };
                    if responses.send(result).is_err() {
                        break;
                    }
                    context.request_repaint();
                }
            })
            .expect("无法创建后台任务线程");
        Self { sender, results }
    }

    pub fn send(&self, task: Task) {
        let _ = self.sender.send(task);
    }
}

fn ipv4_addresses() -> Vec<String> {
    let mut addresses = vec!["0.0.0.0".into(), "127.0.0.1".into()];
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        addresses.extend(
            interfaces
                .into_iter()
                .filter_map(|interface| match interface.addr {
                    if_addrs::IfAddr::V4(address) => Some(address.ip.to_string()),
                    _ => None,
                }),
        );
    }
    addresses.sort();
    addresses.dedup();
    addresses
}
