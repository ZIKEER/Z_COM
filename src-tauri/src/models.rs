use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub transport_mode: String,
    pub port: String,
    pub baudrate: String,
    pub databits: u8,
    pub stopbits: f32,
    pub parity: String,
    pub flowcontrol: String,
    pub frame_timeout: u64,
    pub display_mode: String,
    pub send_mode: String,
    pub auto_scroll: bool,
    pub auto_send_interval: u64,
    pub display_ansi: bool,
    #[serde(alias = "support_jlink")]
    pub support_probes: bool,
    pub show_generic_jtag_adapters: bool,
    #[serde(alias = "rtt_chip")]
    pub probe_chip: String,
    #[serde(alias = "rtt_speed")]
    pub probe_speed: u32,
    #[serde(alias = "rtt_reset")]
    pub probe_reset: bool,
    #[serde(alias = "rtt_chip_history")]
    pub probe_chip_history: Vec<String>,
    pub preset_panel_visible: bool,
    pub socket_host: String,
    pub socket_port: u16,
    pub socket_protocol: String,
    pub socket_role: String,
    pub selected_probe: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            transport_mode: "serial".into(),
            port: String::new(),
            baudrate: "115200".into(),
            databits: 8,
            stopbits: 1.0,
            parity: "None".into(),
            flowcontrol: "None".into(),
            frame_timeout: 50,
            display_mode: "ASCII".into(),
            send_mode: "ASCII".into(),
            auto_scroll: true,
            auto_send_interval: 1000,
            display_ansi: false,
            support_probes: true,
            show_generic_jtag_adapters: false,
            probe_chip: String::new(),
            probe_speed: 4000,
            probe_reset: false,
            probe_chip_history: Vec::new(),
            preset_panel_visible: false,
            socket_host: "127.0.0.1".into(),
            socket_port: 8080,
            socket_protocol: "TCP".into(),
            socket_role: "Client".into(),
            selected_probe: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtendedSendConfig {
    pub items: Vec<ExtendedSendItem>,
    pub settings: ExtendedSendSettings,
}

impl Default for ExtendedSendConfig {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            settings: ExtendedSendSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtendedSendItem {
    pub id: u64,
    pub data: String,
    pub is_hex: bool,
    pub comment: String,
    pub delay: u64,
    pub sort_order: u32,
}

impl Default for ExtendedSendItem {
    fn default() -> Self {
        Self {
            id: 0,
            data: String::new(),
            is_hex: false,
            comment: String::new(),
            delay: 1000,
            sort_order: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtendedSendSettings {
    pub loop_send: bool,
    pub multi_send: bool,
    pub default_delay: u64,
}

impl Default for ExtendedSendSettings {
    fn default() -> Self {
        Self {
            loop_send: false,
            multi_send: true,
            default_delay: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceEntry {
    pub id: String,
    pub label: String,
    pub transport: String,
    pub probe_kind: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRequest {
    pub transport: String,
    pub device_id: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: f32,
    pub parity: String,
    pub flow_control: String,
    pub frame_timeout: u64,
    pub socket_host: String,
    pub socket_port: u16,
    pub socket_protocol: String,
    pub socket_role: String,
    pub probe_chip: String,
    pub probe_speed: u32,
    pub probe_reset: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportEvent {
    pub kind: String,
    pub direction: String,
    pub bytes: Vec<u8>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapData {
    pub config: AppConfig,
    pub extended: ExtendedSendConfig,
    pub version: String,
    pub data_directory: String,
    pub probe_target_directory: String,
    pub instance_id: u32,
}
