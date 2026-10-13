use crate::{
    event_sink::EventSink,
    logger::Logger,
    models::{AppConfig, ConnectRequest, DeviceEntry, ExtendedSendConfig, ExtendedSendItem},
    native_tasks::{Task, TaskResult, Tasks},
    storage,
    transport::TransportManager,
};
use eframe::egui::{self, Color32};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub fn run() {
    let icon = image::load_from_memory(include_bytes!("../icons/128x128.png"))
        .ok()
        .map(|image| {
            let image = image.into_rgba8();
            egui::IconData {
                width: image.width(),
                height: image.height(),
                rgba: image.into_raw(),
            }
        });
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Z_COM")
        .with_inner_size([1080.0, 760.0])
        .with_min_inner_size([760.0, 560.0]);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "Z_COM",
        options,
        Box::new(|cc| {
            App::new(cc)
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|error| {
                    Box::new(std::io::Error::other(error))
                        as Box<dyn std::error::Error + Send + Sync>
                })
        }),
    ) {
        eprintln!("Z_COM: {error}");
        rfd::MessageDialog::new()
            .set_title("Z_COM 启动失败")
            .set_description(error.to_string())
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}

struct App {
    plot: crate::native_plot::NativePlot,
    config: AppConfig,
    extended: ExtendedSendConfig,
    directory: PathBuf,
    instance: Option<crate::InstanceAllocation>,
    transport: Arc<Mutex<TransportManager>>,
    events: crate::native_events::Inbox,
    tasks: Tasks,
    scanning: bool,
    targets: Vec<String>,
    addresses: Vec<String>,
    sdk_status: String,
    live_baud_draft: String,
    counter_baseline: crate::native_events::Counters,
    sink: EventSink,
    devices: Vec<DeviceEntry>,
    receive: VecDeque<String>,
    receive_view: crate::native_receive::ReceiveView,
    send_text: String,
    connected: bool,
    connecting: Option<Instant>,
    sent: u64,
    received: u64,
    status: String,
    error: String,
    settings_open: bool,
    draft: AppConfig,
    about_open: bool,
    legal_open: bool,
    legal_tab: usize,
    baud_open: bool,
    baud_error: String,
    logo: egui::TextureHandle,
    preview_mode: bool,
    crlf: bool,
    auto_send: bool,
    next_auto: Instant,
    dirty: Option<Instant>,
    batch: Vec<ExtendedSendItem>,
    batch_index: usize,
    batch_due: Instant,
    update_info: Option<crate::update::UpdateInfo>,
    update_candidate: Option<crate::update::ResolvedUpdate>,
    update_path: Option<PathBuf>,
    update_events: crossbeam_channel::Receiver<UpdateEvent>,
    update_sender: crossbeam_channel::Sender<UpdateEvent>,
    update_busy: bool,
    update_status: String,
    update_cancel: Arc<std::sync::atomic::AtomicBool>,
}

enum UpdateEvent {
    Checked(
        Result<
            (
                crate::update::UpdateInfo,
                Option<crate::update::ResolvedUpdate>,
            ),
            String,
        >,
    ),
    Progress(u8, String),
    Downloaded(Result<PathBuf, String>),
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        cc.egui_ctx.set_theme(egui::Theme::Light);
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let mut fonts = egui::FontDefinitions::default();
        let mut database = fontdb::Database::new();
        database.load_system_fonts();
        let face = database.query(&fontdb::Query {
            families: &[
                fontdb::Family::Name("SimSun"),
                fontdb::Family::Name("Songti SC"),
                fontdb::Family::Name("Noto Serif CJK SC"),
                fontdb::Family::Name("Microsoft YaHei UI"),
                fontdb::Family::Name("Microsoft YaHei"),
                fontdb::Family::Name("PingFang SC"),
                fontdb::Family::Name("Noto Sans CJK SC"),
                fontdb::Family::SansSerif,
            ],
            ..Default::default()
        });
        if let Some(face) = face {
            database.with_face_data(face, |data, index| {
                let mut font = egui::FontData::from_owned(data.to_vec());
                font.index = index;
                fonts.font_data.insert("cjk".into(), Arc::new(font));
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "cjk".into());
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .push("cjk".into());
            });
        }
        if cfg!(target_os = "windows") {
            let font_directory =
                PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:/Windows".into()))
                    .join("Fonts");
            if let Ok(data) = std::fs::read(font_directory.join("simsun.ttc")) {
                fonts
                    .font_data
                    .insert("song".into(), Arc::new(egui::FontData::from_owned(data)));
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts.families.insert(family, vec!["song".into()]);
                }
            }
        }
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.style_mut(|style| {
            for text_style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                style
                    .text_styles
                    .insert(text_style, egui::FontId::proportional(13.0));
            }
            style.spacing.item_spacing = egui::vec2(5.0, 4.0);
            style.spacing.button_padding = egui::vec2(7.0, 4.0);
            style.spacing.interact_size.y = 26.0;
            style.spacing.button_padding.y = 6.0;
            style.visuals.widgets.inactive.bg_fill = Color32::WHITE;
            style.visuals.widgets.inactive.weak_bg_fill = Color32::WHITE;
            style.visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(1.0_f32, Color32::from_gray(185));
            style.visuals.panel_fill = Color32::from_rgb(245, 245, 245);
            style.visuals.selection.bg_fill = Color32::from_rgb(225, 225, 225);
            style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(119));
            style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(229, 241, 251);
            for widget in [
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
            ] {
                widget.corner_radius = egui::CornerRadius::ZERO;
            }
            style.visuals.override_text_color = Some(Color32::from_rgb(32, 32, 32));
        });
        let image = image::load_from_memory(include_bytes!("../icons/128x128.png"))
            .map_err(|error| error.to_string())?
            .into_rgba8();
        let logo = cc.egui_ctx.load_texture(
            "app-logo",
            egui::ColorImage::from_rgba_unmultiplied([128, 128], image.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        let instance = Some(crate::allocate_instance()?);
        let root = instance
            .as_ref()
            .map(|value| value.data_root.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        let directory = root.join("config");
        crate::panic_hook::set_log_directory(root.join("logs"));
        let config = storage::load_config(&directory);
        let extended = storage::load_extended(&directory);
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                config.window_width as f32,
                config.window_height as f32,
            )));
        let logger = Arc::new(Mutex::new(Some(
            Logger::new(&root).map_err(|error| format!("无法创建通信日志: {error}"))?,
        )));
        let events = crate::native_events::Inbox::default();
        let sender = events.clone();
        let context = cc.egui_ctx.clone();
        let sink = EventSink::new(directory.clone(), move |event| {
            if let Ok(mut logger) = logger.lock() {
                if let Some(logger) = logger.as_mut() {
                    if event.kind == "data" {
                        logger.log_data(&event.direction, &event.bytes);
                    } else {
                        logger.log_event(&event.message);
                    }
                }
            }
            sender.push(event);
            context.request_repaint_after(Duration::from_millis(33));
        });
        let transport = Arc::new(Mutex::new(TransportManager::default()));
        let tasks = Tasks::new(cc.egui_ctx.clone(), transport.clone());
        let (update_sender, update_events) = crossbeam_channel::unbounded();
        let mut app = Self {
            logo,
            preview_mode: false,
            legal_open: false,
            legal_tab: 0,
            baud_open: false,
            baud_error: String::new(),
            tasks,
            scanning: false,
            targets: Vec::new(),
            addresses: Vec::new(),
            sdk_status: String::new(),
            live_baud_draft: config.baudrate.clone(),
            counter_baseline: Default::default(),
            update_info: None,
            update_candidate: None,
            update_path: None,
            update_events,
            update_sender,
            update_busy: false,
            update_status: "可手动检查 GitHub / Gitee 新版本".into(),
            update_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            plot: crate::native_plot::NativePlot::default(),
            draft: config.clone(),
            config,
            extended,
            directory,
            instance,
            transport,
            events,
            sink,
            devices: Vec::new(),
            receive: VecDeque::new(),
            receive_view: Default::default(),
            send_text: String::new(),
            connected: false,
            connecting: None,
            sent: 0,
            received: 0,
            status: "未连接".into(),
            error: String::new(),
            settings_open: false,
            about_open: false,
            crlf: false,
            auto_send: false,
            next_auto: Instant::now(),
            dirty: None,
            batch: Vec::new(),
            batch_index: 0,
            batch_due: Instant::now(),
        };
        if app
            .instance
            .as_ref()
            .is_some_and(|instance| instance.id > 1)
        {
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "Z_COM - 实例 {}",
                    app.instance.as_ref().unwrap().id
                )));
        }
        app.refresh_devices();
        app.error = crate::update_apply::take_startup_error();
        app.plot.template =
            std::fs::read_to_string(app.directory.join("plot-template.txt")).unwrap_or_default();
        let _ = app.plot.compile();
        if cfg!(debug_assertions) {
            if let Some(preview) = std::env::args()
                .find_map(|argument| argument.strip_prefix("--preview=").map(str::to_owned))
            {
                app.preview_mode = true;
                match preview.as_str() {
                    "about" => app.about_open = true,
                    "update" => {
                        app.about_open = true;
                        app.update_busy = true;
                        let sender = app.update_sender.clone();
                        let context = cc.egui_ctx.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(UpdateEvent::Checked(crate::update::check(env!(
                                "CARGO_PKG_VERSION"
                            ))));
                            context.request_repaint();
                        });
                    }
                    "settings" => app.settings_open = true,
                    "legal" => app.legal_open = true,
                    "baud" => {
                        app.baud_open = true;
                        app.live_baud_draft = app.config.baudrate.clone();
                    }
                    "plot" | "rules" => app.plot.preview(preview == "rules"),
                    "extended" => {
                        app.config.preset_panel_visible = true;
                        app.extended.items = vec![
                            ExtendedSendItem {
                                id: 1,
                                data: "01 03 00 00 00 01".into(),
                                comment: "读取寄存器".into(),
                                is_hex: true,
                                ..Default::default()
                            },
                            ExtendedSendItem {
                                id: 2,
                                data: "temp=25\\r\\n".into(),
                                comment: "发送文本".into(),
                                ..Default::default()
                            },
                        ];
                    }
                    "serial" | "socket" | "probe" => app.config.transport_mode = preview,
                    _ => {}
                }
            }
        }
        Ok(app)
    }

    fn refresh_devices(&mut self) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        self.tasks
            .send(Task::Scan(self.config.clone(), self.directory.clone()));
    }

    fn connect(&mut self) {
        if self.connected || self.connecting.is_some() {
            self.transport
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .disconnect();
            self.connecting = None;
            return;
        }
        let baud = match self.config.baudrate.parse::<u32>() {
            Ok(baud) if (1..=10_000_000).contains(&baud) => baud,
            _ => {
                self.error = "波特率必须为 1 - 10000000".into();
                return;
            }
        };
        let request = ConnectRequest {
            transport: self.config.transport_mode.clone(),
            device_id: if self.config.transport_mode == "probe" {
                self.config.selected_probe.clone()
            } else {
                self.config.port.clone()
            },
            baud_rate: baud,
            data_bits: self.config.databits,
            stop_bits: self.config.stopbits,
            parity: self.config.parity.clone(),
            flow_control: self.config.flowcontrol.clone(),
            frame_timeout: self.config.frame_timeout,
            socket_host: self.config.socket_host.clone(),
            socket_port: self.config.socket_port,
            socket_protocol: self.config.socket_protocol.clone(),
            socket_role: self.config.socket_role.clone(),
            probe_chip: self.config.probe_chip.clone(),
            probe_speed: self.config.probe_speed,
            probe_reset: self.config.probe_reset,
            jlink_sdk_path: self.config.jlink_sdk_path.clone(),
        };
        match self
            .transport
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .connect(self.sink.clone(), request)
        {
            Ok(()) => {
                if self.config.transport_mode == "serial" {
                    self.config
                        .baudrate_history
                        .retain(|value| value != &self.config.baudrate);
                    self.config
                        .baudrate_history
                        .insert(0, self.config.baudrate.clone());
                    self.config.baudrate_history.truncate(12);
                }
                if self.config.transport_mode == "probe" && !self.config.probe_chip.is_empty() {
                    self.config
                        .probe_chip_history
                        .retain(|value| value != &self.config.probe_chip);
                    self.config
                        .probe_chip_history
                        .insert(0, self.config.probe_chip.clone());
                    self.config.probe_chip_history.truncate(30);
                }
                self.dirty = Some(Instant::now());
                self.connecting = Some(Instant::now());
                self.status = "连接中".into();
            }
            Err(error) => self.error = error,
        }
    }

    fn send(&mut self, data: &str, hex: bool, escapes: bool, crlf: bool) {
        if !self.connected {
            self.error = "请先建立连接".into();
            return;
        }
        match encode(data, hex, escapes, crlf).and_then(|bytes| {
            self.transport
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .send(bytes)
        }) {
            Ok(()) => {}
            Err(error) => {
                self.error = error;
                self.auto_send = false;
                self.batch.clear();
            }
        }
    }

    fn append(&mut self, text: String) {
        for line in text.lines() {
            self.receive.push_back(line.to_owned());
            while self.receive.len() > 5000 {
                self.receive.pop_front();
                self.receive_view.trimmed();
            }
        }
    }

    fn handle_events(&mut self) {
        let deadline = Instant::now() + Duration::from_millis(8);
        while Instant::now() < deadline {
            let Some(timed) = self.events.pop() else {
                break;
            };
            let event = timed.event;
            match event.kind.as_str() {
                "connected" => {
                    self.live_baud_draft = self.config.baudrate.clone();
                    self.connected = true;
                    self.connecting = None;
                    self.status = event.message.clone();
                }
                "disconnected" => {
                    self.connected = false;
                    self.connecting = None;
                    self.auto_send = false;
                    self.batch.clear();
                    self.status = "已断开".into();
                }
                "error" => {
                    self.error = event.message.clone();
                }
                _ => {}
            }
            let stamp = timed.timestamp;
            if event.kind == "data" {
                if event.direction == "received" {
                    self.plot.consume(&event.bytes);
                }
                let arrow = if event.direction == "received" {
                    "←"
                } else {
                    "→"
                };
                let hex = || {
                    event
                        .bytes
                        .iter()
                        .map(|byte| format!("{byte:02X}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                let ascii = || display_ascii(&event.bytes);
                let text = match self.config.display_mode.as_str() {
                    "HEX" => format!("[{stamp}] {arrow} HEX: {}", hex()),
                    "MIXED" => format!(
                        "[{stamp}]\n{arrow} HEX: {}\n{arrow} ASCII: {}",
                        hex(),
                        ascii()
                    ),
                    _ => format!("[{stamp}] {arrow} ASCII: {}", ascii()),
                };
                self.append(text);
            } else if !event.message.is_empty() {
                self.append(format!("[{stamp}] ◆ {}", event.message));
            }
        }
        let counters = self.events.counters();
        self.sent = counters.sent.saturating_sub(self.counter_baseline.sent);
        self.received = counters
            .received
            .saturating_sub(self.counter_baseline.received);
    }

    fn persist(&mut self) {
        if self.preview_mode {
            self.dirty = None;
            return;
        }
        self.tasks.send(Task::Save(
            self.directory.clone(),
            self.config.clone(),
            self.extended.clone(),
        ));
        self.dirty = None;
    }

    fn start_batch(&mut self) {
        if !self.batch.is_empty() {
            self.batch.clear();
            return;
        }
        if !self.connected {
            self.error = "请先建立连接".into();
            return;
        }
        let mut items: Vec<_> = self
            .extended
            .items
            .iter()
            .filter(|item| item.sort_order > 0)
            .cloned()
            .collect();
        items.sort_by_key(|item| item.sort_order);
        if items.is_empty() {
            self.error = "没有序号大于 0 的扩展发送条目".into();
            return;
        }
        if items
            .windows(2)
            .any(|items| items[0].sort_order == items[1].sort_order)
        {
            self.error = "发送序号重复".into();
            return;
        }
        if !self.extended.settings.multi_send {
            items.truncate(1);
        }
        self.batch = items;
        self.batch_index = 0;
        self.batch_due = Instant::now();
    }

    fn connection_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.connected && self.connecting.is_none(), |ui| {
                control_group(ui, |ui| {
                    for (value, label) in [
                        ("serial", "串口"),
                        ("socket", "Socket"),
                        ("probe", "调试探针 / RTT"),
                    ] {
                        segment(ui, &mut self.config.transport_mode, value.into(), label);
                    }
                });
                if ui
                    .add_enabled_ui(!self.scanning, |ui| {
                        icon_button(ui, "refresh", "刷新设备", false)
                    })
                    .inner
                    .clicked()
                {
                    self.refresh_devices();
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, "expand", "扩展发送", false).clicked() {
                    self.config.preset_panel_visible = !self.config.preset_panel_visible;
                }
                if icon_button(ui, "help", "关于 Z_COM", false).clicked() {
                    self.about_open = true;
                }
                if icon_button(ui, "settings", "更多设置", false).clicked() {
                    self.draft = self.config.clone();
                    self.settings_open = true;
                    self.tasks
                        .send(Task::CheckSdk(self.draft.jlink_sdk_path.clone()));
                }
                let label = if self.connected {
                    "断开"
                } else if self.connecting.is_some() {
                    "取消"
                } else {
                    "连接"
                };
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(label).color(Color32::WHITE))
                            .fill(Color32::from_rgb(244, 67, 54)),
                    )
                    .clicked()
                {
                    self.connect();
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!self.connected && self.connecting.is_none(), |ui| {
                if self.config.transport_mode == "socket" {
                    choices(
                        ui,
                        "协议",
                        &mut self.config.socket_protocol,
                        &["TCP", "UDP"],
                    );
                    choices(
                        ui,
                        "角色",
                        &mut self.config.socket_role,
                        &["Client", "Server"],
                    );
                    ui.label("地址");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.config.socket_host)
                            .desired_width(150.0),
                    );
                    if self.config.socket_role == "Server" {
                        egui::ComboBox::from_id_salt("local_addresses")
                            .selected_text("本机地址")
                            .show_ui(ui, |ui| {
                                for address in &self.addresses {
                                    ui.selectable_value(
                                        &mut self.config.socket_host,
                                        address.clone(),
                                        address,
                                    );
                                }
                            });
                    }
                    ui.label("端口");
                    ui.add(egui::DragValue::new(&mut self.config.socket_port).range(1..=65535));
                } else {
                    let target = if self.config.transport_mode == "probe" {
                        &mut self.config.selected_probe
                    } else {
                        &mut self.config.port
                    };
                    ui.label("设备");
                    egui::ComboBox::from_id_salt("device")
                        .width(235.0)
                        .selected_text(
                            self.devices
                                .iter()
                                .find(|item| item.id == *target)
                                .map(|item| item.label.as_str())
                                .unwrap_or(target),
                        )
                        .show_ui(ui, |ui| {
                            for device in self
                                .devices
                                .iter()
                                .filter(|item| item.transport == self.config.transport_mode)
                            {
                                ui.selectable_value(target, device.id.clone(), &device.label);
                            }
                        });
                    if self.config.transport_mode == "serial" {
                        ui.label("波特率");
                        egui::ComboBox::from_id_salt("baud_history")
                            .width(115.0)
                            .selected_text(&self.config.baudrate)
                            .show_ui(ui, |ui| {
                                for baud in [
                                    "1200", "2400", "4800", "9600", "19200", "38400", "57600",
                                    "115200", "230400", "460800", "921600", "1000000", "2000000",
                                    "3000000",
                                ] {
                                    ui.selectable_value(
                                        &mut self.config.baudrate,
                                        baud.into(),
                                        baud,
                                    );
                                }
                                for baud in &self.config.baudrate_history {
                                    ui.selectable_value(
                                        &mut self.config.baudrate,
                                        baud.clone(),
                                        baud,
                                    );
                                }
                                if ui.button("自定义波特率…").clicked() {
                                    self.live_baud_draft.clear();
                                    self.baud_error.clear();
                                    self.baud_open = true;
                                }
                            });
                    } else {
                        ui.label("目标芯片");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.probe_chip)
                                .desired_width(180.0),
                        );
                        egui::ComboBox::from_id_salt("target_history")
                            .selected_text("历史 / 自定义")
                            .show_ui(ui, |ui| {
                                for target in self
                                    .config
                                    .probe_chip_history
                                    .iter()
                                    .chain(self.targets.iter())
                                {
                                    ui.selectable_value(
                                        &mut self.config.probe_chip,
                                        target.clone(),
                                        target,
                                    );
                                }
                            });
                        ui.checkbox(&mut self.config.probe_reset, "连接后复位");
                    }
                }
            });
            if self.connected && self.config.transport_mode == "serial" {
                ui.label("波特率");
                let response = ui
                    .add(egui::TextEdit::singleline(&mut self.live_baud_draft).desired_width(95.0));
                if response.lost_focus() {
                    let value = self.live_baud_draft.clone();
                    if value != self.config.baudrate {
                        let mut config = self.config.clone();
                        config.baudrate = value;
                        self.tasks.send(Task::Apply(config, true));
                    }
                }
            }
        });
    }

    fn extended_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("扩展发送");
            if icon_button(ui, "upload", "导入", true).clicked() {
                self.tasks.send(Task::Import);
            }
            if icon_button(ui, "download", "导出", true).clicked() {
                self.tasks.send(Task::Export(self.extended.clone()));
            }
            if icon_button(ui, "add", "添加条目", true).clicked() {
                self.extended.items.push(ExtendedSendItem {
                    id: chrono::Local::now().timestamp_micros() as u64,
                    delay: self.extended.settings.default_delay,
                    ..Default::default()
                });
            }
        });
        ui.separator();
        let mut send = None;
        ui.horizontal(|ui| {
            ui.label("HEX");
            ui.label("数据内容 / 注释");
            if ui.available_width() > 380.0 {
                ui.label("序号       延时");
            }
        });
        let mut remove = None;
        let mut move_item = None;
        let height = (ui.available_height() - 38.0).max(30.0);
        egui::ScrollArea::vertical()
            .max_height(height)
            .show(ui, |ui| {
                ui.set_min_height(height);
                let width = ui.available_width();
                let data_width = (width - if width > 510.0 { 250.0 } else { 160.0 }).max(60.0);
                for (index, item) in self.extended.items.iter_mut().enumerate() {
                    ui.push_id(item.id, |ui| {
                        if width < 160.0 {
                            if ui
                                .button(if item.comment.is_empty() {
                                    "发送"
                                } else {
                                    &item.comment
                                })
                                .clicked()
                            {
                                send = Some(item.clone());
                            }
                        } else {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut item.is_hex, "");
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.add(
                                        egui::TextEdit::singleline(&mut item.data)
                                            .desired_width(data_width)
                                            .hint_text("数据内容"),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut item.comment)
                                            .desired_width(data_width)
                                            .hint_text("注释"),
                                    );
                                });
                                if width > 510.0 {
                                    ui.add(
                                        egui::DragValue::new(&mut item.sort_order).range(0..=9999),
                                    );
                                    ui.add(egui::DragValue::new(&mut item.delay).range(0..=600000));
                                }
                                ui.scope(|ui| {
                                    ui.spacing_mut().item_spacing.x = 1.0;
                                    if icon_button(ui, "send", "发送此条", true).clicked() {
                                        send = Some(item.clone());
                                    }
                                    if ui
                                        .add_sized(
                                            [20.0, 24.0],
                                            egui::Button::new("↑").frame(false),
                                        )
                                        .clicked()
                                        && index > 0
                                    {
                                        move_item = Some((index, index - 1));
                                    }
                                    if ui
                                        .add_sized(
                                            [20.0, 24.0],
                                            egui::Button::new("↓").frame(false),
                                        )
                                        .clicked()
                                    {
                                        move_item = Some((index, index + 1));
                                    }
                                    if ui
                                        .add_sized(
                                            [20.0, 24.0],
                                            egui::Button::new("×").frame(false),
                                        )
                                        .clicked()
                                    {
                                        remove = Some(index);
                                    }
                                });
                            });
                        }
                        ui.separator();
                    });
                }
                if self.extended.items.is_empty() {
                    ui.weak("暂无扩展发送条目");
                }
            });
        if let Some(index) = remove {
            self.extended.items.remove(index);
        }
        if let Some((from, to)) = move_item {
            if to < self.extended.items.len() {
                self.extended.items.swap(from, to);
            }
        }
        if let Some(item) = send {
            self.send(&item.data, item.is_hex, !item.is_hex, false);
        }
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.extended.settings.multi_send, "多条发送");
            ui.checkbox(&mut self.extended.settings.loop_send, "循环发送");
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(if self.batch.is_empty() {
                            "启动发送"
                        } else {
                            "停止"
                        })
                        .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(76, 175, 80)),
                )
                .clicked()
            {
                self.start_batch();
            }
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.baud_open {
            let response = egui::Modal::new(egui::Id::new("baud-modal"))
                .frame(dialog_frame(false))
                .show(ctx, |ui| {
                    ui.set_width(380.0_f32.min(ctx.screen_rect().width() - 48.0));
                    dialog_header(ui, "自定义波特率", &mut self.baud_open);
                    ui.label("请输入波特率");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.live_baud_draft)
                            .desired_width(f32::INFINITY)
                            .hint_text("1 - 10000000"),
                    );
                    ui.weak("自定义值会保存到配置文件，并显示在后续的波特率下拉列表中。");
                    if !self.baud_error.is_empty() {
                        ui.colored_label(Color32::DARK_RED, &self.baud_error);
                    }
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("确定").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(22, 134, 90))
                                .min_size(egui::vec2(72.0, 28.0)),
                            )
                            .clicked()
                        {
                            if self
                                .live_baud_draft
                                .parse::<u32>()
                                .is_ok_and(|value| (1..=10_000_000).contains(&value))
                            {
                                if self.connected {
                                    let mut config = self.config.clone();
                                    config.baudrate = self.live_baud_draft.clone();
                                    self.tasks.send(Task::Apply(config, true));
                                } else {
                                    self.config.baudrate = self.live_baud_draft.clone();
                                    self.config
                                        .baudrate_history
                                        .retain(|value| value != &self.config.baudrate);
                                    self.config
                                        .baudrate_history
                                        .insert(0, self.config.baudrate.clone());
                                    self.config.baudrate_history.truncate(12);
                                    self.persist();
                                }
                                self.baud_open = false;
                            } else {
                                self.baud_error = "波特率必须在 1 到 10000000 之间".into();
                            }
                        }
                        if ui
                            .add_sized([72.0, 28.0], egui::Button::new("取消"))
                            .clicked()
                        {
                            self.baud_open = false;
                        }
                    });
                });
            if response.should_close() {
                self.baud_open = false;
            }
        }
        if self.settings_open {
            let response = egui::Modal::new(egui::Id::new("settings-modal"))
                .frame(dialog_frame(false))
                .show(ctx, |ui| {
                    ui.set_width(650.0_f32.min(ctx.screen_rect().width() - 48.0));
                    dialog_header(ui, "更多设置", &mut self.settings_open);
                    egui::ScrollArea::vertical().max_height((ctx.screen_rect().height() - 145.0).max(220.0)).show(ui, |ui| {
                        ui.columns(2, |columns| {
                            section_card(&mut columns[0], "串口参数", Color32::from_gray(245), |ui| {
                                ui.columns(2, |columns| {
                                    columns[0].label("数据位");
                                    egui::ComboBox::from_id_salt("databits").width(columns[0].available_width()).selected_text(self.draft.databits.to_string()).show_ui(&mut columns[0], |ui| {
                                        for value in 5..=8 { ui.selectable_value(&mut self.draft.databits,value,value.to_string()); }
                                    });
                                    columns[1].label("停止位");
                                    egui::ComboBox::from_id_salt("stopbits").width(columns[1].available_width()).selected_text(self.draft.stopbits.to_string()).show_ui(&mut columns[1], |ui| {
                                        ui.selectable_value(&mut self.draft.stopbits,1.0,"1");
                                        ui.selectable_value(&mut self.draft.stopbits,2.0,"2");
                                    });
                                });
                                ui.columns(2, |columns| {
                                    columns[0].label("校验");
                                    egui::ComboBox::from_id_salt("settings-parity").width(columns[0].available_width()).selected_text(option_label(&self.draft.parity)).show_ui(&mut columns[0], |ui| {
                                        for value in ["None","Odd","Even"] { ui.selectable_value(&mut self.draft.parity,value.into(),option_label(value)); }
                                    });
                                    columns[1].label("流控");
                                    egui::ComboBox::from_id_salt("settings-flow").width(columns[1].available_width()).selected_text(option_label(&self.draft.flowcontrol)).show_ui(&mut columns[1], |ui| {
                                        for value in ["None","Software","Hardware"] { ui.selectable_value(&mut self.draft.flowcontrol,value.into(),option_label(value)); }
                                    });
                                });
                                ui.add_space(6.0);
                                ui.weak("已列出跨平台串口后端的全部通用选项。具体组合以当前设备驱动支持情况为准。");
                            });
                            section_card(&mut columns[1], "接收与显示", Color32::from_gray(245), |ui| {
                                ui.label("帧超时 (ms)");
                                ui.add(egui::DragValue::new(&mut self.draft.frame_timeout).range(1..=5000));
                                ui.add_space(12.0);
                                ui.checkbox(&mut self.draft.display_ansi, "ANSI 颜色显示");
                                ui.add_space(12.0);
                                ui.checkbox(&mut self.draft.auto_scroll, "自动滚动");
                            });
                        });
                        ui.add_space(10.0);
                        section_card(ui, "调试探针 / RTT", Color32::from_gray(245), |ui| {
                            ui.checkbox(&mut self.draft.support_probes, "扫描调试探针");
                            ui.add_enabled(self.draft.support_probes, egui::Checkbox::new(&mut self.draft.show_generic_jtag_adapters, "显示通用 FTDI/JTAG 适配器"));
                            ui.label("J-Link SDK 路径");
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.draft.jlink_sdk_path).desired_width((ui.available_width() - 250.0).max(100.0)).hint_text("留空自动查找"));
                                if ui.button("选择目录").clicked() { self.tasks.send(Task::PickSdk(true)); }
                                if ui.button("选择动态库").clicked() { self.tasks.send(Task::PickSdk(false)); }
                                if ui.button("自动查找").clicked() {
                                    self.draft.jlink_sdk_path.clear();
                                    self.tasks.send(Task::CheckSdk(String::new()));
                                }
                            });
                            ui.weak(&self.sdk_status);
                            ui.horizontal(|ui| {
                                ui.label("目标芯片");
                                ui.add(egui::TextEdit::singleline(&mut self.draft.probe_chip).desired_width(210.0));
                                ui.label("SWD 速度 (kHz)");
                                ui.add(egui::DragValue::new(&mut self.draft.probe_speed).range(10..=50000));
                            });
                            egui::Frame::new().fill(Color32::from_rgb(245,239,220)).inner_margin(8.0).show(ui, |ui| {
                                ui.label("J-Link 使用已安装的 SEGGER 官方驱动；其他探针使用 probe-rs。将 Target YAML 放入自定义 MCU 目录即可扩展目标。");
                                ui.horizontal(|ui| {
                                    ui.label(format!("已加载 {} 个自定义目标", self.targets.len()));
                                    if ui.button("打开目录").clicked() {
                                        let path = self.directory.join("probe_rs_targets");
                                        let _ = std::fs::create_dir_all(&path);
                                        let _ = open_directory(&path);
                                    }
                                    if ui.button("重新加载").clicked() { self.refresh_devices(); }
                                });
                            });
                        });
                        ui.add_space(10.0);
                        section_card(ui, "数据存储", Color32::from_gray(245), |ui| {
                            ui.label("设置自动保存到 settings.json，扩展发送保存到 extended_send.json；通信日志按日期自动写入 logs 文件夹。");
                            ui.add(egui::Label::new(self.directory.display().to_string()).wrap());
                            if ui.button("打开配置目录").clicked() { let _ = open_directory(&self.directory); }
                        });
                    });
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("应用").color(Color32::WHITE)).fill(Color32::from_rgb(76,175,80)).min_size(egui::vec2(72.0,28.0))).clicked() {
                            self.tasks.send(Task::Apply(self.draft.clone(), self.connected && self.config.transport_mode == "serial"));
                        }
                        if ui.add_sized([72.0,28.0], egui::Button::new("取消")).clicked() { self.settings_open = false; }
                    });
                });
            if response.should_close() {
                self.settings_open = false;
            }
        }
        let mut about_open = self.about_open;
        if about_open {
            let response = egui::Modal::new(egui::Id::new("about-modal"))
                .frame(dialog_frame(true))
                .show(ctx, |ui| {
                    ui.set_width(584.0_f32.min(ctx.screen_rect().width() - 48.0));
                    ui.spacing_mut().interact_size.y = 22.0;
                    ui.spacing_mut().button_padding = egui::vec2(7.0, 3.0);
                    dialog_header(ui, "关于 Z_COM", &mut about_open);
                    egui::ScrollArea::vertical()
                        .max_height((ctx.screen_rect().height() - 180.0).max(200.0))
                        .show(ui, |ui| {
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Image::new(&self.logo)
                                        .fit_to_exact_size(egui::vec2(72.0, 72.0))
                                        .corner_radius(16.0),
                                );
                                ui.add_space(10.0);
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("Z_COM")
                                                .size(25.0)
                                                .strong()
                                                .color(Color32::from_rgb(21, 60, 44)),
                                        );
                                        badge(ui, &format!("v{}", env!("CARGO_PKG_VERSION")), true);
                                    });
                                    ui.label("便携式跨平台通信与 RTT 调试终端");
                                    ui.horizontal(|ui| {
                                        badge(ui, "绿色免安装", false);
                                        badge(ui, "Windows / Linux", false);
                                        badge(
                                            ui,
                                            &format!(
                                                "实例 {}",
                                                self.instance
                                                    .as_ref()
                                                    .map(|instance| instance.id)
                                                    .unwrap_or(1)
                                            ),
                                            false,
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.weak("代码仓库");
                                        ui.hyperlink_to(
                                            "GitHub",
                                            "https://github.com/ZIKEER/Z_COM",
                                        );
                                        ui.hyperlink_to(
                                            "Gitee",
                                            "https://gitee.com/zzk11111111/Z_COM",
                                        );
                                        if ui.button("开源许可").clicked() {
                                            self.legal_open = true;
                                        }
                                    });
                                });
                            });
                            ui.add_space(14.0);
                            ui.columns(2, |columns| {
                                section_card(
                                    &mut columns[0],
                                    "串口通信",
                                    Color32::WHITE,
                                    |ui| {
                                        ui.weak("运行时参数调整");
                                    },
                                );
                                section_card(
                                    &mut columns[1],
                                    "网络通信",
                                    Color32::WHITE,
                                    |ui| {
                                        ui.weak("TCP / UDP 双角色");
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            ui.columns(2, |columns| {
                                section_card(&mut columns[0], "J-Link RTT", Color32::WHITE, |ui| {
                                    ui.weak("SEGGER 官方后端");
                                });
                                section_card(
                                    &mut columns[1],
                                    "通用探针 RTT",
                                    Color32::WHITE,
                                    |ui| {
                                        ui.weak("probe-rs + 自定义目标");
                                    },
                                );
                            });
                            ui.add_space(14.0);
                            section_card(
                                ui,
                                "运行信息",
                                Color32::from_rgb(238, 245, 241),
                                |ui| {
                                    egui::Grid::new("runtime-info").spacing([12.0, 7.0]).show(
                                        ui,
                                        |ui| {
                                            ui.weak("构建时间");
                                            let timestamp = env!("Z_COM_BUILD_TIMESTAMP")
                                                .parse::<i64>()
                                                .ok()
                                                .and_then(|value| {
                                                    chrono::DateTime::from_timestamp(value, 0)
                                                });
                                            ui.label(
                                                timestamp
                                                    .map(|value| {
                                                        value
                                                            .with_timezone(&chrono::Local)
                                                            .format("%Y/%m/%d %H:%M:%S")
                                                            .to_string()
                                                    })
                                                    .unwrap_or_else(|| "未知".into()),
                                            );
                                            ui.end_row();
                                            ui.weak("数据目录");
                                            ui.add(
                                                egui::Label::new(
                                                    self.directory.display().to_string(),
                                                )
                                                .wrap(),
                                            );
                                            ui.end_row();
                                            ui.weak("数据保护");
                                            ui.label("通信报文按日期自动写入日志，无需手动保存");
                                            ui.end_row();
                                        },
                                    );
                                },
                            );
                            ui.add_space(14.0);
                            section_card(
                                ui,
                                "软件更新",
                                Color32::from_rgb(245, 247, 246),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(&self.update_status)
                                            .size(11.0)
                                            .color(Color32::from_rgb(113, 128, 120)),
                                    );
                                    if let Some(info) = &self.update_info {
                                        if info.available {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    egui::RichText::new(if info.title.is_empty() {
                                                        format!("Z_COM v{}", info.version)
                                                    } else {
                                                        info.title.clone()
                                                    })
                                                    .strong()
                                                    .color(Color32::from_rgb(23, 107, 72)),
                                                );
                                                ui.weak(format!(
                                                    "{} · {:.2} MiB{}",
                                                    info.source,
                                                    info.asset_size as f64 / 1048576.0,
                                                    if info.mirror_available {
                                                        " · 备用镜像可用"
                                                    } else {
                                                        ""
                                                    }
                                                ));
                                            });
                                        } else {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "当前版本 v{}，{}",
                                                    info.current_version,
                                                    if info.latest_confirmed {
                                                        "暂时没有可用的新版本。"
                                                    } else {
                                                        "部分更新源未能完成检测。"
                                                    }
                                                ))
                                                .size(12.0)
                                                .color(Color32::from_rgb(104, 119, 111)),
                                            );
                                        }
                                        if info.available && !info.notes.is_empty() {
                                            egui::ScrollArea::vertical()
                                                .id_salt("release_notes")
                                                .max_height(120.0)
                                                .show(ui, |ui| {
                                                    ui.label(&info.notes);
                                                });
                                        }
                                        ui.add_space(6.0);
                                        ui.columns(2, |columns| {
                                            for (index, result) in
                                                info.source_results.iter().enumerate()
                                            {
                                                let failed = result.state == "failed";
                                                egui::Frame::new()
                                                    .fill(if failed {
                                                        Color32::from_rgb(255, 248, 248)
                                                    } else {
                                                        Color32::WHITE
                                                    })
                                                    .stroke(egui::Stroke::new(
                                                        1.0_f32,
                                                        if failed {
                                                            Color32::from_rgb(231, 194, 194)
                                                        } else {
                                                            Color32::from_rgb(215, 223, 218)
                                                        },
                                                    ))
                                                    .corner_radius(6.0)
                                                    .inner_margin(9.0)
                                                    .show(&mut columns[index % 2], |ui| {
                                                        ui.set_width(ui.available_width());
                                                        ui.horizontal(|ui| {
                                                            ui.label(
                                                                egui::RichText::new(&result.source)
                                                                    .size(12.0)
                                                                    .strong()
                                                                    .color(Color32::from_rgb(
                                                                        41, 75, 60,
                                                                    )),
                                                            );
                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            if failed {
                                                                                "检测失败".into()
                                                                            } else {
                                                                                format!(
                                                                                    "v{}",
                                                                                    result.version
                                                                                )
                                                                            },
                                                                        )
                                                                        .size(11.0)
                                                                        .color(if failed {
                                                                            Color32::from_rgb(
                                                                                161, 38, 38,
                                                                            )
                                                                        } else {
                                                                            Color32::from_rgb(
                                                                                23, 107, 72,
                                                                            )
                                                                        }),
                                                                    );
                                                                },
                                                            );
                                                        });
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(if failed {
                                                                    result.error.as_str()
                                                                } else {
                                                                    match result.state.as_str() {
                                                                        "newer" => "高于当前版本",
                                                                        "older" => "低于当前版本",
                                                                        _ => "与当前版本相同",
                                                                    }
                                                                })
                                                                .size(11.0)
                                                                .color(if failed {
                                                                    Color32::from_rgb(161, 38, 38)
                                                                } else {
                                                                    Color32::from_rgb(113, 128, 120)
                                                                }),
                                                            )
                                                            .wrap(),
                                                        );
                                                    });
                                            }
                                        });
                                        if !info.source_warning.is_empty() {
                                            ui.add_space(6.0);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&info.source_warning)
                                                        .size(12.0)
                                                        .color(Color32::from_rgb(154, 101, 0)),
                                                )
                                                .wrap(),
                                            );
                                        }
                                    }
                                },
                            );
                        });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Copyright 2026 ZIKEER · Apache-2.0")
                                .size(11.0)
                                .color(Color32::from_rgb(130, 144, 135)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_sized([82.0, 28.0], egui::Button::new("关闭"))
                                .clicked()
                            {
                                about_open = false;
                            }
                            if ui
                                .add_enabled(!self.update_busy, egui::Button::new("检查更新"))
                                .clicked()
                            {
                                self.update_busy = true;
                                let sender = self.update_sender.clone();
                                let context = ctx.clone();
                                std::thread::spawn(move || {
                                    let _ = sender.send(UpdateEvent::Checked(
                                        crate::update::check(env!("CARGO_PKG_VERSION")),
                                    ));
                                    context.request_repaint();
                                });
                            }
                            if self.update_candidate.is_some()
                                && self.update_path.is_none()
                                && !self.update_busy
                                && ui.button("下载更新").clicked()
                            {
                                self.update_busy = true;
                                self.update_cancel
                                    .store(false, std::sync::atomic::Ordering::Relaxed);
                                let candidate = self.update_candidate.clone().unwrap();
                                let sender = self.update_sender.clone();
                                let cancel = self.update_cancel.clone();
                                let context = ctx.clone();
                                std::thread::spawn(move || {
                                    let progress = |percent, source: &str| {
                                        let _ = sender
                                            .send(UpdateEvent::Progress(percent, source.into()));
                                        context.request_repaint();
                                    };
                                    let result = crate::update_download::download(
                                        &progress, &candidate, &cancel,
                                    )
                                    .map(|(_, path)| path);
                                    let _ = sender.send(UpdateEvent::Downloaded(result));
                                    context.request_repaint();
                                });
                            }
                            if self.update_busy && ui.button("取消下载").clicked() {
                                self.update_cancel
                                    .store(true, std::sync::atomic::Ordering::Relaxed);
                            }
                            if let Some(path) = &self.update_path {
                                if ui.button("安装并重启").clicked() {
                                    let confirmed = rfd::MessageDialog::new()
                                        .set_title("安装更新")
                                        .set_description("安装已校验的更新并重启 Z_COM？")
                                        .set_buttons(rfd::MessageButtons::YesNo)
                                        .show()
                                        == rfd::MessageDialogResult::Yes;
                                    if confirmed {
                                        let result = if cfg!(debug_assertions) {
                                            Err("开发模式不允许替换程序".into())
                                        } else {
                                            self.instance
                                                .as_ref()
                                                .ok_or_else(|| "实例状态不可用".into())
                                                .and_then(|instance| {
                                                    crate::ensure_instance_lock_is_exclusive(
                                                        &instance.lock_path,
                                                    )
                                                })
                                                .and_then(|_| {
                                                    crate::update_apply::launch(
                                                        self.update_candidate.as_ref().unwrap(),
                                                        path,
                                                    )
                                                })
                                        };
                                        match result {
                                            Ok(()) => {
                                                self.transport
                                                    .lock()
                                                    .unwrap_or_else(|error| error.into_inner())
                                                    .disconnect();
                                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                            }
                                            Err(error) => self.error = error,
                                        }
                                    }
                                }
                            }
                        });
                    });
                });
            if response.should_close() {
                about_open = false;
            }
        }
        self.about_open = about_open;
        if self.legal_open {
            let response = egui::Modal::new(egui::Id::new("legal-modal"))
                .frame(dialog_frame(true))
                .show(ctx, |ui| {
                    ui.set_width(784.0_f32.min(ctx.screen_rect().width() - 48.0));
                    dialog_header(
                        ui,
                        match self.legal_tab {
                            0 => "Apache-2.0 许可证",
                            1 => "NOTICE",
                            _ => "第三方声明",
                        },
                        &mut self.legal_open,
                    );
                    ui.horizontal(|ui| {
                        for (index, title) in ["LICENSE", "NOTICE", "第三方声明"].iter().enumerate()
                        {
                            let selected = self.legal_tab == index;
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new(*title).color(
                                        if selected {
                                            Color32::WHITE
                                        } else {
                                            Color32::from_rgb(82, 99, 91)
                                        },
                                    ))
                                    .fill(if selected {
                                        Color32::from_rgb(23, 107, 72)
                                    } else {
                                        Color32::WHITE
                                    })
                                    .corner_radius(4.0),
                                )
                                .clicked()
                            {
                                self.legal_tab = index;
                            }
                        }
                    });
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .min_scrolled_height((ctx.screen_rect().height() - 200.0).max(180.0))
                        .max_height((ctx.screen_rect().height() - 200.0).max(180.0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(match self.legal_tab {
                                        0 => include_str!("../../LICENSE"),
                                        1 => include_str!("../../NOTICE"),
                                        _ => include_str!("../../THIRD_PARTY_NOTICES.md"),
                                    })
                                    .monospace(),
                                )
                                .wrap(),
                            );
                        });
                });
            if response.should_close() {
                self.legal_open = false;
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.style_mut(|style| {
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
            style
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
            style
                .text_styles
                .insert(egui::TextStyle::Monospace, egui::FontId::monospace(12.0));
            style.spacing.interact_size.y = 29.0;
            style.spacing.button_padding = egui::vec2(9.0, 5.0);
            style.spacing.icon_width = 17.0;
            style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(245, 245, 245);
            style.visuals.widgets.inactive.bg_fill = Color32::WHITE;
            style.visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(1.0_f32, Color32::from_gray(173));
            style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
            style.visuals.selection.bg_fill = Color32::from_rgb(225, 225, 225);
            style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(119));
        });
        self.handle_events();
        if let Some(size) = ctx.input(|input| input.viewport().inner_rect.map(|rect| rect.size())) {
            if size.x >= 760.0 && size.y >= 560.0 {
                let width = size.x.round() as u32;
                let height = size.y.round() as u32;
                if self.config.window_width != width || self.config.window_height != height {
                    self.config.window_width = width;
                    self.config.window_height = height;
                    self.dirty = Some(Instant::now());
                }
            }
        }
        while let Ok(result) = self.tasks.results.try_recv() {
            match result {
                TaskResult::Scanned(devices, warnings, targets, addresses) => {
                    self.scanning = false;
                    self.devices = devices;
                    self.targets = targets;
                    self.addresses = addresses;
                    if let Some(warning) = warnings.first() {
                        self.error = warning.clone();
                    }
                    if self.config.port.is_empty() {
                        self.config.port = self
                            .devices
                            .iter()
                            .find(|device| device.transport == "serial")
                            .map(|device| device.id.clone())
                            .unwrap_or_default();
                        self.dirty = Some(Instant::now());
                    }
                }
                TaskResult::Saved(result) | TaskResult::Exported(result) => {
                    if let Err(error) = result {
                        self.error = error;
                    }
                }
                TaskResult::Applied(result) => match result {
                    Ok(config) => {
                        self.live_baud_draft = config.baudrate.clone();
                        self.config = config;
                        self.settings_open = false;
                        self.persist();
                    }
                    Err(error) => {
                        self.live_baud_draft = self.config.baudrate.clone();
                        self.error = error;
                    }
                },
                TaskResult::Sdk(result) => self.sdk_status = result.unwrap_or_else(|error| error),
                TaskResult::Imported(result) => match result {
                    Ok(Some(extended)) => {
                        self.extended = extended;
                        self.persist();
                    }
                    Ok(None) => {}
                    Err(error) => self.error = error,
                },
                TaskResult::SdkPicked(path) => {
                    if let Some(path) = path {
                        self.draft.jlink_sdk_path = path;
                        self.tasks
                            .send(Task::CheckSdk(self.draft.jlink_sdk_path.clone()));
                    }
                }
            }
        }
        while let Ok(event) = self.update_events.try_recv() {
            match event {
                UpdateEvent::Checked(result) => {
                    self.update_busy = false;
                    match result {
                        Ok((info, candidate)) => {
                            self.update_status = if info.available {
                                "发现新版本".into()
                            } else if info.latest_confirmed {
                                "当前已是最新版本".into()
                            } else {
                                "部分更新源检查失败".into()
                            };
                            self.update_info = Some(info);
                            self.update_candidate = candidate;
                            self.update_path = None;
                        }
                        Err(error) => self.update_status = error,
                    }
                }
                UpdateEvent::Progress(percent, source) => {
                    self.update_status = format!("{source} 下载 {percent}%")
                }
                UpdateEvent::Downloaded(result) => {
                    self.update_busy = false;
                    match result {
                        Ok(path) => {
                            self.update_path = Some(path);
                            self.update_status = "下载完成，SHA-256 校验通过".into();
                        }
                        Err(error) => self.update_status = error,
                    }
                }
            }
        }
        if self
            .connecting
            .is_some_and(|start| start.elapsed() > Duration::from_secs(12))
        {
            self.transport
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .disconnect();
            self.connecting = None;
            self.error = "连接超时".into();
        }
        let before = (self.config.clone(), self.extended.clone());
        if self.auto_send && self.connected && Instant::now() >= self.next_auto {
            self.send(
                &self.send_text.clone(),
                self.config.send_mode == "HEX",
                false,
                self.crlf,
            );
            self.next_auto =
                Instant::now() + Duration::from_millis(self.config.auto_send_interval.max(100));
        }
        if !self.batch.is_empty() && Instant::now() >= self.batch_due {
            let item = self.batch[self.batch_index].clone();
            self.send(&item.data, item.is_hex, !item.is_hex, false);
            if !self.batch.is_empty() {
                self.batch_index += 1;
                self.batch_due = Instant::now() + Duration::from_millis(item.delay);
                if self.batch_index >= self.batch.len() {
                    if self.extended.settings.loop_send {
                        self.batch_index = 0;
                    } else {
                        self.batch.clear();
                    }
                }
            }
        }
        egui::TopBottomPanel::top("connection").show(ctx, |ui| self.connection_ui(ui));
        if !self.error.is_empty() {
            egui::TopBottomPanel::top("error").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::DARK_RED, &self.error);
                    if ui.button("×").clicked() {
                        self.error.clear();
                    }
                });
            });
        }
        egui::TopBottomPanel::bottom("status")
            .exact_height(25.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        if self.connected {
                            Color32::DARK_GREEN
                        } else {
                            Color32::GRAY
                        },
                        "●",
                    );
                    ui.label(&self.status);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                        ui.label(format!("接收 {} B", self.received));
                        ui.label(format!("发送 {} B", self.sent));
                        ui.label(format!(
                            "实例 {}",
                            self.instance
                                .as_ref()
                                .map(|instance| instance.id)
                                .unwrap_or(1)
                        ));
                    });
                });
            });
        let send_panel = egui::TopBottomPanel::bottom("send")
            .resizable(true)
            .default_height(
                (self.config.window_height as f32 * self.config.send_panel_ratio)
                    .clamp(85.0, 350.0),
            )
            .height_range(85.0..=350.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    control_group(ui, |ui| {
                        segment(ui, &mut self.config.send_mode, "ASCII".into(), "ASCII");
                        segment(ui, &mut self.config.send_mode, "HEX".into(), "HEX");
                    });
                    ui.checkbox(&mut self.crlf, "添加 CRLF");
                    ui.add_enabled(
                        self.connected,
                        egui::Checkbox::new(&mut self.auto_send, "自动发送"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.config.auto_send_interval)
                            .range(100..=60000),
                    );
                    ui.label("ms");
                });
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [
                            ui.available_width() - 80.0,
                            (ui.available_height() - 4.0).max(36.0),
                        ],
                        egui::TextEdit::multiline(&mut self.send_text)
                            .hint_text("输入要发送的数据")
                            .font(egui::TextStyle::Monospace),
                    );
                    if ui
                        .add_enabled(
                            self.connected,
                            egui::Button::new("发送").min_size(egui::vec2(65.0, 55.0)),
                        )
                        .clicked()
                    {
                        self.send(
                            &self.send_text.clone(),
                            self.config.send_mode == "HEX",
                            false,
                            self.crlf,
                        );
                    }
                });
            });
        let send_ratio = (send_panel.response.rect.height()
            / self.config.window_height.max(1) as f32)
            .clamp(0.12, 0.5);
        if (self.config.send_panel_ratio - send_ratio).abs() > 0.002 {
            self.config.send_panel_ratio = send_ratio;
        }
        if self.config.preset_panel_visible {
            let panel = egui::SidePanel::right("extended")
                .resizable(true)
                .default_width(self.config.window_width as f32 * self.config.extended_panel_ratio)
                .width_range(65.0..=650.0)
                .show(ctx, |ui| self.extended_ui(ui));
            self.config.extended_panel_ratio = (panel.response.rect.width()
                / self.config.window_width.max(1) as f32)
                .clamp(0.07, 0.7);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(6.0))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("接收区域");
                    control_group(ui, |ui| {
                        for (mode, label) in
                            [("HEX", "HEX"), ("ASCII", "ASCII"), ("MIXED", "HEX+ASCII")]
                        {
                            segment(ui, &mut self.config.display_mode, mode.into(), label);
                        }
                    });
                    ui.checkbox(&mut self.config.auto_scroll, "自动滚动");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, "clear", "清空报文并重置收发计数", true).clicked()
                        {
                            self.receive.clear();
                            self.receive_view.clear();
                            self.counter_baseline = self.events.counters();
                            self.sent = 0;
                            self.received = 0;
                        }
                        if ui.button("曲线").clicked() {
                            self.plot.open = true;
                        }
                    });
                });
                ui.separator();
                if self.receive_view.ui(
                    ui,
                    &self.receive,
                    &mut self.config.display_ansi,
                    self.config.auto_scroll,
                ) {
                    self.receive.clear();
                    self.receive_view.clear();
                    self.counter_baseline = self.events.counters();
                    self.sent = 0;
                    self.received = 0;
                }
            });
        self.dialogs(ctx);
        if self.plot.open {
            let mut plot_open = true;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("plot"),
                egui::ViewportBuilder::default()
                    .with_title("数据曲线")
                    .with_inner_size([900.0, 560.0]),
                |context, _| {
                    if context.input(|input| input.viewport().close_requested()) {
                        plot_open = false;
                    }
                    egui::CentralPanel::default().show(context, |ui| self.plot.ui(ui));
                },
            );
            self.plot.open = plot_open;
        }
        if before.0 != self.config || before.1 != self.extended {
            self.dirty = Some(Instant::now());
        }
        if self
            .dirty
            .is_some_and(|dirty| dirty.elapsed() >= Duration::from_millis(400))
        {
            self.persist();
        }
        if !self.events.is_empty()
            || self.auto_send
            || !self.batch.is_empty()
            || self.connecting.is_some()
            || self.dirty.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.transport
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .disconnect();
        if self.preview_mode {
            return;
        }
        let _ = storage::save_config(&self.directory, &self.config);
        let _ = storage::save_extended(&self.directory, &self.extended);
        let _ = std::fs::write(
            self.directory.join("plot-template.txt"),
            &self.plot.template,
        );
    }
}

fn control_group(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(180, 185, 192)))
        .corner_radius(0.0)
        .inner_margin(0.0)
        .fill(Color32::from_rgb(247, 248, 250))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                contents(ui);
            });
        });
}

fn segment<Value: PartialEq>(ui: &mut egui::Ui, value: &mut Value, option: Value, label: &str) {
    let selected = *value == option;
    let button = egui::Button::new(label)
        .fill(if selected {
            Color32::from_gray(225)
        } else {
            Color32::from_gray(245)
        })
        .stroke(egui::Stroke::new(
            1.0_f32,
            Color32::from_gray(if selected { 119 } else { 200 }),
        ))
        .min_size(egui::vec2(0.0, 28.0));
    if ui.add(button).clicked() {
        *value = option;
    }
}

fn icon_button(ui: &mut egui::Ui, icon: &str, hint: &str, subtle: bool) -> egui::Response {
    let response = ui.add_sized([29.0, 29.0], egui::Button::new("").frame(!subtle));
    let center = response.rect.center();
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.3_f32, Color32::from_gray(60));
    let point = |horizontal: f32, vertical: f32| center + egui::vec2(horizontal, vertical);
    let line = |coordinates: &[(f32, f32)]| {
        painter.add(egui::Shape::line(
            coordinates
                .iter()
                .map(|&(horizontal, vertical)| point(horizontal, vertical))
                .collect(),
            stroke,
        ));
    };
    match icon {
        "refresh" => {
            line(&[
                (-6.0, -1.0),
                (-4.0, -5.0),
                (0.0, -7.0),
                (5.0, -5.0),
                (6.0, -2.0),
            ]);
            line(&[(2.0, -2.0), (6.0, -2.0), (6.0, -6.0)]);
            line(&[(6.0, 1.0), (4.0, 5.0), (0.0, 7.0), (-5.0, 5.0), (-6.0, 2.0)]);
            line(&[(-2.0, 2.0), (-6.0, 2.0), (-6.0, 6.0)]);
        }
        "clear" => {
            line(&[
                (-6.0, 1.0),
                (0.0, -6.0),
                (7.0, 1.0),
                (2.0, 6.0),
                (-1.0, 6.0),
                (-6.0, 1.0),
            ]);
            line(&[(-2.0, -3.0), (5.0, 4.0)]);
            line(&[(0.0, 7.0), (8.0, 7.0)]);
        }
        "settings" => {
            painter.circle_stroke(center, 5.0, stroke);
            painter.circle_stroke(center, 2.0, stroke);
            for index in 0..8 {
                let angle = index as f32 * std::f32::consts::TAU / 8.0;
                let direction = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment([center + direction * 5.0, center + direction * 8.0], stroke);
            }
        }
        "help" => {
            painter.circle_stroke(center, 7.0, stroke);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(13.0),
                Color32::from_gray(60),
            );
        }
        "expand" => line(&[(-2.0, -5.0), (3.0, 0.0), (-2.0, 5.0)]),
        "add" => {
            line(&[(-6.0, 0.0), (6.0, 0.0)]);
            line(&[(0.0, -6.0), (0.0, 6.0)]);
        }
        "upload" | "download" => {
            let direction = if icon == "upload" { -1.0 } else { 1.0 };
            line(&[(0.0, -6.0), (0.0, 4.0)]);
            line(&[
                (-4.0, direction * 2.0),
                (0.0, direction * 6.0),
                (4.0, direction * 2.0),
            ]);
            line(&[(-6.0, 4.0), (-6.0, 7.0), (6.0, 7.0), (6.0, 4.0)]);
        }
        _ => line(&[
            (-6.0, -5.0),
            (7.0, 0.0),
            (-6.0, 6.0),
            (-2.0, 0.0),
            (-6.0, -5.0),
        ]),
    }
    response.on_hover_text(hint)
}

pub(crate) fn dialog_frame(rounded: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(248, 250, 249))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(143, 154, 148)))
        .corner_radius(if rounded { 10.0 } else { 0.0 })
        .inner_margin(16.0)
        .shadow(egui::epaint::Shadow {
            offset: [0, 20],
            blur: 55,
            spread: 0,
            color: Color32::from_black_alpha(65),
        })
}

pub(crate) fn dialog_header(ui: &mut egui::Ui, title: &str, open: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).size(15.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_sized([28.0, 28.0], egui::Button::new("×").frame(false))
                .clicked()
            {
                *open = false;
            }
        });
    });
    ui.separator();
}

fn section_card(
    ui: &mut egui::Ui,
    title: &str,
    fill: Color32,
    contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(210, 226, 217)))
        .corner_radius(6.0)
        .inner_margin(11.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(Color32::from_rgb(41, 75, 60)),
            );
            contents(ui);
        });
}

fn badge(ui: &mut egui::Ui, label: &str, green: bool) {
    egui::Frame::new()
        .fill(if green {
            Color32::from_rgb(229, 244, 236)
        } else {
            Color32::WHITE
        })
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(214, 223, 218)))
        .corner_radius(if green { 12.0 } else { 4.0 })
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.add(egui::Label::new(
                egui::RichText::new(label)
                    .size(11.0)
                    .color(Color32::from_rgb(82, 99, 91)),
            ));
        });
}

fn choices(ui: &mut egui::Ui, name: &str, value: &mut String, values: &[&str]) {
    ui.label(name);
    egui::ComboBox::from_id_salt(name)
        .width(70.0)
        .selected_text(option_label(value))
        .show_ui(ui, |ui| {
            for option in values {
                ui.selectable_value(value, (*option).into(), option_label(option));
            }
        });
}

fn option_label(value: &str) -> &str {
    match value {
        "None" => "无",
        "Odd" => "奇校验",
        "Even" => "偶校验",
        "Software" => "XON/XOFF",
        "Hardware" => "RTS/CTS",
        "Client" => "客户端",
        "Server" => "服务端",
        _ => value,
    }
}

fn open_directory(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

fn display_ascii(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        match *byte {
            27 => output.push('\u{1b}'),
            0..=31 => {
                output.push(char::from_u32(0x2400 + u32::from(*byte)).unwrap());
                if *byte == 10 {
                    output.push('\n');
                }
            }
            32..=126 => output.push(char::from(*byte)),
            127 => output.push('␡'),
            _ => output.push_str(&format!("\\x{byte:02x}")),
        }
    }
    output
}

fn encode(value: &str, hex: bool, escapes: bool, crlf: bool) -> Result<Vec<u8>, String> {
    let mut bytes = if hex {
        let normalized = value.trim();
        if !regex::Regex::new(r"(?i)^(?:0x)?[0-9a-f]+(?:[\s,]+(?:0x)?[0-9a-f]+)*$")
            .unwrap()
            .is_match(normalized)
        {
            return Err("HEX 数据格式无效".into());
        }
        let compact = normalized
            .replace("0x", "")
            .replace("0X", "")
            .chars()
            .filter(|value| !value.is_whitespace() && *value != ',')
            .collect::<String>();
        if compact.len() % 2 != 0 {
            return Err("HEX 数据必须由完整字节组成".into());
        }
        compact
            .as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    } else if escapes {
        let mut output = String::new();
        let mut chars = value.chars();
        while let Some(character) = chars.next() {
            if character != '\\' {
                output.push(character);
                continue;
            }
            output.push(match chars.next().ok_or("转义字符不完整")? {
                'r' => '\r',
                'n' => '\n',
                't' => '\t',
                '0' => '\0',
                '\\' => '\\',
                'x' => {
                    let pair: String = chars.by_ref().take(2).collect();
                    if pair.len() != 2 {
                        return Err("\\x 转义必须包含两个十六进制字符".into());
                    }
                    char::from(u8::from_str_radix(&pair, 16).map_err(|_| "无效的十六进制转义")?)
                }
                other => return Err(format!("不支持的转义 \\{other}")),
            });
        }
        output.into_bytes()
    } else {
        value.as_bytes().to_vec()
    };
    if bytes.is_empty() {
        return Err("发送数据不能为空".into());
    }
    if crlf {
        bytes.extend_from_slice(b"\r\n");
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::encode;
    #[test]
    fn validates_hex_and_escape_sequences() {
        assert_eq!(encode("0x01, 0203", true, false, false).unwrap(), [1, 2, 3]);
        assert!(encode("01 GG", true, false, false).is_err());
        assert!(encode("123", true, false, false).is_err());
        assert_eq!(encode(r"A\r\n\x41", false, true, false).unwrap(), b"A\r\nA");
        assert!(encode(r"\q", false, true, false).is_err());
        assert_eq!(encode("A", false, false, true).unwrap(), b"A\r\n");
    }
}
