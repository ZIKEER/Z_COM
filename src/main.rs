mod app_core;
mod io;
mod ui;
mod version;

use app_core::config::{AppConfig, DisplayMode, SendMode};
use app_core::data_handler;
use app_core::extended_send::{decode_ascii_escapes, SendItem};
use chrono::Local;
use eframe::egui;
use io::serial::SerialTransport;
use io::socket::{Protocol, Role, SocketTransport};
use io::transport::{IOTransport, TransportHandle};
use std::sync::Arc;
use std::time::{Duration, Instant};
use ui::app_state::{AppState, IoMode};

fn main() -> eframe::Result {
    env_logger::init();
    log::info!("{} v{} starting", version::APP_NAME, version::VERSION);

    let config_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("config");
    let log_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("logs");
    let _ = std::fs::create_dir_all(&config_dir);
    let _ = std::fs::create_dir_all(&log_dir);

    let state = Arc::new(AppState::new(&config_dir, &log_dir));
    state.logger.log_event(&format!(
        "{} v{} starting",
        version::APP_NAME, version::VERSION
    ));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Z_COM",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(ZComApp::new(state)))
        }),
    )
}

/// Set up custom fonts (monospace for data display)
fn setup_custom_fonts(_ctx: &egui::Context) {
    // Default fonts include a monospace font suitable for data display
}

// ── Main Application ──

struct ZComApp {
    state: Arc<AppState>,

    // Transport
    transport: Option<Box<dyn IOTransport>>,
    transport_handle: Option<TransportHandle>,

    // UI State - Toolbar
    ports: Vec<String>,
    selected_port: usize,
    baudrates: Vec<String>,
    selected_baud: usize,
    socket_ip: String,
    socket_port: u16,

    // UI State - Receive
    display_lines: Vec<String>,
    auto_scroll: bool,
    max_display_lines: usize,

    // UI State - Send
    send_text: String,
    append_crlf: bool,

    // UI State - Auto send
    auto_send_enabled: bool,
    auto_send_interval: u32,
    last_auto_send: Instant,

    // UI State - Extended send
    ext_items: Vec<SendItem>,
    ext_multi_send: bool,
    ext_loop_send: bool,
    ext_is_sending: bool,
    show_ext_panel: bool,

    // UI State - Dialogs
    show_settings: bool,
    show_about: bool,

    // UI State - Settings
    settings_frame_timeout: u32,
    settings_display_ansi: bool,
    settings_rtt_chip: String,
    settings_rtt_speed: u32,
    settings_rtt_reset: bool,

    // RTT probe selection
    selected_rtt_serial: Option<String>,
}

impl ZComApp {
    fn new(state: Arc<AppState>) -> Self {
        let config = state.config.get();
        let baudrates = vec![
            "9600", "19200", "38400", "57600", "115200", "230400", "460800", "921600",
        ];
        let selected_baud = baudrates
            .iter()
            .position(|&b| b == config.baudrate)
            .unwrap_or(4);
        let ext_items = state.ext_send.items();

        let mut app = ZComApp {
            state,
            transport: None,
            transport_handle: None,
            ports: Vec::new(),
            selected_port: 0,
            baudrates: baudrates.iter().map(|s| s.to_string()).collect(),
            selected_baud,
            socket_ip: "127.0.0.1".to_string(),
            socket_port: 8080,
            display_lines: Vec::new(),
            auto_scroll: config.auto_scroll,
            max_display_lines: 5000,
            send_text: String::new(),
            append_crlf: true,
            auto_send_enabled: false,
            auto_send_interval: config.auto_send_interval,
            last_auto_send: Instant::now(),
            ext_items,
            ext_multi_send: true,
            ext_loop_send: false,
            ext_is_sending: false,
            show_ext_panel: config.preset_panel_visible,
            show_settings: false,
            show_about: false,
            settings_frame_timeout: config.frame_timeout,
            settings_display_ansi: config.display_ansi,
            settings_rtt_chip: config.rtt_chip.clone(),
            settings_rtt_speed: config.rtt_speed,
            settings_rtt_reset: config.rtt_reset,
            selected_rtt_serial: None,
        };

        app.refresh_ports();
        log::info!("[ZComApp::new] Port list initialized with {} ports", app.ports.len());

        app
    }

    /// Try to read incoming data from the transport

    /// Try to read incoming data from the transport
    fn poll_received_data(&mut self) {
        if let Some(ref mut handle) = self.transport_handle {
            let mut all_data = Vec::new();
            while let Ok(data) = handle.rx.try_recv() {
                all_data.extend_from_slice(&data);
            }

            if !all_data.is_empty() {
                self.state.add_receive_count(all_data.len() as u64);

                let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
                let hex_str = data_handler::bytes_to_hex(&all_data);
                let ascii_str = data_handler::bytes_to_ascii(&all_data);
                self.state.logger.log(&timestamp, "recv", &hex_str, &ascii_str);

                let mode = *self.state.display_mode.lock();
                let display = format_display_line(&timestamp, "←", &all_data, mode);
                self.display_lines.push(display);

                // Prune old lines
                if self.display_lines.len() > self.max_display_lines {
                    let drain_count = self.display_lines.len() - self.max_display_lines / 2;
                    self.display_lines.drain(..drain_count);
                }
            }
        }
    }

    /// Send data through the transport
    fn send_data(&mut self) {
        if !self.state.is_connected() || self.send_text.is_empty() {
            return;
        }

        let config = self.state.config.get();
        let data = encode_send_data(&self.send_text, &config.send_mode);
        if data.is_empty() {
            return;
        }

        if let Some(ref mut t) = self.transport {
            match t.send_bytes(&data) {
                Ok(()) => {
                    self.state.add_send_count(data.len() as u64);
                    let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
                    let hex_str = data_handler::bytes_to_hex(&data);
                    let ascii_str = data_handler::bytes_to_ascii(&data);
                    self.state.logger.log(&timestamp, "send", &hex_str, &ascii_str);

                    let mode = *self.state.display_mode.lock();
                    let display = format_display_line(&timestamp, "→", &data, mode);
                    self.display_lines.push(display);
                }
                Err(e) => {
                    self.state
                        .logger
                        .log_event(&format!("Send failed: {}", e));
                }
            }
        }
    }

    /// Toggle connection
    fn toggle_connection(&mut self) {
        if self.state.is_connected() {
            // Disconnect
            if let Some(mut t) = self.transport.take() {
                let _ = t.disconnect();
            }
            self.transport_handle = None;
            self.state.set_connected(false);
            self.state.logger.log_event("Disconnected");
            log::info!("Disconnected");
        } else {
            // Connect
            let config = self.state.config.get();
            let frame_timeout = config.frame_timeout;
            let io_mode = *self.state.io_mode.lock();

            log::info!(
                "Connecting: mode={:?}, port='{}', baud={}, rtt_chip='{}'",
                io_mode, config.port, config.baudrate, config.rtt_chip
            );

            // Check if port is selected
            if config.port.is_empty() {
                let msg = "Error: No port selected".to_string();
                self.state.logger.log_event(&msg);
                self.display_lines.push(msg);
                return;
            }

            // Check RTT chip config
            if io_mode == IoMode::Rtt {
                if config.rtt_chip.is_empty() {
                    let msg = "Error: RTT chip not configured. Go to Settings to set chip model.".to_string();
                    self.state.logger.log_event(&msg);
                    self.display_lines.push(msg);
                    return;
                }
                log::info!("RTT chip: '{}'", config.rtt_chip);
            }

            match self.create_transport(&config, frame_timeout) {
                Ok(mut new_transport) => match new_transport.connect() {
                    Ok(handle) => {
                        self.state.set_connected(true);
                        let info = match io_mode {
                            IoMode::Serial => config.port.clone(),
                            IoMode::Socket => format!("Socket:{}", self.socket_port),
                            IoMode::Rtt => "RTT".to_string(),
                        };
                        let msg = format!("Connected to {}", info);
                        self.state.logger.log_event(&msg);
                        self.display_lines.push(msg);
                        log::info!("Connected to {}", info);
                        self.transport_handle = Some(handle);
                        self.transport = Some(new_transport);
                    }
                    Err(e) => {
                        let msg = format!("Connection failed: {}", e);
                        self.state.logger.log_event(&msg);
                        self.display_lines.push(msg);
                        log::error!("Connection failed: {}", e);
                    }
                },
                Err(e) => {
                    let msg = format!("Error: {}", e);
                    self.state.logger.log_event(&msg);
                    self.display_lines.push(msg);
                    log::error!("Transport error: {}", e);
                }
            }
        }
    }

    fn create_transport(
        &self,
        config: &AppConfig,
        frame_timeout: u32,
    ) -> Result<Box<dyn IOTransport>, String> {
        let io_mode = *self.state.io_mode.lock();
        match io_mode {
            IoMode::Serial => {
                let port = &config.port;
                let baudrate: u32 = config.baudrate.parse().unwrap_or(115200);
                Ok(Box::new(SerialTransport::new(port, baudrate, frame_timeout)))
            }
            IoMode::Socket => {
                let (protocol, role) = match config.port.as_str() {
                    "TCP:Server" | "TCP_SERVER" | "TCP Server" => (Protocol::Tcp, Role::Server),
                    "TCP:Client" | "TCP_CLIENT" | "TCP Client" => (Protocol::Tcp, Role::Client),
                    "UDP:Server" | "UDP_SERVER" | "UDP Server" => (Protocol::Udp, Role::Server),
                    "UDP:Client" | "UDP_CLIENT" | "UDP Client" => (Protocol::Udp, Role::Client),
                    _ => return Err(format!("Unknown socket mode: {}", config.port)),
                };
                Ok(Box::new(SocketTransport::new(
                    protocol,
                    role,
                    &self.socket_ip,
                    self.socket_port,
                    frame_timeout,
                )))
            }
            IoMode::Rtt => {
                log::info!("[create_transport] RTT: chip='{}', serial={:?}", config.rtt_chip, self.selected_rtt_serial);
                Ok(Box::new(io::rtt::RttTransport::new(
                    &config.rtt_chip,
                    config.rtt_speed,
                    config.rtt_reset,
                    self.selected_rtt_serial.as_deref(),
                    frame_timeout,
                )))
            }
        }
    }

    fn refresh_ports(&mut self) {
        let mut all_ports = Vec::new();

        // 1. Serial ports
        let serial = SerialTransport::new("", 115200, 50);
        for d in serial.device_list() {
            all_ports.push(d.display);
        }
        log::info!("[refresh] Found {} serial ports", all_ports.len());

        // 2. J-Link RTT devices
        let rtt = io::rtt::RttTransport::new("", 4000, false, None, 50);
        let rtt_devices = rtt.device_list();
        log::info!("[refresh] Found {} RTT probes", rtt_devices.len());
        for d in &rtt_devices {
            all_ports.push(format!("JLINK:{}", d.display));
        }
        if rtt_devices.is_empty() {
            all_ports.push("--- No J-Link found ---".to_string());
        }

        // 3. Socket modes
        all_ports.push("--- Socket ---".to_string());
        all_ports.push("SOCKET:TCP:Server".to_string());
        all_ports.push("SOCKET:TCP:Client".to_string());
        all_ports.push("SOCKET:UDP:Server".to_string());
        all_ports.push("SOCKET:UDP:Client".to_string());

        log::info!("[refresh_ports] Total ports: {}", all_ports.len());
        self.ports = all_ports;
    }
}

impl eframe::App for ZComApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll received data
        self.poll_received_data();

        // Auto-send logic
        if self.auto_send_enabled
            && self.state.is_connected()
            && self.last_auto_send.elapsed() >= Duration::from_millis(self.auto_send_interval as u64)
        {
            self.send_data();
            self.last_auto_send = Instant::now();
        }

        // Request continuous repaint for real-time updates
        ctx.request_repaint_after(Duration::from_millis(50));

        // ── Top Panel: Toolbar ──
        self.ui_toolbar(ctx);

        // ── Bottom Panel: Status Bar ──
        self.ui_status_bar(ctx);

        // ── Right Panel: Extended Send ──
        if self.show_ext_panel {
            self.ui_extended_panel(ctx);
        }

        // ── Central Panel: Receive + Send ──
        self.ui_central(ctx);

        // ── Dialogs ──
        if self.show_settings {
            self.ui_settings_dialog(ctx);
        }
        if self.show_about {
            self.ui_about_dialog(ctx);
        }
    }
}

// ── UI Components ──

impl ZComApp {
    fn ui_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Refresh button
                if ui.button("🔄").clicked() {
                    self.refresh_ports();
                }

                // Port combo - truncate long names
                let port_label = if self.ports.is_empty() {
                    "No ports".to_string()
                } else if self.selected_port < self.ports.len() {
                    let name = &self.ports[self.selected_port];
                    if name.len() > 30 {
                        format!("{}...", &name[..27])
                    } else {
                        name.clone()
                    }
                } else {
                    "Select port".to_string()
                };
                egui::ComboBox::from_id_salt("port_combo")
                    .width(200.0)
                    .selected_text(&port_label)
                    .show_ui(ui, |ui| {
                        for (i, port) in self.ports.iter().enumerate() {
                            // Skip separator lines
                            if port.starts_with("---") {
                                ui.separator();
                                ui.label(port.trim_matches('-').trim());
                                continue;
                            }
                            if ui
                                .selectable_label(self.selected_port == i, port)
                                .clicked()
                            {
                                self.selected_port = i;
                                // Auto-detect IO mode from port name
                                let io_mode = if port.starts_with("SOCKET:") {
                                    IoMode::Socket
                                } else if port.starts_with("JLINK:") {
                                    IoMode::Rtt
                                } else {
                                    IoMode::Serial
                                };
                                *self.state.io_mode.lock() = io_mode;
                                // Store the raw port identifier in config
                                let port_id = if port.starts_with("SOCKET:") {
                                    port.strip_prefix("SOCKET:").unwrap_or(port).to_string()
                                } else if port.starts_with("JLINK:") {
                                    // Extract serial number from "JLINK:J-Link - 000608888289"
                                    let jlink_info = port.strip_prefix("JLINK:").unwrap_or(port);
                                    let serial = jlink_info.rsplit(" - ").next().unwrap_or("");
                                    self.selected_rtt_serial = Some(serial.to_string());
                                    log::info!("J-Link selected: serial='{}'", serial);
                                    port.clone()
                                } else {
                                    // Extract port name from "COM3 - USB Serial Port"
                                    port.split(" - ").next().unwrap_or(port).to_string()
                                };
                                log::info!("Port selected: '{}' -> io_mode={:?}, port_id='{}'", port, io_mode, port_id);
                                self.state.config.update(|c| c.port = port_id);
                            }
                        }
                    });

                // Baudrate / Socket config
                let io_mode = *self.state.io_mode.lock();

                if io_mode == IoMode::Socket {
                    ui.label("IP:");
                    let ip_resp = ui.text_edit_singleline(&mut self.socket_ip);
                    if ip_resp.changed() {
                        // IP is stored in socket_ip, not in config.port
                    }
                    ui.label("Port:");
                    let mut port_val = self.socket_port as i32;
                    if ui.add(egui::DragValue::new(&mut port_val).range(1..=65535)).changed() {
                        self.socket_port = port_val as u16;
                    }
                } else {
                    ui.label("Baud:");
                    let baud_label = self.baudrates[self.selected_baud].clone();
                    egui::ComboBox::from_id_salt("baud_combo")
                        .width(100.0)
                        .selected_text(&baud_label)
                        .show_ui(ui, |ui| {
                            for (i, baud) in self.baudrates.iter().enumerate() {
                                if ui
                                    .selectable_label(self.selected_baud == i, baud)
                                    .clicked()
                                {
                                    self.selected_baud = i;
                                    self.state
                                        .config
                                        .update(|c| c.baudrate = baud.clone());
                                }
                            }
                        });
                }

                // Spacer
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Preset toggle
                    let preset_text = if self.show_ext_panel { "◀" } else { "▶" };
                    if ui.button(preset_text).clicked() {
                        self.show_ext_panel = !self.show_ext_panel;
                        self.state
                            .config
                            .update(|c| c.preset_panel_visible = self.show_ext_panel);
                    }

                    // Settings button
                    if ui.button("⚙ Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }

                    // Open/Close button
                    let btn_text = if self.state.is_connected() {
                        "Close"
                    } else {
                        "Open"
                    };
                    let btn_color = if self.state.is_connected() {
                        egui::Color32::from_rgb(200, 80, 80)
                    } else {
                        egui::Color32::from_rgb(80, 180, 80)
                    };
                    let btn = egui::Button::new(
                        egui::RichText::new(btn_text).color(egui::Color32::WHITE),
                    )
                    .fill(btn_color);
                    if ui.add(btn).clicked() {
                        self.toggle_connection();
                    }
                });
            });
        });
    }

    fn ui_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Connection indicator
                let (color, text) = if self.state.is_connected() {
                    let config = self.state.config.get();
                    (
                        egui::Color32::from_rgb(0, 180, 0),
                        format!("Connected {}", config.port),
                    )
                } else {
                    (egui::Color32::from_rgb(180, 0, 0), "Disconnected".to_string())
                };
                ui.colored_label(color, "●");
                ui.label(text);

                ui.separator();

                let (tx, rx) = self.state.counts();
                ui.label(format!("TX: {} B", tx));
                ui.separator();
                ui.label(format!("RX: {} B", rx));

                // Menu buttons on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("About").clicked() {
                        // Toggle about dialog
                    }
                    if ui.button("Clear").clicked() {
                        // Clear counters
                    }
                });
            });
        });
    }

    fn ui_central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let send_height = 120.0_f32;
            let receive_height = (available.y - send_height - 8.0).max(100.0);

            // ── Receive Area ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("Receive Area");
                    ui.separator();

                    // Display mode
                    let mode = *self.state.display_mode.lock();
                    if ui
                        .selectable_label(mode == DisplayMode::Ascii, "ASCII")
                        .clicked()
                    {
                        *self.state.display_mode.lock() = DisplayMode::Ascii;
                        self.state
                            .config
                            .update(|c| c.display_mode = DisplayMode::Ascii);
                    }
                    if ui
                        .selectable_label(mode == DisplayMode::Hex, "HEX")
                        .clicked()
                    {
                        *self.state.display_mode.lock() = DisplayMode::Hex;
                        self.state
                            .config
                            .update(|c| c.display_mode = DisplayMode::Hex);
                    }
                    if ui
                        .selectable_label(mode == DisplayMode::Mixed, "MIXED")
                        .clicked()
                    {
                        *self.state.display_mode.lock() = DisplayMode::Mixed;
                        self.state
                            .config
                            .update(|c| c.display_mode = DisplayMode::Mixed);
                    }

                    ui.separator();
                    ui.checkbox(&mut self.auto_scroll, "Auto-scroll");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Clear").clicked() {
                            self.display_lines.clear();
                        }
                    });
                });

                ui.add_space(4.0);

                // Receive text area
                let receive_area = egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(self.auto_scroll)
                    .max_height(receive_height);

                receive_area.show(ui, |ui| {
                    ui.style_mut().override_font_id =
                        Some(egui::FontId::monospace(13.0));
                    for line in &self.display_lines {
                        ui.label(line);
                    }
                });
            });

            ui.add_space(4.0);

            // ── Send Area ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("Send Area");
                    ui.separator();

                    // Send mode
                    let send_mode = self.state.config.get().send_mode;
                    if ui
                        .selectable_label(send_mode == SendMode::Ascii, "ASCII")
                        .clicked()
                    {
                        self.state
                            .config
                            .update(|c| c.send_mode = SendMode::Ascii);
                    }
                    if ui
                        .selectable_label(send_mode == SendMode::Hex, "HEX")
                        .clicked()
                    {
                        self.state
                            .config
                            .update(|c| c.send_mode = SendMode::Hex);
                    }

                    ui.separator();
                    ui.checkbox(&mut self.append_crlf, "CR+LF");

                    ui.separator();
                    ui.checkbox(&mut self.auto_send_enabled, "Auto");

                    let mut interval = self.auto_send_interval as i32;
                    ui.add(
                        egui::DragValue::new(&mut interval)
                            .range(10..=60000)
                            .suffix(" ms"),
                    );
                    if interval != self.auto_send_interval as i32 {
                        self.auto_send_interval = interval as u32;
                        self.state
                            .config
                            .update(|c| c.auto_send_interval = interval as u32);
                    }
                });

                ui.add_space(4.0);

                let send_height = (send_height - 40.0).max(40.0);
                ui.horizontal(|ui| {
                    let text_edit = egui::TextEdit::multiline(&mut self.send_text)
                        .font(egui::FontId::monospace(13.0))
                        .desired_width(ui.available_width() - 70.0)
                        .desired_rows(3)
                        .hint_text("Enter data to send...");
                    let resp = ui.add_sized(
                        [ui.available_width() - 70.0, send_height],
                        text_edit,
                    );

                    // Handle Enter key for sending
                    if resp.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
                    {
                        self.send_data();
                    }

                    let send_btn = egui::Button::new(
                        egui::RichText::new("Send").strong(),
                    )
                    .min_size(egui::vec2(60.0, send_height));
                    if ui.add(send_btn).clicked() {
                        self.send_data();
                    }
                });
            });
        });
    }

    fn ui_extended_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("ext_panel")
            .min_width(260.0)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Extended Send");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+ Add").clicked() {
                            self.state.ext_send.add_item("", false, "", 1000);
                            self.ext_items = self.state.ext_send.items();
                        }
                    });
                });

                ui.separator();

                // Items list
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut remove_id = None;
                        let mut send_id = None;

                        for item in &self.ext_items {
                            ui.horizontal(|ui| {
                                let label = if item.comment.is_empty() {
                                    if item.data.is_empty() {
                                        format!("#{}", item.id)
                                    } else {
                                        item.data.clone()
                                    }
                                } else {
                                    item.comment.clone()
                                };

                                ui.label(
                                    egui::RichText::new(&label)
                                        .monospace()
                                        .size(12.0),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("✕").clicked() {
                                            remove_id = Some(item.id);
                                        }
                                        if ui.small_button("▶").clicked() {
                                            send_id = Some(item.id);
                                        }
                                    },
                                );
                            });
                            ui.add_space(2.0);
                        }

                        if let Some(id) = remove_id {
                            self.state.ext_send.remove_item(id);
                            self.ext_items = self.state.ext_send.items();
                        }
                        if let Some(id) = send_id {
                            self.send_ext_item(id);
                        }
                    });

                ui.separator();

                // Batch controls
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ext_multi_send, "Multi");
                    ui.checkbox(&mut self.ext_loop_send, "Loop");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if self.ext_is_sending {
                            "Stop"
                        } else {
                            "Start"
                        };
                        if ui.button(btn_text).clicked() {
                            self.ext_is_sending = !self.ext_is_sending;
                        }
                    });
                });

                ui.separator();

                // Bottom buttons
                ui.horizontal(|ui| {
                    if ui.button("Clear All").clicked() {
                        self.state.ext_send.clear_items();
                        self.ext_items = Vec::new();
                    }
                    if ui.button("Import").clicked() {
                        // TODO: file dialog
                    }
                    if ui.button("Export").clicked() {
                        // TODO: file dialog
                    }
                });
            });
    }

    fn send_ext_item(&mut self, id: u32) {
        if let Some(item) = self.ext_items.iter().find(|i| i.id == id) {
            let data = if item.is_hex {
                let cleaned: String = item.data.chars().filter(|c| !c.is_whitespace()).collect();
                hex::decode(&cleaned).unwrap_or_default()
            } else {
                decode_ascii_escapes(&item.data).into_bytes()
            };

            if !data.is_empty() {
                if let Some(ref mut t) = self.transport {
                    if t.send_bytes(&data).is_ok() {
                        self.state.add_send_count(data.len() as u64);
                        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
                        let hex_str = data_handler::bytes_to_hex(&data);
                        let ascii_str = data_handler::bytes_to_ascii(&data);
                        self.state.logger.log(&timestamp, "send", &hex_str, &ascii_str);

                        let mode = *self.state.display_mode.lock();
                        let display = format_display_line(&timestamp, "→", &data, mode);
                        self.display_lines.push(display);
                    }
                }
            }
        }
    }

    fn ui_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_settings;
        let mut close_clicked = false;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                // Serial settings
                ui.strong("Serial Settings");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Data bits:");
                    // TODO: combo for data bits
                    ui.label("8");
                    ui.label("Stop bits:");
                    ui.label("1");
                });
                ui.horizontal(|ui| {
                    ui.label("Parity:");
                    ui.label("None");
                    ui.label("Flow control:");
                    ui.label("None");
                });

                ui.separator();

                // Common settings
                ui.strong("Common Settings");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Frame timeout:");
                    let mut timeout = self.settings_frame_timeout as i32;
                    ui.add(egui::DragValue::new(&mut timeout).range(10..=1000).suffix(" ms"));
                    if timeout != self.settings_frame_timeout as i32 {
                        self.settings_frame_timeout = timeout as u32;
                        self.state.config.update(|c| c.frame_timeout = timeout as u32);
                    }
                });
                if ui.checkbox(&mut self.settings_display_ansi, "ANSI color display").changed() {
                    self.state.config.update(|c| c.display_ansi = self.settings_display_ansi);
                }

                ui.separator();

                // RTT settings
                ui.strong("RTT Device Config");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Chip:");
                    let chip_resp = ui.text_edit_singleline(&mut self.settings_rtt_chip);
                    if chip_resp.changed() {
                        self.state.config.update(|c| c.rtt_chip = self.settings_rtt_chip.clone());
                    }
                });
                ui.label(egui::RichText::new("Example: STM32F103C8, nRF52840_xxAA, ESP32").small().color(egui::Color32::GRAY));
                ui.horizontal(|ui| {
                    ui.label("J-Link speed:");
                    let mut speed = self.settings_rtt_speed as i32;
                    ui.add(egui::DragValue::new(&mut speed).range(100..=50000).suffix(" kHz"));
                    if speed != self.settings_rtt_speed as i32 {
                        self.settings_rtt_speed = speed as u32;
                        self.state.config.update(|c| c.rtt_speed = speed as u32);
                    }
                });
                if ui.checkbox(&mut self.settings_rtt_reset, "Reset target on connect").changed() {
                    self.state.config.update(|c| c.rtt_reset = self.settings_rtt_reset);
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                });
            });
        if close_clicked {
            open = false;
        }
        self.show_settings = open;
    }

    fn ui_about_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_about;
        let mut close_clicked = false;
        egui::Window::new("About")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Z_COM");
                    ui.label(format!("Version {}", version::VERSION));
                    ui.label(format!("Built: {}", version::BUILD_TIME));
                    ui.add_space(8.0);
                    ui.label("Multi-protocol serial communication debugging tool");
                    ui.add_space(4.0);
                    ui.label("• Serial Port (pyserial compatible)");
                    ui.label("• TCP/UDP Socket (Server/Client)");
                    ui.label("• J-Link RTT (via probe-rs)");
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                });
            });
        if close_clicked {
            open = false;
        }
        self.show_about = open;
    }
}

// ── Helper functions ──

fn encode_send_data(text: &str, mode: &SendMode) -> Vec<u8> {
    match mode {
        SendMode::Hex => {
            let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            hex::decode(&cleaned).unwrap_or_default()
        }
        SendMode::Ascii => decode_ascii_escapes(text).into_bytes(),
    }
}

fn format_display_line(timestamp: &str, direction: &str, data: &[u8], mode: DisplayMode) -> String {
    let dh_mode = match mode {
        DisplayMode::Ascii => data_handler::DisplayMode::Ascii,
        DisplayMode::Hex => data_handler::DisplayMode::Hex,
        DisplayMode::Mixed => data_handler::DisplayMode::Mixed,
    };
    let data_str = data_handler::format_display(data, dh_mode);
    format!("[{}] {} {}", timestamp, direction, data_str)
}
