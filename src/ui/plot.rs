use eframe::egui::{self, Color32};
use regex::Regex;
use std::collections::{BTreeMap, VecDeque};

/// 规则中的一个字段：`{名称}`、`{名称:类型}`、`{名称:单位}` 或 `{名称:类型:单位}`
#[derive(Clone, Debug, PartialEq)]
struct Field {
    name: String,
    hex: bool,
    unit: String,
}

/// 字段的最新值与统计，供表格显示
#[derive(Clone, Debug, Default)]
struct Latest {
    value: f64,
    raw: String,
    min: f64,
    max: f64,
    count: u64,
}

/// 曲线窗口的显示方式，持久化在配置中
pub(crate) const VIEWS: &[(&str, &str)] = &[
    ("plot", "曲线"),
    ("table", "表格"),
    ("both", "曲线+表格"),
];

#[derive(Default)]
pub(crate) struct NativePlot {
    pub open: bool,
    pub template: String,
    pub view: String,
    rules: Vec<(Regex, Vec<Field>)>,
    /// 按规则中首次出现的顺序排列的字段，曲线配色与表格行序都以此为准
    fields: Vec<Field>,
    buffer: String,
    series: BTreeMap<String, VecDeque<[f64; 2]>>,
    latest: BTreeMap<String, Latest>,
    sequence: u64,
    paused: bool,
    error: String,
    max_points: usize,
    y_min: String,
    y_max: String,
    reset_view: bool,
    rule_dialog: bool,
    rule_draft: String,
}

impl NativePlot {
    /// 调试预览：打开窗口并按当前规则填充示例数据，便于截图检查布局
    pub fn preview(&mut self, rules: bool) {
        self.open = true;
        self.rule_dialog = rules;
        self.rule_draft = self.template.clone();
        if self.rules.is_empty() {
            self.template = "t={温度:℃},v={电压:int:mV},i={电流:mA},r={寄存器:hex0x}\r\n".into();
            let _ = self.compile();
        }
        for step in 0..120 {
            let phase = step as f64 / 12.0;
            let line = format!(
                "t={:.2},v={},i={:.1},r=0x{:02X}
",
                25.0 + phase.sin() * 3.0,
                3300 + (phase.cos() * 40.0) as i64,
                120.0 + (phase * 0.7).sin() * 15.0,
                step % 256
            );
            self.consume(line.as_bytes());
        }
    }

    pub fn consume(&mut self, bytes: &[u8]) {
        if self.paused || !self.open {
            return;
        }
        self.buffer.push_str(&String::from_utf8_lossy(bytes));
        if self.rules.is_empty() {
            self.buffer.clear();
            return;
        }
        let mut consumed = 0;
        for (matcher, fields) in &self.rules {
            for captures in matcher.captures_iter(&self.buffer) {
                if captures.get(0).unwrap().end() == self.buffer.len()
                    && !self.buffer.ends_with(['\r', '\n'])
                {
                    continue;
                }
                self.sequence += 1;
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
                    let Some(value) = value.filter(|value| value.is_finite()) else {
                        continue;
                    };
                    let series = self.series.entry(field.name.clone()).or_default();
                    series.push_back([self.sequence as f64, value]);
                    while series.len() > self.max_points.max(20) {
                        series.pop_front();
                    }
                    let latest = self.latest.entry(field.name.clone()).or_default();
                    if latest.count == 0 {
                        latest.min = value;
                        latest.max = value;
                    }
                    latest.value = value;
                    latest.raw = raw.to_owned();
                    latest.min = latest.min.min(value);
                    latest.max = latest.max.max(value);
                    latest.count += 1;
                }
                consumed = consumed.max(captures.get(0).unwrap().end());
            }
        }
        self.buffer.drain(..consumed);
        if self.buffer.len() > 65536 {
            self.buffer.clear();
        }
    }

    pub fn compile(&mut self) -> Result<(), String> {
        let placeholder = Regex::new(r"\{([^{}:]+)(?::([^{}:]*))?(?::([^{}]*))?\}").unwrap();
        let mut rules = Vec::new();
        let mut ordered: Vec<Field> = Vec::new();
        for template in self
            .template
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
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
        self.rules = rules;
        self.fields = ordered;
        self.buffer.clear();
        if self.max_points == 0 {
            self.max_points = 500;
        }
        Ok(())
    }

    fn clear(&mut self) {
        self.series.clear();
        self.latest.clear();
        self.buffer.clear();
        self.sequence = 0;
        self.reset_view = true;
    }

    fn unit(&self, name: &str) -> &str {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map_or("", |field| field.unit.as_str())
    }

    fn color(&self, name: &str) -> Color32 {
        let index = self
            .fields
            .iter()
            .position(|field| field.name == name)
            .unwrap_or(self.fields.len() + self.series.keys().position(|key| key == name).unwrap_or(0));
        series_color(index)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if !VIEWS.iter().any(|(key, _)| *key == self.view) {
            self.view = "plot".into();
        }
        let show_plot = self.view != "table";
        let show_table = self.view != "plot";
        ui.horizontal_wrapped(|ui| {
            super::app::mode_switch(ui, &mut self.view, VIEWS, true);
            ui.separator();
            if ui.button("规则配置").clicked() {
                self.rule_draft = self.template.clone();
                self.error.clear();
                self.rule_dialog = true;
            }
            if ui
                .button(if self.paused { "继续" } else { "暂停" })
                .clicked()
            {
                self.paused = !self.paused;
            }
            if ui.button("清除").on_hover_text("清除曲线与表格统计").clicked() {
                self.clear();
            }
            if show_plot {
                ui.separator();
                ui.label("点数");
                ui.add(egui::DragValue::new(&mut self.max_points).range(20..=10000));
                if ui.button("回到最新").clicked() {
                    self.reset_view = true;
                }
                ui.label("Y 最小");
                ui.add(
                    egui::TextEdit::singleline(&mut self.y_min)
                        .desired_width(70.0)
                        .hint_text("自动"),
                );
                ui.label("Y 最大");
                ui.add(
                    egui::TextEdit::singleline(&mut self.y_max)
                        .desired_width(70.0)
                        .hint_text("自动"),
                );
            }
        });
        self.rule_dialog_ui(ui.ctx());
        let status_height = 22.0;
        if show_plot && show_table {
            egui::SidePanel::right("plot-table")
                .resizable(true)
                .default_width(300.0)
                .width_range(200.0..=(ui.available_width() * 0.7).max(200.0))
                .show_inside(ui, |ui| self.table_ui(ui, status_height));
        }
        if show_plot {
            self.plot_ui(ui, status_height);
        } else {
            self.table_ui(ui, status_height);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(if self.paused {
                "已暂停"
            } else {
                "实时跟随"
            });
            if !self.rules.is_empty() {
                ui.weak(format!("{} 个字段", self.fields.len()));
            }
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
                        let previous = self.template.clone();
                        self.template = self.rule_draft.clone();
                        match self.compile() {
                            Ok(()) => {
                                self.error.clear();
                                self.rule_dialog = false;
                            }
                            Err(error) => {
                                // compile 失败时不会替换已生效的规则
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

    fn plot_ui(&mut self, ui: &mut egui::Ui, status_height: f32) {
        let mut plot = egui_plot::Plot::new("data_plot")
            .legend(egui_plot::Legend::default())
            .show_grid([false, true])
            .height((ui.available_height() - status_height).max(100.0));
        if self.series.is_empty() {
            plot = plot.include_y(0.0).include_y(1.0);
        }
        if self.reset_view {
            plot = plot.reset();
            self.reset_view = false;
        }
        if let (Ok(min), Ok(max)) = (self.y_min.parse::<f64>(), self.y_max.parse::<f64>()) {
            if min.is_finite() && max.is_finite() && min < max {
                plot = plot.default_y_bounds(min, max);
            }
        }
        let response = plot.show(ui, |plot| {
            for (name, points) in &self.series {
                let unit = self.unit(name);
                let label = if unit.is_empty() {
                    name.clone()
                } else {
                    format!("{name} ({unit})")
                };
                plot.line(
                    egui_plot::Line::new(label, points.iter().copied().collect::<Vec<_>>())
                        .color(self.color(name)),
                );
            }
        });
        if self.series.is_empty() {
            ui.painter().text(
                response.response.rect.center(),
                egui::Align2::CENTER_CENTER,
                self.empty_hint(),
                egui::FontId::proportional(16.0),
                Color32::from_rgb(139, 150, 157),
            );
        }
    }

    fn empty_hint(&self) -> &'static str {
        if self.rules.is_empty() {
            "请先在“规则配置”中添加解析规则"
        } else {
            "等待接收数据…"
        }
    }

    fn table_ui(&mut self, ui: &mut egui::Ui, status_height: f32) {
        let height = (ui.available_height() - status_height).max(60.0);
        let header = Color32::from_rgb(0xf0, 0xf0, 0xf0);
        let line = Color32::from_rgb(0xe0, 0xe4, 0xe7);
        let muted = Color32::from_rgb(0x66, 0x70, 0x78);
        egui::ScrollArea::vertical()
            .id_salt("plot-table-scroll")
            .max_height(height)
            .min_scrolled_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // 规则中声明的字段按顺序显示，尚未收到数据的显示占位；
                // 规则修改前遗留的数据排在最后
                let mut names: Vec<String> = self.fields.iter().map(|field| field.name.clone()).collect();
                names.extend(self.latest.keys().filter(|name| !names.contains(name)).cloned().collect::<Vec<_>>());
                if names.is_empty() {
                    ui.add_space(28.0);
                    ui.vertical_centered(|ui| ui.label(egui::RichText::new(self.empty_hint()).color(muted)));
                    return;
                }
                let width = ui.available_width();
                let compact = width < 320.0;
                // 列宽：色块 | 字段 | 当前值 | 最小 | 最大（窄时隐藏最小/最大）
                let stats = if compact { 0.0 } else { (width * 0.2).clamp(60.0, 120.0) };
                let name_width = (width * 0.32).clamp(70.0, 220.0);
                let value_width = (width - 18.0 - name_width - stats * 2.0).max(60.0);
                let row = |ui: &mut egui::Ui, height: f32, fill: Option<Color32>| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                    if let Some(fill) = fill {
                        ui.painter().rect_filled(rect, 0.0, fill);
                    }
                    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, egui::Stroke::new(1.0, line));
                    rect
                };
                let left = |rect: egui::Rect, offset: f32| egui::pos2(rect.left() + offset, rect.center().y);
                let header_rect = row(ui, 27.0, Some(header));
                let header_font = egui::FontId::proportional(12.0);
                let painter = ui.painter();
                painter.text(left(header_rect, 18.0), egui::Align2::LEFT_CENTER, "字段", header_font.clone(), Color32::from_gray(32));
                painter.text(egui::pos2(header_rect.left() + 18.0 + name_width + value_width - 8.0, header_rect.center().y), egui::Align2::RIGHT_CENTER, "当前值", header_font.clone(), Color32::from_gray(32));
                if !compact {
                    let x = header_rect.left() + 18.0 + name_width + value_width;
                    painter.text(egui::pos2(x + stats - 8.0, header_rect.center().y), egui::Align2::RIGHT_CENTER, "最小", header_font.clone(), Color32::from_gray(32));
                    painter.text(egui::pos2(x + stats * 2.0 - 8.0, header_rect.center().y), egui::Align2::RIGHT_CENTER, "最大", header_font, Color32::from_gray(32));
                }
                for name in names {
                    let unit = self.unit(&name).to_owned();
                    let color = self.color(&name);
                    let latest = self.latest.get(&name);
                    let rect = row(ui, 30.0, None);
                    let painter = ui.painter();
                    painter.rect_filled(
                        egui::Rect::from_center_size(left(rect, 9.0), egui::vec2(8.0, 8.0)),
                        2.0,
                        color,
                    );
                    let name_galley = painter.layout_no_wrap(name.clone(), egui::FontId::proportional(13.0), Color32::from_gray(32));
                    let clip = egui::Rect::from_min_size(egui::pos2(rect.left() + 18.0, rect.top()), egui::vec2(name_width - 6.0, rect.height()));
                    painter.with_clip_rect(clip).galley(egui::pos2(clip.left(), rect.center().y - name_galley.size().y / 2.0), name_galley, Color32::from_gray(32));
                    let value_right = rect.left() + 18.0 + name_width + value_width - 8.0;
                    let value_font = egui::FontId::monospace(14.0);
                    match latest {
                        Some(latest) => {
                            let text = with_unit(&format_value(latest.value, &latest.raw), &unit);
                            painter.text(egui::pos2(value_right, rect.center().y), egui::Align2::RIGHT_CENTER, text, value_font, Color32::from_gray(32));
                            if !compact {
                                let x = rect.left() + 18.0 + name_width + value_width;
                                for (column, value) in [(1.0, latest.min), (2.0, latest.max)] {
                                    painter.text(
                                        egui::pos2(x + stats * column - 8.0, rect.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        format_value(value, ""),
                                        egui::FontId::monospace(12.0),
                                        muted,
                                    );
                                }
                            }
                        }
                        None => {
                            painter.text(egui::pos2(value_right, rect.center().y), egui::Align2::RIGHT_CENTER, "—", value_font, muted);
                        }
                    }
                    if let Some(latest) = latest {
                        ui.interact(rect, ui.id().with(("plot-row", &name)), egui::Sense::hover())
                            .on_hover_text(format!("原始文本：{}\n更新次数：{}", latest.raw, latest.count));
                    }
                }
            });
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

/// 十进制原文直接显示；十六进制原文附带换算后的十进制
fn format_value(value: f64, raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if !raw.is_empty() && raw.parse::<f64>().is_ok() {
        return raw.to_owned();
    }
    if lower.starts_with("0x") || (!raw.is_empty() && raw.chars().any(|c| c.is_ascii_alphabetic())) {
        return format!("{raw} ({})", value as i64);
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let text = format!("{value:.6}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
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
    #[test]
    fn waits_for_complete_split_packets_and_bounds_history() {
        let mut plot = NativePlot {
            open: true,
            template: r"temp={温度},hex={寄存器:hex0x}\r\n".into(),
            max_points: 20,
            ..Default::default()
        };
        plot.compile().unwrap();
        plot.consume(b"temp=12");
        assert!(plot.series.is_empty());
        plot.consume(b"3,hex=0xFF\r\n");
        assert_eq!(plot.series["温度"][0][1], 123.0);
        assert_eq!(plot.series["寄存器"][0][1], 255.0);
        for _ in 0..40 {
            plot.consume(b"temp=2,hex=0x01\r\n");
        }
        assert_eq!(plot.series["温度"].len(), 20);
    }

    #[test]
    fn rejects_empty_field_names_without_replacing_previous_rules() {
        let mut plot = NativePlot {
            template: "x={value}".into(),
            ..Default::default()
        };
        plot.compile().unwrap();
        plot.template = "x={ :int}".into();
        assert!(plot.compile().is_err());
        assert_eq!(plot.rules.len(), 1);
    }

    #[test]
    fn parses_real_device_reply_into_table() {
        let mut plot = NativePlot {
            open: true,
            template: "arm_voltage={臂电压:int:mV}(mV)
ep1_voltage={EP1电压:int:mV}(mV)
heater_current={加热电流:int:mA}(mA)".into(),
            ..Default::default()
        };
        plot.compile().unwrap();
        plot.consume(b"$PA ex_current=0(0.1mA) cool_current=0(0.1mA) case_current=0(0.1mA)
");
        plot.consume(b"heater_current=0(mA) arm_current=0(mA) arm_voltage=48321(mV) hw_version=635(mV)
");
        plot.consume(b"ep1_voltage=8674(mV) ep2_voltage=8580(mV)
");
        let shown = |name: &str| {
            let latest = &plot.latest[name];
            with_unit(&format_value(latest.value, &latest.raw), plot.unit(name))
        };
        assert_eq!(shown("臂电压"), "48321 mV");
        assert_eq!(shown("EP1电压"), "8674 mV");
        assert_eq!(shown("加热电流"), "0 mA");
    }

    #[test]
    fn parses_units_and_tracks_latest_values() {
        let mut plot = NativePlot {
            open: true,
            template: "t={温度:℃},v={电压:int:mV},r={寄存器:hex0x}\\r\\nt2={温度}\\r\\n".into(),
            ..Default::default()
        };
        plot.compile().unwrap();
        let units: Vec<_> = plot.fields.iter().map(|field| (field.name.as_str(), field.unit.as_str())).collect();
        assert_eq!(units, [("温度", "℃"), ("电压", "mV"), ("寄存器", "")]);
        plot.consume(b"t=25.5,v=3300,r=0x1F\r\n");
        plot.consume(b"t2=20.25\r\n");
        let temperature = &plot.latest["温度"];
        assert_eq!((temperature.raw.as_str(), temperature.min, temperature.max, temperature.count), ("20.25", 20.25, 25.5, 2));
        assert_eq!(with_unit(&format_value(plot.latest["电压"].value, &plot.latest["电压"].raw), plot.unit("电压")), "3300 mV");
        assert_eq!(format_value(plot.latest["寄存器"].value, &plot.latest["寄存器"].raw), "0x1F (31)");
        plot.template = "x={a:bogus:V}".into();
        assert!(plot.compile().is_err());
    }
}
