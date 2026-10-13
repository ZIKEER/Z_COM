use eframe::egui;
use regex::Regex;
use std::collections::{BTreeMap, VecDeque};

#[derive(Default)]
pub(crate) struct NativePlot {
    pub open: bool,
    pub template: String,
    rules: Vec<(Regex, Vec<(String, bool)>)>,
    buffer: String,
    series: BTreeMap<String, VecDeque<[f64; 2]>>,
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
    pub fn preview(&mut self, rules: bool) {
        self.open = true;
        self.rule_dialog = rules;
        self.rule_draft = self.template.clone();
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
                for (index, (name, hex)) in fields.iter().enumerate() {
                    let raw = &captures[index + 1];
                    let value = if *hex {
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
                        let series = self.series.entry(name.clone()).or_default();
                        series.push_back([self.sequence as f64, value]);
                        while series.len() > self.max_points.max(20) {
                            series.pop_front();
                        }
                    }
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
        let placeholder = Regex::new(r"\{([^{}:]+)(?::(number|int|hex|hex0x))?\}").unwrap();
        let mut rules = Vec::new();
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
                let kind = capture
                    .get(2)
                    .map(|value| value.as_str())
                    .unwrap_or("number");
                let name = capture[1].trim();
                if name.is_empty() {
                    return Err("曲线名不能为空".into());
                }
                fields.push((name.to_owned(), kind.starts_with("hex")));
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
        self.buffer.clear();
        if self.max_points == 0 {
            self.max_points = 500;
        }
        Ok(())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("规则配置").clicked() {
                self.rule_draft = self.template.clone();
                self.rule_dialog = true;
            }
            if ui
                .button(if self.paused { "继续" } else { "暂停" })
                .clicked()
            {
                self.paused = !self.paused;
            }
            ui.label("点数");
            ui.add(egui::DragValue::new(&mut self.max_points).range(20..=10000));
            if ui.button("清除").clicked() {
                self.series.clear();
                self.buffer.clear();
                self.sequence = 0;
                self.reset_view = true;
            }
            if ui.button("回到最新").clicked() {
                self.reset_view = true;
            }
            if ui.button("重置视图").clicked() {
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
        });
        if self.rule_dialog {
            let response = egui::Modal::new(egui::Id::new("plot-rules")).frame(super::egui_app::dialog_frame(true).fill(egui::Color32::WHITE).corner_radius(6.0)).show(ui.ctx(), |ui| {
                ui.set_width(700.0_f32.min(ui.ctx().screen_rect().width()-48.0));
                super::egui_app::dialog_header(ui,"规则配置",&mut self.rule_dialog);
                ui.label("每行一条规则，用 {字段名} 提取数值；支持 :int、:hex、:hex0x 和转义字符。示例：temp={温度},voltage={电压}\\r\\n");
                ui.add(egui::TextEdit::multiline(&mut self.rule_draft).desired_width(f32::INFINITY).desired_rows(12).min_size(egui::vec2(0.0,220.0)).font(egui::TextStyle::Monospace));
                if !self.error.is_empty() { ui.colored_label(egui::Color32::DARK_RED,&self.error); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("保存规则").color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(22,119,255))).clicked() {
                        let previous = self.template.clone();
                        self.template = self.rule_draft.clone();
                        match self.compile() {
                            Ok(()) => { self.error.clear(); self.rule_dialog=false; }
                            Err(error) => { self.template=previous; self.error=error; }
                        }
                    }
                    if ui.button("取消").clicked() { self.rule_dialog=false; }
                });
            });
            if response.should_close() {
                self.rule_dialog = false;
            }
        }
        let mut plot = egui_plot::Plot::new("data_plot")
            .legend(egui_plot::Legend::default())
            .show_grid([false, true])
            .height((ui.available_height() - 26.0).max(100.0));
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
                plot.line(egui_plot::Line::new(
                    name,
                    points.iter().copied().collect::<Vec<_>>(),
                ));
            }
        });
        if self.series.is_empty() {
            ui.painter().text(
                response.response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "等待接收数据…",
                egui::FontId::proportional(16.0),
                egui::Color32::from_rgb(139, 150, 157),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(if self.paused {
                "已暂停"
            } else {
                "实时跟随"
            });
        });
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
}
