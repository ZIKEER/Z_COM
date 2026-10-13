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
    receive: VecDeque<crate::native_terminal::Line>,
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
        let fonts = font_definitions();
        cc.egui_ctx.set_fonts(fonts);
        apply_style(&cc.egui_ctx);
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

    fn append(&mut self, lines: Vec<crate::native_terminal::Line>) {
        for line in lines {
            self.receive.push_back(line);
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
                self.append(crate::native_terminal::data_lines(
                    &stamp,
                    event.direction == "received",
                    &event.bytes,
                    &self.config.display_mode,
                    self.config.display_ansi,
                ));
            } else if !event.message.is_empty() {
                self.append(vec![crate::native_terminal::event_line(
                    &stamp,
                    &event.kind,
                    &event.message,
                )]);
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
        let idle = !self.connected && self.connecting.is_none();
        let viewport = ui.ctx().screen_rect().width();
        let vw = |percent: f32, min: f32, max: f32| (viewport * percent / 100.0).clamp(min, max);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            ui.add_enabled_ui(idle, |ui| {
                mode_switch(
                    ui,
                    &mut self.config.transport_mode,
                    &[
                        ("serial", "串口"),
                        ("socket", "Socket"),
                        ("probe", "调试探针 / RTT"),
                    ],
                    false,
                );
            });
            ui.add_enabled_ui(idle && !self.scanning, |ui| {
                let hint = if self.scanning {
                    "正在扫描设备"
                } else {
                    "刷新设备"
                };
                if icon_button(ui, "refresh", hint, false).clicked() {
                    self.refresh_devices();
                }
            });
            match self.config.transport_mode.as_str() {
                "socket" => self.socket_fields(ui, idle),
                "probe" => self.probe_fields(ui, idle, vw(27.0, 190.0, 360.0), vw(18.0, 145.0, 220.0)),
                _ => self.serial_fields(ui, idle, vw(23.0, 160.0, 300.0)),
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let open = self.config.preset_panel_visible;
                if icon_button_sized(
                    ui,
                    if open { "chevron-right" } else { "chevron-left" },
                    "扩展发送",
                    false,
                    open,
                    30.0,
                )
                .clicked()
                {
                    self.config.preset_panel_visible = !open;
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
                let (icon, label, fill, border) = if self.connected {
                    ("unlink", "断开", palette::GREEN, palette::GREEN_BORDER)
                } else if self.connecting.is_some() {
                    ("stop", "取消", palette::AMBER, palette::AMBER_BORDER)
                } else {
                    ("link", "连接", palette::RED, palette::RED_BORDER)
                };
                if command_button(ui, icon, label, fill, border, 30.0).clicked() {
                    self.connect();
                }
            });
        });
    }

    fn device_select(&mut self, ui: &mut egui::Ui, label: &str, width: f32) {
        field_label(ui, label);
        let probe = self.config.transport_mode == "probe";
        let target = if probe {
            &mut self.config.selected_probe
        } else {
            &mut self.config.port
        };
        let selected = self
            .devices
            .iter()
            .find(|item| item.id == *target)
            .map(|item| item.label.clone())
            .unwrap_or_else(|| target.clone());
        select_box(ui, "device", width, selected, |ui| {
            for device in self
                .devices
                .iter()
                .filter(|item| item.transport == self.config.transport_mode)
            {
                ui.selectable_value(target, device.id.clone(), &device.label);
            }
        });
    }

    fn serial_fields(&mut self, ui: &mut egui::Ui, idle: bool, width: f32) {
        ui.add_enabled_ui(idle, |ui| self.device_select(ui, "设备", width));
        // 与原版一致：连接后仍可切换波特率，立即下发到串口
        ui.add_enabled_ui(self.connecting.is_none(), |ui| {
            field_label(ui, "波特率");
            let before = self.config.baudrate.clone();
            let mut chosen = before.clone();
            select_box(ui, "baudrate", 128.0, before.clone(), |ui| {
                ui.weak("常用波特率");
                for baud in DEFAULT_BAUDRATES {
                    ui.selectable_value(&mut chosen, (*baud).to_owned(), *baud);
                }
                let custom: Vec<_> = self
                    .config
                    .baudrate_history
                    .iter()
                    .filter(|baud| !DEFAULT_BAUDRATES.contains(&baud.as_str()))
                    .cloned()
                    .collect();
                if !custom.is_empty() {
                    ui.separator();
                    ui.weak("自定义记录");
                    for baud in custom {
                        ui.selectable_value(&mut chosen, baud.clone(), baud);
                    }
                }
                ui.separator();
                if ui.selectable_label(false, "自定义…").clicked() {
                    self.live_baud_draft.clear();
                    self.baud_error.clear();
                    self.baud_open = true;
                }
            });
            if chosen != before {
                if self.connected {
                    let mut config = self.config.clone();
                    config.baudrate = chosen;
                    self.tasks.send(Task::Apply(config, true));
                } else {
                    self.config.baudrate = chosen;
                }
            }
        });
    }

    fn socket_fields(&mut self, ui: &mut egui::Ui, idle: bool) {
        ui.add_enabled_ui(idle, |ui| {
            mode_switch(
                ui,
                &mut self.config.socket_protocol,
                &[("TCP", "TCP"), ("UDP", "UDP")],
                true,
            );
            mode_switch(
                ui,
                &mut self.config.socket_role,
                &[("Client", "客户端"), ("Server", "服务端")],
                true,
            );
            field_label(ui, "地址");
            let server = self.config.socket_role == "Server";
            let response = ui.add_sized(
                [150.0, 30.0],
                egui::TextEdit::singleline(&mut self.config.socket_host)
                    .margin(egui::Margin {
                        left: 7,
                        right: if server { 32 } else { 7 },
                        top: 4,
                        bottom: 4,
                    })
                    .vertical_align(egui::Align::Center),
            );
            if server {
                let popup = ui.make_persistent_id("local-addresses");
                let open = ui.memory(|memory| memory.is_popup_open(popup));
                let toggle = egui::Rect::from_min_max(
                    egui::pos2(response.rect.right() - 29.0, response.rect.top() + 1.0),
                    response.rect.right_bottom() - egui::vec2(1.0, 1.0),
                );
                if ui
                    .put(toggle, |ui: &mut egui::Ui| {
                        icon_button_sized(ui, "chevron-down", "本机地址", true, open, 28.0)
                    })
                    .clicked()
                {
                    ui.memory_mut(|memory| memory.toggle_popup(popup));
                }
                egui::popup_below_widget(
                    ui,
                    popup,
                    &response,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.set_min_width(150.0);
                        if self.addresses.is_empty() {
                            ui.weak("未找到本机 IPv4 地址");
                        }
                        for address in &self.addresses {
                            ui.selectable_value(
                                &mut self.config.socket_host,
                                address.clone(),
                                address,
                            );
                        }
                    },
                );
            }
            field_label(ui, "端口");
            ui.add_sized(
                [82.0, 30.0],
                egui::DragValue::new(&mut self.config.socket_port).range(1..=65535),
            );
        });
    }

    fn probe_fields(&mut self, ui: &mut egui::Ui, idle: bool, width: f32, chip_width: f32) {
        ui.add_enabled_ui(idle, |ui| {
            self.device_select(ui, "调试探针", width);
            field_label(ui, "目标芯片");
            // 可输入的下拉框：输入框右侧附带展开按钮，对应原版 `.chip-combobox`
            let popup = ui.make_persistent_id("probe-chip-options");
            let response = ui.add_sized(
                [chip_width, 30.0],
                egui::TextEdit::singleline(&mut self.config.probe_chip)
                    .hint_text("输入或选择目标芯片")
                    .margin(egui::Margin {
                        left: 7,
                        right: 32,
                        top: 4,
                        bottom: 4,
                    })
                    .vertical_align(egui::Align::Center),
            );
            let open = ui.memory(|memory| memory.is_popup_open(popup));
            let toggle = egui::Rect::from_min_max(
                egui::pos2(response.rect.right() - 29.0, response.rect.top() + 1.0),
                response.rect.right_bottom() - egui::vec2(1.0, 1.0),
            );
            if ui
                .put(toggle, |ui: &mut egui::Ui| {
                    icon_button_sized(ui, "chevron-down", "选择目标芯片", true, open, 28.0)
                })
                .clicked()
            {
                ui.memory_mut(|memory| memory.toggle_popup(popup));
            }
            egui::popup_below_widget(
                ui,
                popup,
                &response,
                egui::PopupCloseBehavior::CloseOnClick,
                |ui| {
                    ui.set_min_width(chip_width);
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        let mut options = self.config.probe_chip_history.clone();
                        for target in &self.targets {
                            if !options.contains(target) {
                                options.push(target.clone());
                            }
                        }
                        if options.is_empty() {
                            ui.weak("暂无记录，请直接输入");
                        }
                        for chip in options {
                            ui.selectable_value(&mut self.config.probe_chip, chip.clone(), chip);
                        }
                    });
                },
            );
            check_styled(
                ui,
                &mut self.config.probe_reset,
                "连接后复位",
                palette::CHECK_GREEN,
                18.0,
            );
        });
    }

    fn status_ui(&mut self, ui: &mut egui::Ui) {
        let small = |text: &str| egui::RichText::new(text).size(12.0).color(palette::LABEL);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(
                dot.center(),
                4.0,
                if self.connected {
                    Color32::from_rgb(0x26, 0x83, 0x4b)
                } else {
                    Color32::from_rgb(0x89, 0x93, 0x9a)
                },
            );
            ui.label(small(&self.status));
            let target = match self.config.transport_mode.as_str() {
                "probe" => "调试探针 RTT".to_owned(),
                "socket" => format!(
                    "{} {}",
                    self.config.socket_protocol,
                    if self.config.socket_role == "Server" {
                        "服务端"
                    } else {
                        "客户端"
                    }
                ),
                _ if self.config.port.is_empty() => "未选择串口".to_owned(),
                _ => self.config.port.clone(),
            };
            ui.label(small(&target))
                .on_hover_text(self.directory.display().to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(small(&format!("v{}", env!("CARGO_PKG_VERSION"))));
                ui.label(small(&format!("接收 {} B", group_digits(self.received))));
                ui.label(small(&format!("发送 {} B", group_digits(self.sent))));
                ui.label(small(&format!(
                    "实例 {}",
                    self.instance
                        .as_ref()
                        .map(|instance| instance.id)
                        .unwrap_or(1)
                )));
            });
        });
    }

    fn send_ui(&mut self, ui: &mut egui::Ui) {
        const BUTTON_WIDTH: f32 = 82.0;
        let area = ui.available_rect_before_wrap();
        let left = egui::Rect::from_min_max(
            area.min,
            egui::pos2(area.right() - BUTTON_WIDTH - 5.0, area.bottom()),
        );
        let button = egui::Rect::from_min_max(egui::pos2(area.right() - BUTTON_WIDTH, area.top()), area.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
            ui.horizontal(|ui| {
                ui.set_height(29.0);
                mode_switch(
                    ui,
                    &mut self.config.send_mode,
                    &[("ASCII", "ASCII"), ("HEX", "HEX")],
                    true,
                );
                check(ui, &mut self.crlf, "添加 CRLF");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("ms");
                    ui.add_sized(
                        [72.0, 25.0],
                        egui::DragValue::new(&mut self.config.auto_send_interval).range(100..=60000),
                    );
                    ui.add_enabled_ui(self.connected, |ui| {
                        check(ui, &mut self.auto_send, "自动发送");
                    });
                });
            });
            ui.add_space(3.0);
            let hint = if self.config.send_mode == "HEX" {
                "输入十六进制字节，例如 01 03 00 00"
            } else {
                "输入要发送的文本"
            };
            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.send_text)
                    .hint_text(hint)
                    .margin(egui::vec2(8.0, 6.0))
                    .font(egui::TextStyle::Monospace),
            );
        });
        let clicked = ui
            .scope_builder(egui::UiBuilder::new().max_rect(button), |ui| {
                ui.add_enabled_ui(self.connected, |ui| {
                    labeled_icon_button(ui, "send", "发送", button.size())
                })
                .inner
            })
            .inner
            .clicked();
        if clicked {
            self.send(
                &self.send_text.clone(),
                self.config.send_mode == "HEX",
                false,
                self.crlf,
            );
        }
        ui.allocate_rect(area, egui::Sense::hover());
    }

    fn receive_ui(&mut self, ui: &mut egui::Ui) {
        let mut clear = false;
        pane_toolbar(ui, |ui| {
            ui.label(bold("接收区域"));
            mode_switch(
                ui,
                &mut self.config.display_mode,
                &[("HEX", "HEX"), ("ASCII", "ASCII"), ("MIXED", "HEX+ASCII")],
                true,
            );
            check(ui, &mut self.config.auto_scroll, "自动滚动");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                clear |= icon_button(ui, "clear", "清空报文并重置收发计数", true).clicked();
                let plot = ui.add(
                    egui::Button::new(
                        egui::RichText::new("曲线")
                            .size(12.0)
                            .color(Color32::from_rgb(0x3f, 0x4b, 0x45)),
                    )
                    .fill(Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(0xb8, 0xc2, 0xbc)))
                    .corner_radius(3.0)
                    .min_size(egui::vec2(0.0, 24.0)),
                );
                if plot.on_hover_text("打开独立数据曲线窗口").clicked() {
                    self.plot.open = true;
                }
            });
        });
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(9, 7))
            .show(ui, |ui| {
                clear |= self.receive_view.ui(
                    ui,
                    &self.receive,
                    &mut self.config.display_ansi,
                    self.config.auto_scroll,
                );
            });
        if clear {
            self.receive.clear();
            self.receive_view.clear();
            self.counter_baseline = self.events.counters();
            self.sent = 0;
            self.received = 0;
        }
    }

    fn extended_ui(&mut self, ui: &mut egui::Ui) {
        const HEX_COLUMN: f32 = 36.0;
        const ORDER_COLUMN: f32 = 48.0;
        const DELAY_COLUMN: f32 = 64.0;
        const ROW_HEIGHT: f32 = 55.0;
        const FOOTER_HEIGHT: f32 = 37.0;
        let width = ui.available_width();
        // 与原版 container query 一致的三档宽度
        let wide = width > 429.0;
        let narrow = width < 280.0;
        let tiny = width < 180.0;
        let actions_column = if narrow { 54.0 } else { 92.0 };
        pane_toolbar(ui, |ui| {
            ui.label(bold("扩展发送"));
            if tiny {
                return;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if icon_button(ui, "add", "添加条目", true).clicked() {
                    self.extended.items.push(ExtendedSendItem {
                        id: chrono::Local::now().timestamp_micros() as u64,
                        delay: self.extended.settings.default_delay,
                        ..Default::default()
                    });
                }
                if icon_button(ui, "download", "导出", true).clicked() {
                    self.tasks.send(Task::Export(self.extended.clone()));
                }
                if icon_button(ui, "upload", "导入", true).clicked() {
                    self.tasks.send(Task::Import);
                }
            });
        });
        let data_column = (width
            - HEX_COLUMN
            - actions_column
            - if wide { ORDER_COLUMN + DELAY_COLUMN } else { 0.0 })
        .max(40.0);
        if !tiny {
            let (header, _) = ui.allocate_exact_size(egui::vec2(width, 27.0), egui::Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(header, 0.0, palette::WINDOW);
            painter.hline(
                header.x_range(),
                header.bottom() - 0.5,
                egui::Stroke::new(1.0_f32, palette::BAR_LINE),
            );
            let mut x = header.left();
            let mut title = |text: &str, column: f32| {
                if !text.is_empty() {
                    painter.text(
                        egui::pos2(x + column / 2.0, header.center().y),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::new(12.0, bold_family()),
                        palette::TEXT,
                    );
                }
                x += column;
            };
            title("HEX", HEX_COLUMN);
            title("数据内容 / 注释", data_column);
            if wide {
                title("序号", ORDER_COLUMN);
                title("延时", DELAY_COLUMN);
            }
        }
        let mut send = None;
        let mut remove = None;
        let mut move_item = None;
        let count = self.extended.items.len();
        let list_height = (ui.available_height() - FOOTER_HEIGHT).max(30.0);
        egui::ScrollArea::vertical()
            .max_height(list_height)
            .min_scrolled_height(list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let width = ui.available_width();
                for (index, item) in self.extended.items.iter_mut().enumerate() {
                    ui.push_id(item.id, |ui| {
                        if tiny {
                            let label = if item.comment.trim().is_empty() {
                                "发送"
                            } else {
                                item.comment.trim()
                            };
                            let (row, _) = ui.allocate_exact_size(egui::vec2(width, 37.0), egui::Sense::hover());
                            let button = egui::Button::new(egui::RichText::new(label))
                                .fill(palette::BAR)
                                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(0xc6, 0xcd, 0xd1)))
                                .truncate();
                            if ui.put(row.shrink(3.0), button).on_hover_text(&item.data).clicked() {
                                send = Some(item.clone());
                            }
                            ui.painter().hline(row.x_range(), row.bottom() - 0.5, egui::Stroke::new(1.0_f32, palette::ROW_LINE));
                            return;
                        }
                        let (row, _) = ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());
                        ui.painter().hline(row.x_range(), row.bottom() - 0.5, egui::Stroke::new(1.0_f32, palette::ROW_LINE));
                        let mut x = row.left();
                        let mut column = |column_width: f32| {
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(x, row.top()),
                                egui::vec2(column_width, ROW_HEIGHT - 1.0),
                            );
                            x += column_width;
                            rect
                        };
                        let hex = column(HEX_COLUMN);
                        ui.put(
                            egui::Rect::from_center_size(hex.center(), egui::vec2(14.0, 20.0)),
                            |ui: &mut egui::Ui| check(ui, &mut item.is_hex, ""),
                        )
                        .on_hover_text("HEX 格式");
                        let data = column(data_column).shrink2(egui::vec2(3.0, 3.0));
                        let (data_hint, data_tip) = if item.is_hex {
                            ("HEX 字节", "输入 HEX 字节，例如 01 03 00 00")
                        } else {
                            (
                                "ASCII，支持 \\r \\n \\t \\0 \\xNN",
                                "支持转义：\\r  \\n  \\t  \\0  \\\\  \\xNN",
                            )
                        };
                        ui.put(
                            egui::Rect::from_min_size(data.min, egui::vec2(data.width(), 24.0)),
                            egui::TextEdit::singleline(&mut item.data)
                                .hint_text(data_hint)
                                .margin(egui::vec2(5.0, 2.0))
                                .vertical_align(egui::Align::Center),
                        )
                        .on_hover_text(data_tip);
                        ui.put(
                            egui::Rect::from_min_size(
                                data.min + egui::vec2(0.0, 24.0),
                                egui::vec2(data.width(), 24.0),
                            ),
                            egui::TextEdit::singleline(&mut item.comment)
                                .hint_text("注释")
                                .text_color(palette::MUTED)
                                .background_color(Color32::from_rgb(0xfa, 0xfb, 0xfb))
                                .margin(egui::vec2(5.0, 2.0))
                                .vertical_align(egui::Align::Center),
                        );
                        if wide {
                            let order = column(ORDER_COLUMN);
                            ui.put(
                                egui::Rect::from_center_size(order.center(), egui::vec2(ORDER_COLUMN - 6.0, 24.0)),
                                egui::DragValue::new(&mut item.sort_order).range(0..=9999),
                            )
                            .on_hover_text("发送序号，0 不参与发送");
                            let delay = column(DELAY_COLUMN);
                            ui.put(
                                egui::Rect::from_center_size(delay.center(), egui::vec2(DELAY_COLUMN - 6.0, 24.0)),
                                egui::DragValue::new(&mut item.delay).range(0..=600000),
                            )
                            .on_hover_text("发送延时（毫秒）");
                        }
                        let actions = column(actions_column);
                        let hint = if item.comment.trim().is_empty() {
                            "发送此条".to_owned()
                        } else {
                            item.comment.clone()
                        };
                        if narrow {
                            let rect = egui::Rect::from_center_size(actions.center(), egui::vec2(48.0, 24.0));
                            if ui
                                .put(rect, |ui: &mut egui::Ui| labeled_icon_button(ui, "send", "发送", rect.size()))
                                .on_hover_text(hint)
                                .clicked()
                            {
                                send = Some(item.clone());
                            }
                            return;
                        }
                        let buttons = [
                            ("send", hint.as_str(), true),
                            ("up", "上移", index > 0),
                            ("down", "下移", index + 1 < count),
                            ("trash", "删除", true),
                        ];
                        let start = actions.center().x - (4.0 * 21.0 + 3.0) / 2.0;
                        for (slot, (icon, tip, enabled)) in buttons.into_iter().enumerate() {
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(start + slot as f32 * 22.0, actions.center().y - 10.5),
                                egui::vec2(21.0, 21.0),
                            );
                            let clicked = ui
                                .put(rect, |ui: &mut egui::Ui| {
                                    ui.add_enabled_ui(enabled, |ui| {
                                        icon_button_sized(ui, icon, tip, true, false, 21.0)
                                    })
                                    .inner
                                })
                                .clicked();
                            if clicked {
                                match icon {
                                    "send" => send = Some(item.clone()),
                                    "up" => move_item = Some((index, index - 1)),
                                    "down" => move_item = Some((index, index + 1)),
                                    _ => remove = Some(index),
                                }
                            }
                        }
                    });
                }
                if self.extended.items.is_empty() {
                    ui.add_space(28.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("暂无扩展发送条目").color(Color32::from_rgb(0x7b, 0x85, 0x8d)));
                    });
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
        let footer = egui::Frame::new()
            .fill(palette::BAR)
            .inner_margin(egui::Margin::symmetric(6, 4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(FOOTER_HEIGHT - 8.0);
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if !tiny {
                        check(ui, &mut self.extended.settings.multi_send, "多条发送");
                        check(ui, &mut self.extended.settings.loop_send, "循环发送");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !tiny {
                            let (rect, response) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                            paint_icon(ui.painter(), rect.center(), "help", Color32::from_rgb(0x69, 0x74, 0x7d));
                            response.on_hover_text(
                                "ASCII 支持 \\r、\\n、\\t、\\0、\\\\、\\xNN 转义；序号 0 不参与发送；序号决定发送顺序；延时单位为毫秒",
                            );
                        }
                        let (icon, label) = if self.batch.is_empty() {
                            ("play", "启动发送")
                        } else {
                            ("stop", "停止")
                        };
                        if command_button(ui, icon, label, palette::GREEN, palette::GREEN_BORDER, 27.0).clicked() {
                            self.start_batch();
                        }
                    });
                });
            });
        let rect = footer.response.rect;
        ui.painter().hline(
            rect.x_range(),
            rect.top() + 0.5,
            egui::Stroke::new(1.0_f32, palette::TOOLBAR_LINE),
        );
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
            self.error = if self.config.transport_mode == "probe" {
                "连接超时：调试探针无响应，可能正被其他程序（其他 Z_COM 实例、Keil、J-Link Commander 等）占用。串口等其他连接不受影响".into()
            } else {
                "连接超时".into()
            };
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
        egui::TopBottomPanel::top("connection")
            .exact_height(44.0)
            .frame(
                egui::Frame::new()
                    .fill(palette::BAR)
                    .inner_margin(egui::Margin::symmetric(7, 5)),
            )
            .show(ctx, |ui| self.connection_ui(ui));
        if !self.error.is_empty() {
            egui::TopBottomPanel::top("error")
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(0xf7, 0xe4, 0xe5))
                        .inner_margin(egui::Margin::symmetric(9, 4)),
                )
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.set_min_height(21.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icon_button_sized(ui, "minus", "关闭", true, false, 22.0).clicked() {
                                self.error.clear();
                            }
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&self.error)
                                            .color(Color32::from_rgb(0x84, 0x2b, 0x32)),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                    });
                });
        }
        egui::TopBottomPanel::bottom("status")
            .exact_height(25.0)
            .frame(
                egui::Frame::new()
                    .fill(palette::WINDOW)
                    .inner_margin(egui::Margin::symmetric(8, 0)),
            )
            .show(ctx, |ui| self.status_ui(ui));
        let screen = ctx.screen_rect();
        // 与原版一致按窗口比例布局，分隔条自绘，拖动时直接修改比例
        let send_height = (self.config.send_panel_ratio * (screen.height() - 69.0))
            .clamp(96.0, (screen.height() * 0.5).max(96.0));
        egui::TopBottomPanel::bottom("send")
            .resizable(false)
            .exact_height(send_height)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(palette::WINDOW).inner_margin(egui::Margin {
                left: 4,
                right: 4,
                top: 5,
                bottom: 4,
            }))
            .show(ctx, |ui| {
                let content = ui.max_rect();
                let splitter = egui::Rect::from_min_max(
                    egui::pos2(content.left(), content.top() - 5.0),
                    egui::pos2(content.right(), content.top()),
                );
                if let Some(pointer) = splitter_ui(ui, splitter, "send-splitter", false) {
                    self.config.send_panel_ratio = ((content.bottom() + 4.0 - pointer.y)
                        / (screen.height() - 69.0).max(1.0))
                    .clamp(0.12, 0.5);
                }
                pane_frame().inner_margin(4.0).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    self.send_ui(ui);
                });
            });
        let sidebar = self.config.preset_panel_visible;
        if sidebar {
            let extended_width = (screen.width() * self.config.extended_panel_ratio)
                .clamp(60.0, (screen.width() * 0.7).max(60.0));
            egui::SidePanel::right("extended")
                .resizable(false)
                .exact_width(extended_width)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(palette::WINDOW).inner_margin(egui::Margin {
                    left: 5,
                    right: 4,
                    top: 4,
                    bottom: 0,
                }))
                .show(ctx, |ui| {
                    let content = ui.max_rect();
                    let splitter = egui::Rect::from_min_max(
                        egui::pos2(content.left() - 5.0, content.top()),
                        egui::pos2(content.left(), content.bottom()),
                    );
                    if let Some(pointer) = splitter_ui(ui, splitter, "extended-splitter", true) {
                        self.config.extended_panel_ratio =
                            ((content.right() + 4.0 - pointer.x) / screen.width().max(1.0)).clamp(0.05, 0.7);
                    }
                    pane_frame().show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        self.extended_ui(ui);
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(palette::WINDOW).inner_margin(egui::Margin {
                left: 4,
                right: if sidebar { 0 } else { 4 },
                top: 4,
                bottom: 0,
            }))
            .show(ctx, |ui| {
                pane_frame().show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    self.receive_ui(ui);
                });
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

/// 原 WebView 版本的配色（Windows 经典风格）
mod palette {
    use eframe::egui::Color32;
    pub const TEXT: Color32 = Color32::from_rgb(0x20, 0x20, 0x20);
    pub const LABEL: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);
    pub const WINDOW: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
    pub const BAR: Color32 = Color32::from_rgb(0xf5, 0xf5, 0xf5);
    pub const BAR_LINE: Color32 = Color32::from_rgb(0xb8, 0xb8, 0xb8);
    pub const PANE_BORDER: Color32 = Color32::from_rgb(0xa9, 0xa9, 0xa9);
    pub const TOOLBAR_LINE: Color32 = Color32::from_rgb(0xc8, 0xc8, 0xc8);
    pub const ROW_LINE: Color32 = Color32::from_rgb(0xe0, 0xe4, 0xe7);
    pub const BUTTON_BORDER: Color32 = Color32::from_rgb(0xad, 0xad, 0xad);
    pub const HOVER: Color32 = Color32::from_rgb(0xe5, 0xf1, 0xfb);
    pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x78, 0xd4);
    pub const ACTIVE: Color32 = Color32::from_rgb(0xe1, 0xe1, 0xe1);
    pub const ACTIVE_BORDER: Color32 = Color32::from_rgb(0x77, 0x77, 0x77);
    pub const RED: Color32 = Color32::from_rgb(0xf4, 0x43, 0x36);
    pub const RED_BORDER: Color32 = Color32::from_rgb(0xd7, 0x35, 0x2b);
    pub const AMBER: Color32 = Color32::from_rgb(0xd9, 0x77, 0x06);
    pub const AMBER_BORDER: Color32 = Color32::from_rgb(0xb9, 0x61, 0x00);
    pub const GREEN: Color32 = Color32::from_rgb(0x4c, 0xaf, 0x50);
    pub const GREEN_BORDER: Color32 = Color32::from_rgb(0x42, 0x98, 0x46);
    pub const CHECK_GREEN: Color32 = Color32::from_rgb(0x16, 0x86, 0x5a);
    pub const ICON: Color32 = Color32::from_rgb(0x3c, 0x3c, 0x3c);
    pub const MUTED: Color32 = Color32::from_rgb(0x66, 0x70, 0x78);
}

/// 原版常用波特率列表
const DEFAULT_BAUDRATES: &[&str] = &[
    "110", "300", "600", "1200", "2400", "4800", "9600", "14400", "19200", "38400", "56000",
    "57600", "115200", "128000", "230400", "256000", "460800", "500000", "512000", "600000",
    "750000", "921600", "1000000", "1500000", "2000000", "3000000", "4000000",
];

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(palette::LABEL));
}

/// 白底 30px 高的下拉框，对应原版 `select`
fn select_box(
    ui: &mut egui::Ui,
    id: &str,
    width: f32,
    selected: String,
    contents: impl FnOnce(&mut egui::Ui),
) {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 30.0;
        ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::WHITE;
        ui.visuals_mut().widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0_f32, palette::BAR_LINE);
        egui::ComboBox::from_id_salt(id)
            .width(width)
            .height(320.0)
            .truncate()
            .selected_text(selected)
            .show_ui(ui, contents);
    });
}

fn bold_family() -> egui::FontFamily {
    egui::FontFamily::Name("bold".into())
}

/// 粗体文字（标题、表头），对应原版 `strong`
fn bold(text: &str) -> egui::RichText {
    egui::RichText::new(text).family(bold_family())
}

/// 普通底色、图标加文字的按钮（发送）
fn labeled_icon_button(ui: &mut egui::Ui, icon: &str, label: &str, size: egui::Vec2) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = ui.is_enabled();
    let hovered = response.hovered() && enabled;
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, if hovered { palette::HOVER } else { palette::BAR });
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, if hovered { palette::ACCENT } else { palette::BUTTON_BORDER }),
        egui::StrokeKind::Inside,
    );
    let color = palette::TEXT;
    let galley = painter.layout_no_wrap(label.to_owned(), egui::FontId::proportional(13.0), color);
    let total = 16.0 + 6.0 + galley.size().x;
    let left = rect.center().x - total / 2.0;
    paint_icon(painter, egui::pos2(left + 8.0, rect.center().y), icon, color);
    painter.galley(
        egui::pos2(left + 22.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    response
}

/// 5px 分隔条（原版 `.extended-splitter` / `.send-splitter`），拖动时返回指针位置
fn splitter_ui(ui: &mut egui::Ui, rect: egui::Rect, id: &str, vertical: bool) -> Option<egui::Pos2> {
    let response = ui.interact(rect, ui.id().with(id), egui::Sense::drag());
    let highlight = response.hovered() || response.dragged();
    ui.painter().rect_filled(
        rect,
        0.0,
        if highlight {
            palette::ACCENT
        } else {
            Color32::from_rgb(0xd5, 0xda, 0xdd)
        },
    );
    if highlight {
        ui.ctx().set_cursor_icon(if vertical {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::ResizeVertical
        });
    }
    response
        .dragged()
        .then(|| response.interact_pointer_pos())
        .flatten()
}

fn apply_style(context: &egui::Context) {
    context.style_mut(|style| {
        for (text_style, size) in [
            (egui::TextStyle::Body, 13.0),
            (egui::TextStyle::Button, 13.0),
            (egui::TextStyle::Small, 11.0),
        ] {
            style
                .text_styles
                .insert(text_style, egui::FontId::proportional(size));
        }
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(12.5));
        style.spacing.item_spacing = egui::vec2(7.0, 4.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.icon_width = 14.0;
        style.spacing.icon_spacing = 5.0;
        let visuals = &mut style.visuals;
        visuals.panel_fill = palette::WINDOW;
        visuals.window_fill = Color32::WHITE;
        visuals.extreme_bg_color = Color32::WHITE;
        visuals.selection.bg_fill = Color32::from_rgb(0xcc, 0xe4, 0xf7);
        visuals.selection.stroke = egui::Stroke::new(1.0_f32, palette::ACCENT);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, palette::BAR_LINE);
        let widgets = &mut visuals.widgets;
        widgets.inactive.weak_bg_fill = palette::BAR;
        widgets.inactive.bg_fill = Color32::WHITE;
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, palette::BUTTON_BORDER);
        for (state, fill, border) in [
            (&mut widgets.hovered, palette::HOVER, palette::ACCENT),
            (
                &mut widgets.active,
                Color32::from_rgb(0xcc, 0xe4, 0xf7),
                Color32::from_rgb(0x00, 0x54, 0x99),
            ),
            (&mut widgets.open, palette::HOVER, palette::ACCENT),
        ] {
            state.weak_bg_fill = fill;
            state.bg_fill = fill;
            state.bg_stroke = egui::Stroke::new(1.0_f32, border);
            state.expansion = 0.0;
        }
        for state in [
            &mut widgets.noninteractive,
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            state.corner_radius = egui::CornerRadius::same(2);
            state.fg_stroke.color = palette::TEXT;
        }
    });
}

/// 白色面板（接收区、扩展发送、发送区）
fn pane_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(egui::Stroke::new(1.0_f32, palette::PANE_BORDER))
}

/// 面板顶部 33px 灰色工具栏，对应原版 `.pane-toolbar`
fn pane_toolbar(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    let response = egui::Frame::new()
        .fill(palette::BAR)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(29.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                contents(ui);
            });
        })
        .response;
    ui.painter().hline(
        response.rect.x_range(),
        response.rect.bottom() - 0.5,
        egui::Stroke::new(1.0_f32, palette::TOOLBAR_LINE),
    );
}

/// 分段切换按钮组，对应原版 `.mode-switch`；`compact` 对应 `.mode-switch.compact`
fn mode_switch(
    ui: &mut egui::Ui,
    value: &mut String,
    options: &[(&str, &str)],
    compact: bool,
) -> bool {
    let (height, size, padding) = if compact {
        (24.0, 12.0, 8.0)
    } else {
        (29.0, 13.0, 10.0)
    };
    let galleys: Vec<_> = options
        .iter()
        .map(|(_, label)| {
            ui.painter().layout_no_wrap(
                (*label).to_owned(),
                egui::FontId::proportional(size),
                palette::TEXT,
            )
        })
        .collect();
    let width = galleys
        .iter()
        .map(|galley| galley.size().x + padding * 2.0)
        .sum::<f32>()
        + options.len().saturating_sub(1) as f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let enabled = ui.is_enabled();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 2.0, palette::BAR);
    let mut changed = false;
    let mut left = rect.left();
    for (index, ((option, _), galley)) in options.iter().zip(galleys).enumerate() {
        let item = egui::Rect::from_min_size(
            egui::pos2(left, rect.top()),
            egui::vec2(galley.size().x + padding * 2.0, height),
        );
        let response = ui.interact(
            item,
            ui.id().with(("mode-switch", *option)),
            egui::Sense::click(),
        );
        let selected = value == option;
        if selected {
            painter.rect_filled(item, 0.0, palette::ACTIVE);
            painter.rect_stroke(
                item.shrink(1.0),
                0.0,
                egui::Stroke::new(1.0_f32, palette::ACTIVE_BORDER),
                egui::StrokeKind::Inside,
            );
        } else if response.hovered() && enabled {
            painter.rect_filled(item, 0.0, palette::HOVER);
        }
        painter.galley(item.center() - galley.size() / 2.0, galley, palette::TEXT);
        if index + 1 < options.len() {
            painter.vline(
                item.right() + 0.5,
                rect.y_range(),
                egui::Stroke::new(1.0_f32, palette::TOOLBAR_LINE),
            );
        }
        if response.clicked() && !selected {
            *value = (*option).to_owned();
            changed = true;
        }
        left = item.right() + 1.0;
    }
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, palette::BUTTON_BORDER),
        egui::StrokeKind::Inside,
    );
    changed
}

/// 复选框，选中时为强调色底白色对勾（与原版 Chromium 原生复选框一致）
fn check(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    check_styled(ui, value, label, palette::ACCENT, 14.0)
}

fn check_styled(
    ui: &mut egui::Ui,
    value: &mut bool,
    label: &str,
    accent: Color32,
    size: f32,
) -> egui::Response {
    let galley = (!label.is_empty()).then(|| {
        ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(13.0),
            palette::TEXT,
        )
    });
    let text_width = galley.as_ref().map_or(0.0, |galley| galley.size().x + 5.0);
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(size + text_width, size.max(20.0)),
        egui::Sense::click(),
    );
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let enabled = ui.is_enabled();
    let painter = ui.painter();
    let check_box = egui::Rect::from_center_size(
        egui::pos2(rect.left() + size / 2.0, rect.center().y),
        egui::vec2(size, size),
    );
    if *value {
        painter.rect_filled(check_box, 2.0, accent);
        let point = |x: f32, y: f32| check_box.left_top() + egui::vec2(x, y) * size;
        painter.add(egui::Shape::line(
            vec![point(0.22, 0.52), point(0.42, 0.72), point(0.78, 0.30)],
            egui::Stroke::new(1.8_f32, Color32::WHITE),
        ));
    } else {
        let border = if response.hovered() && enabled {
            palette::ACCENT
        } else {
            Color32::from_rgb(0x76, 0x76, 0x76)
        };
        painter.rect_filled(check_box, 2.0, Color32::WHITE);
        painter.rect_stroke(
            check_box,
            2.0,
            egui::Stroke::new(1.0_f32, border),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(galley) = galley {
        let position = egui::pos2(
            check_box.right() + 5.0,
            rect.center().y - galley.size().y / 2.0,
        );
        painter.galley(position, galley, palette::TEXT);
    }
    response
}

/// 带图标的实心命令按钮（连接 / 断开 / 启动发送），对应原版 `.command-button` / `.run-button`
fn command_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    fill: Color32,
    border: Color32,
    height: f32,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(13.0),
        Color32::WHITE,
    );
    let width = 12.0 + 16.0 + 6.0 + galley.size().x + 12.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let darken = response.hovered() && ui.is_enabled();
    let shade = |color: Color32| {
        if darken {
            Color32::from_rgb(
                (color.r() as f32 * 0.92) as u8,
                (color.g() as f32 * 0.92) as u8,
                (color.b() as f32 * 0.92) as u8,
            )
        } else {
            color
        }
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, shade(fill));
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, shade(border)),
        egui::StrokeKind::Inside,
    );
    paint_icon(
        painter,
        egui::pos2(rect.left() + 20.0, rect.center().y),
        icon,
        Color32::WHITE,
    );
    painter.galley(
        egui::pos2(rect.left() + 34.0, rect.center().y - galley.size().y / 2.0),
        galley,
        Color32::WHITE,
    );
    response
}

/// 30x30 图标按钮；`subtle` 时无边框，对应原版 `.icon-button.subtle`
fn icon_button(ui: &mut egui::Ui, icon: &str, hint: &str, subtle: bool) -> egui::Response {
    icon_button_sized(ui, icon, hint, subtle, false, 30.0)
}

fn icon_button_sized(
    ui: &mut egui::Ui,
    icon: &str,
    hint: &str,
    subtle: bool,
    active: bool,
    size: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let enabled = ui.is_enabled();
    let hovered = response.hovered() && enabled;
    let painter = ui.painter();
    let frame = if active {
        Some((palette::ACTIVE, palette::ACTIVE_BORDER))
    } else if hovered {
        Some((palette::HOVER, palette::ACCENT))
    } else if subtle {
        None
    } else {
        Some((palette::BAR, palette::BUTTON_BORDER))
    };
    if let Some((fill, border)) = frame {
        painter.rect_filled(rect, 2.0, fill);
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0_f32, border),
            egui::StrokeKind::Inside,
        );
    }
    paint_icon(painter, rect.center(), icon, palette::ICON);
    response.on_hover_text(hint)
}

/// 线框图标，形状参考原版使用的 Lucide 图标（16px 网格）
fn paint_icon(painter: &egui::Painter, center: egui::Pos2, icon: &str, color: Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let point = |x: f32, y: f32| center + egui::vec2(x, y);
    let line = |coordinates: &[(f32, f32)]| {
        painter.add(egui::Shape::line(
            coordinates.iter().map(|&(x, y)| point(x, y)).collect(),
            stroke,
        ));
    };
    let arc = |cx: f32, cy: f32, radius: f32, from: f32, to: f32| -> Vec<egui::Pos2> {
        (0..=16)
            .map(|step| {
                let angle = (from + (to - from) * step as f32 / 16.0).to_radians();
                point(cx + radius * angle.cos(), cy + radius * angle.sin())
            })
            .collect()
    };
    let stadium = |a: (f32, f32), b: (f32, f32), radius: f32| {
        let angle = (b.1 - a.1).atan2(b.0 - a.0).to_degrees();
        let mut points = arc(b.0, b.1, radius, angle - 90.0, angle + 90.0);
        points.extend(arc(a.0, a.1, radius, angle + 90.0, angle + 270.0));
        points.push(points[0]);
        painter.add(egui::Shape::line(points, stroke));
    };
    match icon {
        "refresh" => {
            painter.add(egui::Shape::line(arc(0.0, 0.0, 6.0, 200.0, 340.0), stroke));
            painter.add(egui::Shape::line(arc(0.0, 0.0, 6.0, 20.0, 160.0), stroke));
            line(&[(2.5, -2.3), (5.6, -2.0), (6.1, -5.4)]);
            line(&[(-2.5, 2.3), (-5.6, 2.0), (-6.1, 5.4)]);
        }
        "clear" => {
            line(&[
                (-6.5, 1.5),
                (0.0, -5.0),
                (6.5, 1.5),
                (2.0, 6.0),
                (-2.0, 6.0),
                (-6.5, 1.5),
            ]);
            line(&[(-3.0, -1.5), (3.0, 4.5)]);
            line(&[(2.0, 6.0), (7.0, 6.0)]);
        }
        "settings" => {
            painter.circle_stroke(center, 4.8, stroke);
            painter.circle_stroke(center, 1.8, stroke);
            for index in 0..8 {
                let angle = index as f32 * std::f32::consts::TAU / 8.0;
                let direction = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment(
                    [center + direction * 4.8, center + direction * 7.2],
                    egui::Stroke::new(2.2_f32, color),
                );
            }
        }
        "help" => {
            painter.circle_stroke(center, 7.0, stroke);
            painter.add(egui::Shape::line(arc(0.0, -1.6, 2.2, 180.0, 400.0), stroke));
            line(&[(1.7, -0.2), (0.0, 1.0), (0.0, 1.9)]);
            painter.circle_filled(point(0.0, 4.1), 0.9, color);
        }
        "chevron-right" => line(&[(-2.0, -5.0), (3.0, 0.0), (-2.0, 5.0)]),
        "chevron-left" => line(&[(2.0, -5.0), (-3.0, 0.0), (2.0, 5.0)]),
        "chevron-down" => line(&[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]),
        "add" => {
            line(&[(-6.0, 0.0), (6.0, 0.0)]);
            line(&[(0.0, -6.0), (0.0, 6.0)]);
        }
        "minus" => line(&[(-5.0, 0.0), (5.0, 0.0)]),
        "upload" => {
            line(&[(0.0, 3.0), (0.0, -6.5)]);
            line(&[(-3.5, -3.0), (0.0, -6.5), (3.5, -3.0)]);
            line(&[(-6.5, 3.0), (-6.5, 6.5), (6.5, 6.5), (6.5, 3.0)]);
        }
        "download" => {
            line(&[(0.0, -6.5), (0.0, 3.0)]);
            line(&[(-3.5, -0.5), (0.0, 3.0), (3.5, -0.5)]);
            line(&[(-6.5, 3.0), (-6.5, 6.5), (6.5, 6.5), (6.5, 3.0)]);
        }
        "up" => {
            line(&[(0.0, 5.5), (0.0, -5.5)]);
            line(&[(-4.5, -1.0), (0.0, -5.5), (4.5, -1.0)]);
        }
        "down" => {
            line(&[(0.0, -5.5), (0.0, 5.5)]);
            line(&[(-4.5, 1.0), (0.0, 5.5), (4.5, 1.0)]);
        }
        "trash" => {
            line(&[(-6.0, -4.0), (6.0, -4.0)]);
            line(&[(-2.5, -4.0), (-2.5, -6.0), (2.5, -6.0), (2.5, -4.0)]);
            line(&[(-4.8, -4.0), (-4.0, 6.0), (4.0, 6.0), (4.8, -4.0)]);
            line(&[(-1.5, -1.0), (-1.5, 3.5)]);
            line(&[(1.5, -1.0), (1.5, 3.5)]);
        }
        "link" => {
            stadium((-4.2, 4.2), (-0.8, 0.8), 2.6);
            stadium((0.8, -0.8), (4.2, -4.2), 2.6);
        }
        "unlink" => {
            stadium((-4.6, 4.6), (-2.2, 2.2), 2.4);
            stadium((2.2, -2.2), (4.6, -4.6), 2.4);
            line(&[(-1.0, -6.5), (-1.0, -4.5)]);
            line(&[(-6.5, -1.0), (-4.5, -1.0)]);
            line(&[(1.0, 6.5), (1.0, 4.5)]);
            line(&[(6.5, 1.0), (4.5, 1.0)]);
        }
        "play" => {
            painter.add(egui::Shape::convex_polygon(
                vec![point(-3.5, -5.5), point(5.5, 0.0), point(-3.5, 5.5)],
                color,
                egui::Stroke::NONE,
            ));
        }
        "stop" => {
            painter.rect_filled(
                egui::Rect::from_center_size(center, egui::vec2(9.0, 9.0)),
                1.0,
                color,
            );
        }
        _ => {
            // Lucide Send：纸飞机
            line(&[
                (-6.5, -0.5),
                (6.5, -6.5),
                (0.5, 6.5),
                (-0.8, 0.8),
                (-6.5, -0.5),
            ]);
            line(&[(-0.8, 0.8), (6.5, -6.5)]);
        }
    }
}

/// 千分位格式，与原版 `toLocaleString()` 一致
fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(character);
    }
    output
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

/// 与原 WebView 版本的 CSS 字体栈保持一致：界面用 Segoe UI + 微软雅黑 UI，接收区用 Cascadia Mono / Consolas。
/// 末尾的 Segoe UI Symbol 等提供 ␍␊◆ 等控制图符，避免显示为方块。
fn font_definitions() -> egui::FontDefinitions {
    const UI: &[&str] = &["Segoe UI", "SF Pro Text", "Noto Sans", "DejaVu Sans"];
    const MONO: &[&str] = &[
        "Cascadia Mono",
        "Consolas",
        "Menlo",
        "DejaVu Sans Mono",
        "Noto Sans Mono",
    ];
    const CJK: &[&str] = &[
        "Microsoft YaHei UI",
        "Microsoft YaHei",
        "PingFang SC",
        "Noto Sans CJK SC",
        "WenQuanYi Micro Hei",
    ];
    const SYMBOL: &[&str] = &["Segoe UI Symbol", "Apple Symbols", "DejaVu Sans"];

    let mut database = fontdb::Database::new();
    database.load_system_fonts();
    let mut fonts = egui::FontDefinitions::default();
    let mut load = |key: &str, families: &[&str], weight: fontdb::Weight| -> Option<String> {
        let names: Vec<_> = families
            .iter()
            .map(|name| fontdb::Family::Name(name))
            .collect();
        let face = database.query(&fontdb::Query {
            families: &names,
            weight,
            ..Default::default()
        })?;
        database.with_face_data(face, |data, index| {
            let mut font = egui::FontData::from_owned(data.to_vec());
            font.index = index;
            fonts.font_data.insert(key.into(), Arc::new(font));
        })?;
        Some(key.into())
    };
    let ui = load("ui", UI, fontdb::Weight::NORMAL);
    let ui_bold = load("ui-bold", UI, fontdb::Weight::BOLD);
    let mono = load("mono", MONO, fontdb::Weight::NORMAL);
    let cjk = load("cjk", CJK, fontdb::Weight::NORMAL);
    let cjk_bold = load("cjk-bold", CJK, fontdb::Weight::BOLD);
    let symbol = load("symbol", SYMBOL, fontdb::Weight::NORMAL);
    let proportional = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let bold_chain = [ui_bold, cjk_bold, ui.clone(), cjk.clone(), symbol.clone()]
        .into_iter()
        .flatten()
        .chain(proportional)
        .collect();
    fonts.families.insert(bold_family(), bold_chain);
    for (family, primary) in [
        (egui::FontFamily::Proportional, ui),
        (egui::FontFamily::Monospace, mono),
    ] {
        let defaults = fonts.families.remove(&family).unwrap_or_default();
        let chain = [primary, cjk.clone(), symbol.clone()]
            .into_iter()
            .flatten()
            .chain(defaults)
            .collect();
        fonts.families.insert(family, chain);
    }
    fonts
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
