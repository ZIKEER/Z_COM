//! 数据图表：按规则从报文中提取数值，以一天内的时间为横轴绘制曲线并汇总表格。
//!
//! 实时数据与导入的日志各自保存一份数据集；横轴范围 0–24 时，跨天时实时数据清空重画。

use chrono::{NaiveDate, NaiveDateTime, Timelike};
use eframe::egui::{self, Color32};
use regex::Regex;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
};

const DAY: f64 = 86_400.0;
/// 单条曲线最多保留的点数；约 32 MB，超出时丢弃最早的 10%
const MAX_POINTS: usize = 2_000_000;
const MIN_SPAN: f64 = 1.0;
const DEFAULT_SPAN: f64 = 600.0;

/// 规则中的一个字段：`{名称}`、`{名称:类型}`、`{名称:单位}` 或 `{名称:类型:单位}`
#[derive(Clone, Debug, PartialEq)]
struct Field {
    name: String,
    hex: bool,
    unit: String,
}

/// 图表窗口的显示方式，持久化在配置中
pub(crate) const VIEWS: &[(&str, &str)] = &[
    ("plot", "曲线"),
    ("table", "表格"),
    ("both", "曲线+表格"),
];

/// 编译后的规则与半包缓冲；实时接收与日志导入各用一份
#[derive(Clone, Default)]
struct Parser {
    rules: Vec<(Regex, Vec<Field>)>,
    /// 按规则中首次出现的顺序排列的字段，曲线配色与表格行序都以此为准
    fields: Vec<Field>,
    buffer: String,
}

impl Parser {
    fn compile(template: &str) -> Result<Self, String> {
        let placeholder = Regex::new(r"\{([^{}:]+)(?::([^{}:]*))?(?::([^{}]*))?\}").unwrap();
        let mut rules = Vec::new();
        let mut ordered: Vec<Field> = Vec::new();
        for template in template.lines().map(str::trim).filter(|line| !line.is_empty()) {
            let mut pattern = String::new();
            let mut cursor = 0;
            let mut fields = Vec::new();
            for capture in placeholder.captures_iter(template) {
                let entire = capture.get(0).unwrap();
                pattern.push_str(&literal(&template[cursor..entire.start()]));
                let name = capture[1].trim();
                if name.is_empty() {
                    return Err("字段名不能为空".into());
                }
                let second = capture.get(2).map_or("", |value| value.as_str().trim());
                let third = capture.get(3).map(|value| value.as_str().trim());
                // 第二段是类型时第三段为单位；否则第二段本身就是单位（`{温度:℃}`）
                let (kind, unit) = match (second, third) {
                    ("" | "number" | "int" | "hex" | "hex0x", unit) => {
                        (if second.is_empty() { "number" } else { second }, unit.unwrap_or(""))
                    }
                    (unit, None) => ("number", unit),
                    (other, Some(_)) => {
                        return Err(format!(
                            "字段 {name} 的类型 {other} 无效，可用 number、int、hex、hex0x"
                        ));
                    }
                };
                let field = Field {
                    name: name.to_owned(),
                    hex: kind.starts_with("hex"),
                    unit: unit.to_owned(),
                };
                match ordered.iter_mut().find(|known| known.name == field.name) {
                    // 同名字段在多条规则中出现时，以首次声明的单位为准，后续规则可省略单位
                    Some(known) if known.unit.is_empty() => known.unit = field.unit.clone(),
                    Some(_) => {}
                    None => ordered.push(field.clone()),
                }
                fields.push(field);
                pattern.push_str(match kind {
                    "hex0x" => "(0[xX][0-9a-fA-F]+)",
                    "hex" => "(0[xX][0-9a-fA-F]+|[0-9a-fA-F]+)",
                    "int" => "(-?\\d+)",
                    _ => "(-?\\d+(?:\\.\\d+)?(?:[eE][+-]?\\d+)?)",
                });
                cursor = entire.end();
            }
            pattern.push_str(&literal(&template[cursor..]));
            if fields.is_empty() {
                return Err("模板中至少需要一个 {字段名} 占位符".into());
            }
            rules.push((
                Regex::new(&pattern).map_err(|error| error.to_string())?,
                fields,
            ));
        }
        Ok(Self {
            rules,
            fields: ordered,
            buffer: String::new(),
        })
    }

    /// 追加报文并解析完整记录；半包留在缓冲中等待后续数据。返回解析出的记录数
    fn feed(&mut self, bytes: &[u8], mut emit: impl FnMut(&Field, f64)) -> usize {
        if self.rules.is_empty() {
            return 0;
        }
        self.buffer.push_str(&String::from_utf8_lossy(bytes));
        let mut consumed = 0;
        let mut records = 0;
        for (matcher, fields) in &self.rules {
            for captures in matcher.captures_iter(&self.buffer) {
                if captures.get(0).unwrap().end() == self.buffer.len()
                    && !self.buffer.ends_with(['\r', '\n'])
                {
                    continue;
                }
                records += 1;
                for (index, field) in fields.iter().enumerate() {
                    let raw = &captures[index + 1];
                    let value = if field.hex {
                        i64::from_str_radix(
                            raw.trim_start_matches("0x").trim_start_matches("0X"),
                            16,
                        )
                        .map(|value| value as f64)
                        .ok()
                    } else {
                        raw.parse::<f64>().ok()
                    };
                    if let Some(value) = value.filter(|value| value.is_finite()) {
                        emit(field, value);
                    }
                }
                consumed = consumed.max(captures.get(0).unwrap().end());
            }
        }
        self.buffer.drain(..consumed);
        if self.buffer.len() > 65536 {
            self.buffer.clear();
        }
        records
    }
}

/// 一天的数据：每条曲线按时间递增保存 `[一天内秒数, 数值]`
#[derive(Default)]
struct Dataset {
    date: Option<NaiveDate>,
    series: BTreeMap<String, Vec<[f64; 2]>>,
    /// 每分钟解析出的记录数，用于全天时间轴的数据密度
    minutes: Vec<u32>,
    last_time: Option<f64>,
    /// 数据变化计数，用于判断绘制缓存是否失效
    version: u64,
}

impl Dataset {
    fn clear(&mut self, date: Option<NaiveDate>) {
        *self = Self {
            date,
            version: self.version + 1,
            ..Default::default()
        };
    }

    fn push(&mut self, name: &str, time: f64, value: f64) {
        let series = self.series.entry(name.to_owned()).or_default();
        // 系统时间回拨时保持单调，保证二分查找有效
        let time = series.last().map_or(time, |last| time.max(last[0]));
        series.push([time, value]);
        if series.len() > MAX_POINTS {
            series.drain(..MAX_POINTS / 10);
        }
        self.last_time = Some(self.last_time.map_or(time, |last| last.max(time)));
        self.version += 1;
    }

    fn mark(&mut self, time: f64, records: usize) {
        if records == 0 {
            return;
        }
        if self.minutes.is_empty() {
            self.minutes = vec![0; 1440];
        }
        let minute = ((time / 60.0) as usize).min(1439);
        self.minutes[minute] = self.minutes[minute].saturating_add(records as u32);
    }

    fn ingest(&mut self, parser: &mut Parser, bytes: &[u8], time: f64) {
        let mut values = Vec::new();
        let records = parser.feed(bytes, |field, value| values.push((field.name.clone(), value)));
        for (name, value) in values {
            self.push(&name, time, value);
        }
        self.mark(time, records);
    }
}

struct History {
    name: String,
    path: PathBuf,
    data: Dataset,
}

type ImportResult = Result<Option<History>, String>;

#[derive(Clone, Copy)]
enum TimelineDrag {
    Start,
    End,
    Move(f64),
    Create(f64),
}

type CacheKey = (u64, bool, u64, u64);

struct Visible {
    key: (CacheKey, usize),
    lines: Vec<(String, Vec<[f64; 2]>)>,
    y: Option<(f64, f64)>,
}

/// 字段在所选范围内的统计：最新值、最小值、最大值、点数
type Stats = (f64, f64, f64, usize);

#[derive(Default)]
pub(crate) struct NativePlot {
    pub open: bool,
    pub template: String,
    pub view: String,
    /// 日志所在目录，作为导入对话框的初始目录
    pub log_directory: PathBuf,
    parser: Parser,
    live: Dataset,
    history: Option<History>,
    importing: Option<Receiver<ImportResult>>,
    paused: bool,
    error: String,
    notice: String,
    y_min: String,
    y_max: String,
    /// 当前显示的时间范围（一天内秒数）
    range: (f64, f64),
    follow: bool,
    initialized: bool,
    start_draft: String,
    end_draft: String,
    timeline_drag: Option<TimelineDrag>,
    plot_width: f32,
    visible: Option<Visible>,
    summary: Option<(CacheKey, BTreeMap<String, Stats>)>,
    rule_dialog: bool,
    rule_draft: String,
}

impl NativePlot {
    /// 调试预览：打开窗口并按当前规则填充一小时示例数据，便于截图检查布局
    pub fn preview(&mut self, rules: bool) {
        self.open = true;
        self.rule_dialog = rules;
        self.rule_draft = self.template.clone();
        if self.parser.rules.is_empty() {
            self.template =
                "t={温度:℃},v={电压:int:mV},i={电流:mA},r={寄存器:hex0x}\\r\\n".into();
            let _ = self.compile(None);
        }
        let now = chrono::Local::now().naive_local();
        for step in 0..720 {
            let phase = step as f64 / 60.0;
            let line = format!(
                "t={:.2},v={},i={:.1},r=0x{:02X}\r\n",
                25.0 + phase.sin() * 3.0,
                3300 + (phase.cos() * 40.0) as i64,
                120.0 + (phase * 0.7).sin() * 15.0,
                step % 256
            );
            let at = now - chrono::Duration::seconds((720 - step) * 5);
            if at.date() == now.date() {
                self.consume(line.as_bytes(), at);
            }
        }
    }

    /// 实时数据：窗口关闭时也持续记录，打开后可查看当天完整曲线
    pub fn consume(&mut self, bytes: &[u8], at: NaiveDateTime) {
        if self.paused || self.parser.rules.is_empty() {
            return;
        }
        let date = at.date();
        if self.live.date != Some(date) {
            self.live.clear(Some(date));
            self.parser.buffer.clear();
        }
        self.live.ingest(&mut self.parser, bytes, seconds_of_day(at));
    }

    /// 编译 `template`；失败时保留已生效的规则。已导入日志时按新规则重新解析
    pub fn compile(&mut self, context: Option<&egui::Context>) -> Result<(), String> {
        self.parser = Parser::compile(&self.template)?;
        if let (Some(path), Some(context)) = (
            self.history.as_ref().map(|history| history.path.clone()),
            context,
        ) {
            self.import(Some(path), context.clone());
        }
        Ok(())
    }

    fn active(&self) -> &Dataset {
        self.history.as_ref().map_or(&self.live, |history| &history.data)
    }

    fn cache_key(&self) -> CacheKey {
        (
            self.active().version,
            self.history.is_some(),
            self.range.0.to_bits(),
            self.range.1.to_bits(),
        )
    }

    fn field(&self, name: &str) -> Option<&Field> {
        self.parser.fields.iter().find(|field| field.name == name)
    }

    fn unit(&self, name: &str) -> &str {
        self.field(name).map_or("", |field| field.unit.as_str())
    }

    fn color(&self, name: &str) -> Color32 {
        let index = self
            .parser
            .fields
            .iter()
            .position(|field| field.name == name)
            .unwrap_or_else(|| {
                self.parser.fields.len()
                    + self.active().series.keys().position(|key| key == name).unwrap_or(0)
            });
        series_color(index)
    }

    /// 选择日志文件（`path` 为空时弹出对话框）并在后台解析
    fn import(&mut self, path: Option<PathBuf>, context: egui::Context) {
        let mut parser = self.parser.clone();
        parser.buffer.clear();
        let directory = self.log_directory.clone();
        let (sender, receiver) = mpsc::channel();
        self.importing = Some(receiver);
        std::thread::spawn(move || {
            let path = path.or_else(|| {
                rfd::FileDialog::new()
                    .set_title("导入日志")
                    .add_filter("Z_COM 日志", &["txt", "log"])
                    .set_directory(&directory)
                    .pick_file()
            });
            let result = match path {
                Some(path) => parse_log_file(&path, &mut parser).map(Some),
                None => Ok(None),
            };
            let _ = sender.send(result);
            context.request_repaint();
        });
    }

    fn poll_import(&mut self) {
        let Some(receiver) = &self.importing else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("导入线程异常退出".into()),
        };
        self.importing = None;
        match result {
            Ok(Some(history)) => {
                self.notice = if history.data.series.is_empty() {
                    "日志中没有匹配当前规则的数据".into()
                } else {
                    String::new()
                };
                let reimport = self
                    .history
                    .as_ref()
                    .is_some_and(|previous| previous.path == history.path);
                let first = history
                    .data
                    .series
                    .values()
                    .filter_map(|points| points.first())
                    .map(|point| point[0])
                    .fold(f64::INFINITY, f64::min);
                let last = history.data.last_time;
                self.history = Some(history);
                self.follow = false;
                // 首次导入时显示有数据的整段时间；规则变化后的重新解析保持当前范围
                if !reimport {
                    match (first.is_finite(), last) {
                        (true, Some(last)) => self.set_range(first, last.max(first + MIN_SPAN)),
                        _ => self.range = (0.0, DAY),
                    }
                }
            }
            Ok(None) => {}
            Err(error) => self.notice = error,
        }
    }

    fn back_to_live(&mut self) {
        self.history = None;
        self.notice.clear();
        self.follow = true;
        self.range = (0.0, DEFAULT_SPAN);
    }

    fn set_range(&mut self, start: f64, end: f64) {
        let span = (end - start).clamp(MIN_SPAN, DAY);
        let start = start.clamp(0.0, DAY - span);
        self.range = (start, start + span);
    }

    fn latest_or_now(&self) -> f64 {
        self.active().last_time.unwrap_or_else(|| {
            if self.history.is_some() {
                DAY
            } else {
                seconds_of_day(chrono::Local::now().naive_local())
            }
        })
    }

    /// 跟随模式下保持跨度不变，右端对齐最新数据
    fn apply_follow(&mut self) {
        if !self.follow || self.history.is_some() {
            return;
        }
        let span = self.range.1 - self.range.0;
        let latest = self.latest_or_now();
        self.set_range(latest - span, latest.max(span));
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if !VIEWS.iter().any(|(key, _)| *key == self.view) {
            self.view = "plot".into();
        }
        if !self.initialized {
            self.initialized = true;
            self.follow = true;
            self.range = (0.0, DEFAULT_SPAN);
        }
        self.poll_import();
        self.apply_follow();
        let show_plot = self.view != "table";
        let show_table = self.view != "plot";
        self.toolbar_ui(ui, show_plot);
        self.range_ui(ui);
        self.rule_dialog_ui(ui.ctx());
        egui::TopBottomPanel::bottom("plot-timeline")
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(0, 4)))
            .show_separator_line(false)
            .show_inside(ui, |ui| {
                self.timeline_ui(ui);
                self.status_ui(ui);
            });
        if show_plot && show_table {
            egui::SidePanel::right("plot-table")
                .resizable(true)
                .default_width(300.0)
                .width_range(200.0..=(ui.available_width() * 0.7).max(200.0))
                .show_inside(ui, |ui| self.table_ui(ui));
        }
        if show_plot {
            self.plot_ui(ui);
        } else {
            self.table_ui(ui);
        }
    }

    fn toolbar_ui(&mut self, ui: &mut egui::Ui, show_plot: bool) {
        ui.horizontal_wrapped(|ui| {
            super::app::mode_switch(ui, &mut self.view, VIEWS, true);
            ui.separator();
            if ui.button("规则配置").clicked() {
                self.rule_draft = self.template.clone();
                self.error.clear();
                self.rule_dialog = true;
            }
            let importing = self.importing.is_some();
            if ui
                .add_enabled(!importing, egui::Button::new(if importing { "导入中…" } else { "导入日志" }))
                .on_hover_text("选择一个 log_日期.txt，按当前规则解析接收数据并绘制当天曲线")
                .clicked()
            {
                if self.parser.rules.is_empty() {
                    self.notice = "请先在“规则配置”中添加解析规则".into();
                } else {
                    self.notice.clear();
                    self.import(None, ui.ctx().clone());
                }
            }
            if let Some(history) = &self.history {
                let label = format!(
                    "历史：{}{}",
                    history.name,
                    history
                        .data
                        .date
                        .map(|date| format!("（{date}）"))
                        .unwrap_or_default()
                );
                if ui
                    .button("返回实时")
                    .on_hover_text("实时数据在查看历史期间仍在后台记录")
                    .clicked()
                {
                    self.back_to_live();
                }
                ui.label(egui::RichText::new(label).color(Color32::from_rgb(0xb1, 0x5c, 0x00)));
            } else {
                if ui
                    .button(if self.paused { "继续" } else { "暂停" })
                    .on_hover_text("暂停期间不记录实时数据")
                    .clicked()
                {
                    self.paused = !self.paused;
                }
                if ui
                    .button("清除")
                    .on_hover_text("清除今天的实时曲线与表格数据")
                    .clicked()
                {
                    let date = self.live.date;
                    self.live.clear(date);
                    self.parser.buffer.clear();
                }
            }
            if show_plot {
                ui.separator();
                ui.label("Y 最小");
                ui.add(
                    egui::TextEdit::singleline(&mut self.y_min)
                        .desired_width(64.0)
                        .hint_text("自动"),
                );
                ui.label("Y 最大");
                ui.add(
                    egui::TextEdit::singleline(&mut self.y_max)
                        .desired_width(64.0)
                        .hint_text("自动"),
                );
            }
        });
    }

    fn range_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let mut apply = false;
            for (label, draft, id) in [
                ("开始", &mut self.start_draft, "plot-range-start"),
                ("结束", &mut self.end_draft, "plot-range-end"),
            ] {
                ui.label(label);
                let response = ui.add(
                    egui::TextEdit::singleline(draft)
                        .id(egui::Id::new(id))
                        .desired_width(92.0)
                        .hint_text("HH:MM:SS")
                        .font(egui::TextStyle::Monospace),
                );
                apply |= response.lost_focus();
            }
            if apply {
                match (parse_clock(&self.start_draft), parse_clock(&self.end_draft)) {
                    (Some(start), Some(end)) if end > start => {
                        self.follow = false;
                        self.set_range(start, end);
                        self.notice.clear();
                    }
                    _ => {
                        self.notice = "时间格式为 HH:MM[:SS]，结束时间需晚于开始时间".into()
                    }
                }
            }
            ui.separator();
            let latest = self.latest_or_now();
            for (label, span) in [("最近 10 分钟", 600.0), ("最近 1 小时", 3600.0)] {
                if ui.button(label).clicked() {
                    self.set_range(latest - span, latest.max(span));
                    self.follow = self.history.is_none();
                }
            }
            if ui.button("全天").clicked() {
                self.follow = false;
                self.range = (0.0, DAY);
            }
            if self.history.is_none() {
                super::app::check(ui, &mut self.follow, "跟随最新")
                    .on_hover_text("窗口右端对齐最新数据；拖动曲线或时间轴后自动关闭");
            }
            if !self.notice.is_empty() {
                ui.colored_label(Color32::from_rgb(0xb4, 0x23, 0x18), &self.notice);
            }
        });
        // 未在编辑时，输入框始终显示当前范围
        for (draft, id, value) in [
            (&mut self.start_draft, "plot-range-start", self.range.0),
            (&mut self.end_draft, "plot-range-end", self.range.1),
        ] {
            if !ui.memory(|memory| memory.has_focus(egui::Id::new(id))) {
                *draft = format_clock(value, false);
            }
        }
    }

    fn status_ui(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let data = self.active();
            let points: usize = data.series.values().map(Vec::len).sum();
            ui.weak(format!(
                "{} 个字段 · {} 个数据点{}",
                self.parser.fields.len(),
                points,
                data.date.map(|date| format!(" · {date}")).unwrap_or_default()
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(if self.history.is_some() {
                    "历史日志"
                } else if self.paused {
                    "已暂停"
                } else if self.follow {
                    "跟随最新"
                } else {
                    "实时记录中"
                });
            });
        });
    }

    fn rule_dialog_ui(&mut self, context: &egui::Context) {
        if !self.rule_dialog {
            return;
        }
        let response = egui::Modal::new(egui::Id::new("plot-rules"))
            .frame(
                super::app::dialog_frame(true)
                    .fill(Color32::WHITE)
                    .corner_radius(6.0),
            )
            .show(context, |ui| {
                ui.set_width(700.0_f32.min(context.screen_rect().width() - 48.0));
                super::app::dialog_header(ui, "规则配置", &mut self.rule_dialog);
                ui.label("每行一条规则，用 {字段名} 提取数值，曲线与表格共用这些规则。");
                ui.label("类型可选 :int、:hex、:hex0x；单位写在最后，如 {温度:℃}、{电压:int:mV}。支持 \\r \\n 转义。");
                ui.label(
                    egui::RichText::new("示例：temp={温度:℃},voltage={电压:int:mV}\\r\\n")
                        .monospace()
                        .color(Color32::from_rgb(0x7a, 0x3f, 0x00)),
                );
                ui.add(
                    egui::TextEdit::multiline(&mut self.rule_draft)
                        .desired_width(f32::INFINITY)
                        .desired_rows(12)
                        .min_size(egui::vec2(0.0, 220.0))
                        .font(egui::TextStyle::Monospace),
                );
                if !self.error.is_empty() {
                    ui.colored_label(Color32::DARK_RED, &self.error);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("保存规则").color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(22, 119, 255)),
                        )
                        .clicked()
                    {
                        let previous =
                            std::mem::replace(&mut self.template, self.rule_draft.clone());
                        match self.compile(Some(ui.ctx())) {
                            Ok(()) => {
                                self.error.clear();
                                self.rule_dialog = false;
                            }
                            // 编译失败时不替换已生效的规则
                            Err(error) => {
                                self.template = previous;
                                self.error = error;
                            }
                        }
                    }
                    if ui.button("取消").clicked() {
                        self.rule_dialog = false;
                    }
                });
            });
        if response.should_close() {
            self.rule_dialog = false;
        }
    }

    /// 可见范围内的曲线，按像素列降采样（每列保留首尾与最小、最大值，峰值不丢失）
    fn visible_lines(&mut self) -> &Visible {
        let columns = (self.plot_width.max(200.0) as usize).min(4000);
        let key = (self.cache_key(), columns);
        if !self.visible.as_ref().is_some_and(|visible| visible.key == key) {
            let (start, end) = self.range;
            let mut y: Option<(f64, f64)> = None;
            let mut lines = Vec::new();
            for (name, points) in &self.active().series {
                // 多取范围外的一个点，使曲线延伸到边界
                let lower = points
                    .partition_point(|point| point[0] < start)
                    .saturating_sub(1);
                let upper = (points.partition_point(|point| point[0] <= end) + 1).min(points.len());
                let reduced = downsample(&points[lower..upper], start, end, columns);
                for point in reduced.iter().filter(|point| point[0] >= start && point[0] <= end) {
                    y = Some(y.map_or((point[1], point[1]), |(low, high)| {
                        (low.min(point[1]), high.max(point[1]))
                    }));
                }
                lines.push((name.clone(), reduced));
            }
            self.visible = Some(Visible { key, lines, y });
        }
        self.visible.as_ref().unwrap()
    }

    fn summary(&mut self) -> &BTreeMap<String, Stats> {
        let key = self.cache_key();
        if !self.summary.as_ref().is_some_and(|(cached, _)| *cached == key) {
            let (start, end) = self.range;
            let mut rows = BTreeMap::new();
            for (name, points) in &self.active().series {
                let lower = points.partition_point(|point| point[0] < start);
                let upper = points.partition_point(|point| point[0] <= end);
                let slice = &points[lower..upper];
                if let Some(last) = slice.last() {
                    let (low, high) = slice
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), point| {
                            (low.min(point[1]), high.max(point[1]))
                        });
                    rows.insert(name.clone(), (last[1], low, high, slice.len()));
                }
            }
            self.summary = Some((key, rows));
        }
        &self.summary.as_ref().unwrap().1
    }

    fn plot_ui(&mut self, ui: &mut egui::Ui) {
        let (start, end) = self.range;
        let manual_y = match (self.y_min.parse::<f64>(), self.y_max.parse::<f64>()) {
            (Ok(min), Ok(max)) if min.is_finite() && max.is_finite() && min < max => {
                Some((min, max))
            }
            _ => None,
        };
        let styles: BTreeMap<String, (Color32, String)> = self
            .active()
            .series
            .keys()
            .map(|name| {
                let unit = self.unit(name);
                let label = if unit.is_empty() {
                    name.clone()
                } else {
                    format!("{name} ({unit})")
                };
                (name.clone(), (self.color(name), label))
            })
            .collect();
        let empty = self.active().series.is_empty();
        let hint = self.empty_hint();
        let visible = self.visible_lines();
        let (low, high) = manual_y.or(visible.y).unwrap_or((0.0, 1.0));
        let padding = if manual_y.is_some() {
            0.0
        } else if high > low {
            (high - low) * 0.06
        } else {
            1.0
        };
        let bounds =
            egui_plot::PlotBounds::from_min_max([start, low - padding], [end, high + padding]);
        // 横轴完全由时间范围控制，纵轴每帧按可见数据自动适配
        let response = egui_plot::Plot::new("data_plot")
            .legend(egui_plot::Legend::default())
            .show_grid([true, true])
            .allow_zoom([true, false])
            .allow_drag([true, false])
            .allow_scroll([true, false])
            .allow_boxed_zoom(false)
            .allow_double_click_reset(false)
            .x_grid_spacer(egui_plot::uniform_grid_spacer(time_steps))
            .x_axis_formatter(|mark, range| {
                format_clock(mark.value, range.end() - range.start() < 10.0)
            })
            .label_formatter(|name, point| {
                if name.is_empty() {
                    format_clock(point.x, true)
                } else {
                    format!(
                        "{name}\n{}\n{}",
                        format_clock(point.x, true),
                        trim_number(point.y)
                    )
                }
            })
            .show(ui, |plot| {
                plot.set_plot_bounds(bounds);
                for (name, points) in &visible.lines {
                    let (color, label) = styles[name].clone();
                    plot.line(egui_plot::Line::new(label, points.clone()).color(color));
                }
            });
        self.plot_width = response.response.rect.width();
        // 曲线上的拖动、缩放回写到时间范围，并关闭跟随
        let after = response.transform.bounds();
        let (new_start, new_end) = (after.min()[0], after.max()[0]);
        let tolerance = (end - start) * 1e-6;
        if (new_start - start).abs() > tolerance || (new_end - end).abs() > tolerance {
            self.follow = false;
            self.set_range(new_start, new_end);
        }
        if empty {
            ui.painter().text(
                response.response.rect.center(),
                egui::Align2::CENTER_CENTER,
                hint,
                egui::FontId::proportional(16.0),
                Color32::from_rgb(139, 150, 157),
            );
        }
    }

    fn empty_hint(&self) -> &'static str {
        if self.parser.rules.is_empty() {
            "请先在“规则配置”中添加解析规则"
        } else if self.importing.is_some() {
            "正在导入日志…"
        } else {
            "等待接收数据…"
        }
    }

    /// 全天时间轴：按分钟显示数据密度；拖动选区两端调整范围，拖动中间平移，空白处拖动重新框选
    fn timeline_ui(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 46.0),
            egui::Sense::click_and_drag(),
        );
        let bar = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.bottom() - 14.0));
        let to_x = |time: f64| bar.left() + (time / DAY) as f32 * bar.width();
        let to_time = |x: f32| (((x - bar.left()) / bar.width()) as f64 * DAY).clamp(0.0, DAY);
        let muted = Color32::from_rgb(0x66, 0x70, 0x78);
        let accent = Color32::from_rgb(0x00, 0x78, 0xd4);
        let painter = ui.painter_at(rect);
        painter.rect_filled(bar, 2.0, Color32::WHITE);
        let minutes = &self.active().minutes;
        if let Some(&peak) = minutes.iter().max().filter(|peak| **peak > 0) {
            let scale = (peak as f32 + 1.0).ln();
            let width = (bar.width() / 1440.0).max(1.0);
            for (minute, &count) in minutes.iter().enumerate().filter(|(_, count)| **count > 0) {
                let height = ((count as f32 + 1.0).ln() / scale).max(0.15) * (bar.height() - 4.0);
                let x = to_x(minute as f64 * 60.0);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(x, bar.bottom() - 2.0 - height),
                        egui::pos2(x + width, bar.bottom() - 2.0),
                    ),
                    0.0,
                    Color32::from_rgb(0x8f, 0xb8, 0xe8),
                );
            }
        }
        for hour in (0..=24).step_by(3) {
            let x = to_x(hour as f64 * 3600.0);
            painter.vline(x, bar.y_range(), egui::Stroke::new(1.0, Color32::from_rgb(0xe0, 0xe4, 0xe7)));
            let align = match hour {
                0 => egui::Align2::LEFT_TOP,
                24 => egui::Align2::RIGHT_TOP,
                _ => egui::Align2::CENTER_TOP,
            };
            painter.text(
                egui::pos2(x, bar.bottom() + 1.0),
                align,
                format!("{hour:02}:00"),
                egui::FontId::proportional(10.5),
                muted,
            );
        }
        let left = to_x(self.range.0);
        let right = to_x(self.range.1).max(left + 2.0);
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(left, bar.top()), egui::pos2(right, bar.bottom())),
            0.0,
            accent.gamma_multiply(0.18),
        );
        for x in [left, right] {
            painter.rect_filled(
                egui::Rect::from_center_size(egui::pos2(x, bar.center().y), egui::vec2(4.0, bar.height())),
                1.0,
                accent,
            );
        }
        painter.rect_stroke(
            bar,
            2.0,
            egui::Stroke::new(1.0, Color32::from_rgb(0xa9, 0xa9, 0xa9)),
            egui::StrokeKind::Inside,
        );

        if let Some(position) = response.hover_pos() {
            let left_dist = (position.x - left).abs();
            let right_dist = (position.x - right).abs();
            let near_left = left_dist <= 4.0 && left_dist < right_dist;
            let near_right = right_dist <= 4.0 && right_dist < left_dist;
            ui.ctx().set_cursor_icon(if near_left || near_right {
                egui::CursorIcon::ResizeHorizontal
            } else if position.x > left && position.x < right {
                egui::CursorIcon::Grab
            } else {
                egui::CursorIcon::PointingHand
            });
        }
        let pointer = response.interact_pointer_pos();
        if response.drag_started() {
            if let Some(position) = pointer {
                let time = to_time(position.x);
                let left_dist = (position.x - left).abs();
                let right_dist = (position.x - right).abs();
                // 边界拖动的判定：距离边界4像素以内，且距离该边界比另一边界更近
                self.timeline_drag = Some(if left_dist <= 4.0 && left_dist < right_dist {
                    TimelineDrag::Start
                } else if right_dist <= 4.0 && right_dist < left_dist {
                    TimelineDrag::End
                } else if position.x > left && position.x < right {
                    TimelineDrag::Move(time - self.range.0)
                } else {
                    TimelineDrag::Create(time)
                });
            }
        }
        if let (Some(drag), Some(position), true) = (self.timeline_drag, pointer, response.dragged()) {
            let time = to_time(position.x);
            self.follow = false;
            match drag {
                TimelineDrag::Start => self.set_range(time.min(self.range.1 - MIN_SPAN), self.range.1),
                TimelineDrag::End => self.set_range(self.range.0, time.max(self.range.0 + MIN_SPAN)),
                TimelineDrag::Move(offset) => {
                    let span = self.range.1 - self.range.0;
                    self.set_range(time - offset, time - offset + span);
                }
                TimelineDrag::Create(anchor) => {
                    self.set_range(anchor.min(time), anchor.max(time).max(anchor.min(time) + MIN_SPAN))
                }
            }
        }
        if response.drag_stopped() {
            self.timeline_drag = None;
        }
        // 单击：保持跨度，把选区移到点击位置
        if response.clicked() {
            if let Some(position) = pointer {
                let span = self.range.1 - self.range.0;
                let center = to_time(position.x);
                self.follow = false;
                self.set_range(center - span / 2.0, center + span / 2.0);
            }
        }
        response.on_hover_text(format!(
            "{} – {}\n拖动两端调整范围，拖动中间平移，在空白处拖动重新框选，单击移动到该时刻",
            format_clock(self.range.0, false),
            format_clock(self.range.1, false)
        ));
    }

    fn table_ui(&mut self, ui: &mut egui::Ui) {
        let mut names: Vec<String> = self.parser.fields.iter().map(|field| field.name.clone()).collect();
        let extra: Vec<String> = self
            .active()
            .series
            .keys()
            .filter(|name| !names.contains(name))
            .cloned()
            .collect();
        names.extend(extra);
        let rows: Vec<(String, String, bool, Color32)> = names
            .iter()
            .map(|name| {
                let hex = self.field(name).is_some_and(|field| field.hex);
                (name.clone(), self.unit(name).to_owned(), hex, self.color(name))
            })
            .collect();
        let hint = self.empty_hint();
        let stats = self.summary().clone();
        let line = Color32::from_rgb(0xe0, 0xe4, 0xe7);
        let muted = Color32::from_rgb(0x66, 0x70, 0x78);
        let text = Color32::from_gray(32);
        egui::ScrollArea::vertical()
            .id_salt("plot-table-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if rows.is_empty() {
                    ui.add_space(28.0);
                    ui.vertical_centered(|ui| ui.label(egui::RichText::new(hint).color(muted)));
                    return;
                }
                let width = ui.available_width();
                let compact = width < 320.0;
                // 列宽：色块 | 字段 | 最新值 | 最小 | 最大（窄时隐藏最小/最大）
                let column = if compact { 0.0 } else { (width * 0.2).clamp(60.0, 120.0) };
                let name_width = (width * 0.32).clamp(70.0, 220.0);
                let value_right = 18.0 + name_width + (width - 18.0 - name_width - column * 2.0).max(60.0) - 8.0;
                let row = |ui: &mut egui::Ui, height: f32, fill: Option<Color32>| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                    if let Some(fill) = fill {
                        ui.painter().rect_filled(rect, 0.0, fill);
                    }
                    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, egui::Stroke::new(1.0, line));
                    rect
                };
                let at = |rect: egui::Rect, x: f32| egui::pos2(rect.left() + x, rect.center().y);
                let header = row(ui, 27.0, Some(Color32::from_rgb(0xf0, 0xf0, 0xf0)));
                let font = egui::FontId::proportional(12.0);
                let painter = ui.painter();
                painter.text(at(header, 18.0), egui::Align2::LEFT_CENTER, "字段", font.clone(), text);
                painter.text(at(header, value_right), egui::Align2::RIGHT_CENTER, "最新值", font.clone(), text);
                if !compact {
                    painter.text(at(header, value_right + column), egui::Align2::RIGHT_CENTER, "最小", font.clone(), text);
                    painter.text(at(header, value_right + column * 2.0), egui::Align2::RIGHT_CENTER, "最大", font, text);
                }
                for (name, unit, hex, color) in rows {
                    let rect = row(ui, 30.0, None);
                    let painter = ui.painter();
                    painter.rect_filled(
                        egui::Rect::from_center_size(at(rect, 9.0), egui::vec2(8.0, 8.0)),
                        2.0,
                        color,
                    );
                    let galley = painter.layout_no_wrap(name.clone(), egui::FontId::proportional(13.0), text);
                    let clip = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 18.0, rect.top()),
                        egui::vec2(name_width - 6.0, rect.height()),
                    );
                    painter
                        .with_clip_rect(clip)
                        .galley(egui::pos2(clip.left(), rect.center().y - galley.size().y / 2.0), galley, text);
                    match stats.get(&name) {
                        Some(&(last, low, high, count)) => {
                            painter.text(
                                at(rect, value_right),
                                egui::Align2::RIGHT_CENTER,
                                with_unit(&display_value(last, hex), &unit),
                                egui::FontId::monospace(14.0),
                                text,
                            );
                            if !compact {
                                for (offset, value) in [(1.0, low), (2.0, high)] {
                                    painter.text(
                                        at(rect, value_right + column * offset),
                                        egui::Align2::RIGHT_CENTER,
                                        display_value(value, hex),
                                        egui::FontId::monospace(12.0),
                                        muted,
                                    );
                                }
                            }
                            ui.interact(rect, ui.id().with(("plot-row", &name)), egui::Sense::hover())
                                .on_hover_text(format!("所选时间范围内 {count} 个数据点"));
                        }
                        None => {
                            painter.text(
                                at(rect, value_right),
                                egui::Align2::RIGHT_CENTER,
                                "—",
                                egui::FontId::monospace(14.0),
                                muted,
                            );
                        }
                    }
                }
            });
    }
}

fn seconds_of_day(at: NaiveDateTime) -> f64 {
    let time = at.time();
    time.num_seconds_from_midnight() as f64 + time.nanosecond().min(999_999_999) as f64 / 1e9
}

/// `HH:MM:SS`；`millis` 时追加毫秒。24:00:00 表示当天结束
fn format_clock(seconds: f64, millis: bool) -> String {
    let total = (seconds.clamp(0.0, DAY) * 1000.0).round() as u64;
    let (hours, minutes, secs, ms) = (
        total / 3_600_000,
        total / 60_000 % 60,
        total / 1000 % 60,
        total % 1000,
    );
    if millis {
        format!("{hours:02}:{minutes:02}:{secs:02}.{ms:03}")
    } else {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    }
}

/// 解析 `HH:MM`、`HH:MM:SS` 或 `HH:MM:SS.mmm`（全角冒号同样可用）
fn parse_clock(text: &str) -> Option<f64> {
    let normalized = text.trim().replace('：', ":");
    let parts: Vec<&str> = normalized.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let hours: u32 = parts[0].parse().ok()?;
    let minutes: u32 = parts[1].parse().ok()?;
    let seconds: f64 = match parts.get(2) {
        Some(value) => value.parse().ok()?,
        None => 0.0,
    };
    if minutes >= 60 || !(0.0..60.0).contains(&seconds) {
        return None;
    }
    let total = hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds;
    (total <= DAY).then_some(total)
}

/// 时间轴刻度：在 0.1 秒到 3 小时之间选取整齐的步长
fn time_steps(input: egui_plot::GridInput) -> [f64; 3] {
    const STEPS: [f64; 16] = [
        0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0,
        3600.0, 10800.0,
    ];
    // 三级刻度依次拉开约 4 倍与 16 倍，最大一级才显示时间标签，避免文字重叠
    let pick = |factor: f64| {
        STEPS
            .iter()
            .copied()
            .find(|step| *step >= input.base_step_size * factor)
            .unwrap_or(STEPS[STEPS.len() - 1])
    };
    [pick(1.0), pick(4.0), pick(16.0)]
}

/// 每个像素列保留首、尾、最小、最大点，按时间顺序输出
fn downsample(points: &[[f64; 2]], start: f64, end: f64, columns: usize) -> Vec<[f64; 2]> {
    if points.len() <= columns * 4 {
        return points.to_vec();
    }
    let width = (end - start).max(f64::EPSILON) / columns as f64;
    let bucket = |time: f64| ((time - start) / width).floor();
    let mut output = Vec::with_capacity(columns * 4);
    let mut index = 0;
    while index < points.len() {
        let current = bucket(points[index][0]);
        let (first, mut low, mut high) = (index, index, index);
        while index < points.len() && bucket(points[index][0]) == current {
            if points[index][1] < points[low][1] {
                low = index;
            }
            if points[index][1] > points[high][1] {
                high = index;
            }
            index += 1;
        }
        let mut picks = [first, low, high, index - 1];
        picks.sort_unstable();
        let mut previous = usize::MAX;
        for pick in picks {
            if pick != previous {
                output.push(points[pick]);
                previous = pick;
            }
        }
    }
    output
}

/// 读取 Z_COM 日志，按 `← HEX:` 行还原接收字节并以该包的时间戳解析
fn parse_log_file(path: &Path, parser: &mut Parser) -> Result<History, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("读取日志失败：{error}"))?;
    let mut data = Dataset::default();
    parse_log(&String::from_utf8_lossy(&bytes), parser, &mut data);
    if data.date.is_none() {
        return Err("文件中没有 Z_COM 日志格式的接收数据".into());
    }
    Ok(History {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path.to_path_buf(),
        data,
    })
}

fn parse_log(text: &str, parser: &mut Parser, data: &mut Dataset) {
    let mut current: Option<NaiveDateTime> = None;
    for line in text.lines() {
        if let Some(at) = line
            .strip_prefix('[')
            .and_then(|rest| rest.get(..23))
            .and_then(|stamp| NaiveDateTime::parse_from_str(stamp, "%Y-%m-%d %H:%M:%S%.3f").ok())
        {
            current = Some(at);
            continue;
        }
        let (Some(hex), Some(at)) = (line.trim_start().strip_prefix("← HEX:"), current) else {
            continue;
        };
        // 只绘制一天：以文件中第一条接收数据的日期为准
        match data.date {
            None => data.date = Some(at.date()),
            Some(date) if date != at.date() => continue,
            Some(_) => {}
        }
        let bytes: Vec<u8> = hex
            .split_whitespace()
            .filter_map(|byte| u8::from_str_radix(byte, 16).ok())
            .collect();
        data.ingest(parser, &bytes, seconds_of_day(at));
    }
}

/// 曲线与表格共用的配色，按字段在规则中的顺序分配
fn series_color(index: usize) -> Color32 {
    const PALETTE: [Color32; 10] = [
        Color32::from_rgb(0x1f, 0x6f, 0xeb),
        Color32::from_rgb(0xd9, 0x48, 0x2b),
        Color32::from_rgb(0x18, 0x79, 0x4e),
        Color32::from_rgb(0xb1, 0x5c, 0x00),
        Color32::from_rgb(0x6f, 0x42, 0xc1),
        Color32::from_rgb(0x00, 0x8b, 0x8b),
        Color32::from_rgb(0xc2, 0x18, 0x5b),
        Color32::from_rgb(0x5f, 0x6b, 0x73),
        Color32::from_rgb(0x8a, 0x9a, 0x00),
        Color32::from_rgb(0x35, 0x69, 0xb8),
    ];
    PALETTE[index % PALETTE.len()]
}

fn trim_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let text = format!("{value:.6}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// 十六进制字段显示原进制并附带十进制
fn display_value(value: f64, hex: bool) -> String {
    if hex {
        format!("0x{:X} ({})", value as i64, value as i64)
    } else {
        trim_number(value)
    }
}

fn with_unit(value: &str, unit: &str) -> String {
    if unit.is_empty() {
        value.to_owned()
    } else {
        format!("{value} {unit}")
    }
}

fn literal(value: &str) -> String {
    let value = value
        .replace("\\r\\n", "\n")
        .replace("\\r", "\n")
        .replace("\\n", "\n")
        .replace("␍␊", "\n")
        .replace(['␍', '␊'], "\n");
    let mut output = String::new();
    let mut space = false;
    for character in value.chars() {
        if character == '\n' {
            output.push_str("\\r?\\n");
            space = false;
        } else if character.is_whitespace() {
            if !space {
                output.push_str("\\s*");
            }
            space = true;
        } else {
            output.push_str(&regex::escape(&character.to_string()));
            space = false;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S%.3f").unwrap()
    }

    fn plot(template: &str) -> NativePlot {
        let mut plot = NativePlot {
            template: template.into(),
            ..Default::default()
        };
        plot.compile(None).unwrap();
        plot
    }

    #[test]
    fn waits_for_complete_split_packets() {
        let mut plot = plot(r"temp={温度},hex={寄存器:hex0x}\r\n");
        plot.consume(b"temp=12", at("2026-10-13 10:00:00.000"));
        assert!(plot.live.series.is_empty());
        plot.consume(b"3,hex=0xFF\r\n", at("2026-10-13 10:00:01.500"));
        assert_eq!(plot.live.series["温度"][0], [36001.5, 123.0]);
        assert_eq!(plot.live.series["寄存器"][0][1], 255.0);
    }

    #[test]
    fn live_data_restarts_on_new_day() {
        let mut plot = plot("v={v}\\n");
        plot.consume(b"v=1\n", at("2026-10-13 23:59:59.000"));
        plot.consume(b"v=2\n", at("2026-10-14 00:00:01.000"));
        assert_eq!(plot.live.series["v"], vec![[1.0, 2.0]]);
        assert_eq!(plot.live.date, NaiveDate::from_ymd_opt(2026, 10, 14));
    }

    #[test]
    fn rejects_invalid_rules_without_replacing_previous() {
        let mut plot = plot("x={value}");
        plot.template = "x={ :int}".into();
        assert!(plot.compile(None).is_err());
        plot.template = "x={a:bogus:V}".into();
        assert!(plot.compile(None).is_err());
        assert_eq!(plot.parser.rules.len(), 1);
    }

    #[test]
    fn parses_units_and_summarizes_selected_range() {
        let mut plot = plot("t={温度:℃},v={电压:int:mV},r={寄存器:hex0x}\\r\\nt2={温度}\\r\\n");
        let units: Vec<_> = plot
            .parser
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.unit.as_str()))
            .collect();
        assert_eq!(units, [("温度", "℃"), ("电压", "mV"), ("寄存器", "")]);
        plot.consume(b"t=25.5,v=3300,r=0x1F\r\n", at("2026-10-13 08:00:00.000"));
        plot.consume(b"t2=20.25\r\n", at("2026-10-13 09:00:00.000"));
        plot.consume(b"t2=30\r\n", at("2026-10-13 10:00:00.000"));
        plot.range = (8.0 * 3600.0, 9.5 * 3600.0);
        let rows = plot.summary().clone();
        assert_eq!(rows["温度"], (20.25, 20.25, 25.5, 2));
        assert_eq!(
            with_unit(&display_value(rows["电压"].0, false), plot.unit("电压")),
            "3300 mV"
        );
        assert_eq!(display_value(rows["寄存器"].0, true), "0x1F (31)");
    }

    #[test]
    fn imports_log_using_packet_timestamps() {
        let log = "[2026-10-12 16:05:02.100] 串口 COM5 已连接\n\
[2026-10-12 16:05:03.250]\n ← HEX: 61 72 6D 5F 76 6F 6C 74 61 67 65 3D 34 38 33\n ← ASCII: arm_voltage=483\n\
[2026-10-12 16:05:03.300]\n ← HEX: 32 31 28 6D 56 29 0D 0A\n ← ASCII: 21(mV)␍␊\n\
[2026-10-12 16:05:04.000]\n → HEX: 61 72 6D 5F 76 6F 6C 74 61 67 65 3D 31 0D 0A\n → ASCII: arm_voltage=1␍␊\n";
        let mut parser = Parser::compile("arm_voltage={臂电压:int:mV}(mV)").unwrap();
        let mut data = Dataset::default();
        parse_log(log, &mut parser, &mut data);
        assert_eq!(data.date, NaiveDate::from_ymd_opt(2026, 10, 12));
        // 跨包的报文在后一包的时刻完成解析；发送方向不参与
        assert_eq!(data.series["臂电压"], vec![[57903.3, 48321.0]]);
        assert_eq!(data.minutes[16 * 60 + 5], 1);
    }

    #[test]
    fn clock_round_trip_and_downsample_keeps_peaks() {
        assert_eq!(parse_clock("08:30"), Some(30600.0));
        assert_eq!(parse_clock("23：59：59.5"), Some(86399.5));
        assert_eq!(parse_clock("24:00:00"), Some(DAY));
        assert_eq!(parse_clock("12:60"), None);
        assert_eq!(format_clock(30600.25, true), "08:30:00.250");
        let points: Vec<[f64; 2]> = (0..10_000)
            .map(|index| [index as f64, if index == 5_000 { 99.0 } else { 0.0 }])
            .collect();
        let reduced = downsample(&points, 0.0, 10_000.0, 100);
        assert!(reduced.len() <= 400);
        assert!(reduced.iter().any(|point| point[1] == 99.0));
    }
}
