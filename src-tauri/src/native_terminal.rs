use eframe::egui::{text::LayoutJob, Color32, FontId, Stroke, TextFormat};

/// 接收区片段样式，颜色与原 WebView 版本的 CSS 保持一致。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Timestamp,
    Arrow,
    HexLabel,
    AsciiLabel,
    /// 报文内容；仅此类片段解析 ANSI 转义序列
    Data,
    EventMarker,
    Connected,
    Disconnected,
    Error,
    Warning,
    Peer,
    Info,
}

impl Style {
    fn color(self) -> Color32 {
        match self {
            Self::Timestamp => Color32::from_rgb(0x00, 0x8b, 0x8b),
            Self::Arrow => Color32::from_rgb(0x1f, 0x6f, 0xeb),
            Self::HexLabel => Color32::from_rgb(0xb1, 0x5c, 0x00),
            Self::AsciiLabel | Self::Connected => Color32::from_rgb(0x18, 0x79, 0x4e),
            Self::Data => Color32::from_rgb(0x20, 0x20, 0x20),
            Self::EventMarker => Color32::from_rgb(0x6b, 0x72, 0x80),
            Self::Disconnected | Self::Info => Color32::from_rgb(0x5f, 0x6b, 0x73),
            Self::Error => Color32::from_rgb(0xb4, 0x23, 0x18),
            Self::Warning => Color32::from_rgb(0xa1, 0x5c, 0x00),
            Self::Peer => Color32::from_rgb(0x6f, 0x42, 0xc1),
        }
    }
}

/// 接收区的一行，由带样式的片段组成。
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Line {
    spans: Vec<(Style, String)>,
}

impl Line {
    fn span(mut self, style: Style, text: impl Into<String>) -> Self {
        self.spans.push((style, text.into()));
        self
    }

    #[cfg(test)]
    pub(crate) fn data(text: &str) -> Self {
        Self::default().span(Style::Data, text)
    }

    fn spans_from(mut self, other: Line) -> Self {
        self.spans.extend(other.spans);
        self
    }

    /// 去除 ANSI 序列后的纯文本，字符位置与 `layout` 结果一致，用于选区与复制。
    pub(crate) fn plain(&self) -> String {
        layout(self, false).text
    }
}

/// 原版 ASCII 模式续行缩进宽度，等于 `[HH:MM:SS.mmm] ← ASCII: ` 的长度。
const ASCII_PREFIX_WIDTH: usize = 24;

pub(crate) fn event_line(timestamp: &str, kind: &str, message: &str) -> Line {
    let (style, label) = match kind {
        "connected" => (Style::Connected, "连接"),
        "disconnected" => (Style::Disconnected, "断开"),
        "error" => (Style::Error, "错误"),
        "warning" => (Style::Warning, "警告"),
        "peer" => (Style::Peer, "客户端"),
        _ => (Style::Info, "提示"),
    };
    Line::default()
        .span(Style::Timestamp, format!("[{timestamp}]"))
        .span(Style::Data, " ")
        .span(Style::EventMarker, "◆")
        .span(Style::Data, " ")
        .span(style, format!("{label}: {message}"))
}

pub(crate) fn data_lines(
    timestamp: &str,
    received: bool,
    bytes: &[u8],
    mode: &str,
    ansi: bool,
) -> Vec<Line> {
    let arrow = if received { "←" } else { "→" };
    let stamp = format!("[{timestamp}]");
    let hex = || {
        bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let prefix = |label_style: Style, label: &str| {
        Line::default()
            .span(Style::Arrow, arrow)
            .span(Style::Data, " ")
            .span(label_style, label)
            .span(Style::Data, " ")
    };
    match mode {
        "HEX" => vec![Line::default()
            .span(Style::Timestamp, stamp)
            .span(Style::Data, " ")
            .spans_from(prefix(Style::HexLabel, "HEX:"))
            .span(Style::Data, hex())],
        "MIXED" => {
            let mut lines = vec![
                Line::default().span(Style::Timestamp, stamp),
                prefix(Style::HexLabel, "HEX:").span(Style::Data, hex()),
            ];
            for (index, content) in split_lines(&display_ascii(bytes, ansi))
                .into_iter()
                .enumerate()
            {
                lines.push(if index == 0 {
                    prefix(Style::AsciiLabel, "ASCII:").span(Style::Data, content)
                } else {
                    Line::default().span(Style::Data, content)
                });
            }
            lines
        }
        _ => split_lines(&display_ascii(bytes, true))
            .into_iter()
            .enumerate()
            .map(|(index, content)| {
                if index == 0 {
                    Line::default()
                        .span(Style::Timestamp, stamp.clone())
                        .span(Style::Data, " ")
                        .spans_from(prefix(Style::AsciiLabel, "ASCII:"))
                        .span(Style::Data, content)
                } else {
                    Line::default()
                        .span(Style::Data, " ".repeat(ASCII_PREFIX_WIDTH))
                        .span(Style::Data, content)
                }
            })
            .collect(),
    }
}

/// 控制字符显示为 Unicode 控制图符（␍␊ 等），LF 之后换行；`retain_escape` 时保留 ESC 供 ANSI 解析。
fn display_ascii(bytes: &[u8], retain_escape: bool) -> String {
    let mut output = String::with_capacity(bytes.len());
    for byte in bytes {
        match *byte {
            0x1b if retain_escape => output.push('\u{1b}'),
            0..=0x1f => {
                output.push(char::from_u32(0x2400 + u32::from(*byte)).unwrap());
                if *byte == b'\n' {
                    output.push('\n');
                }
            }
            0x20..=0x7e => output.push(char::from(*byte)),
            0x7f => output.push('␡'),
            _ => output.push_str(&format!("\\x{byte:02x}")),
        }
    }
    output
}

fn split_lines(content: &str) -> Vec<String> {
    let mut lines: Vec<String> = content.split('\n').map(str::to_owned).collect();
    if lines.len() > 1 && lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

pub(crate) fn layout(line: &Line, ansi: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    for (style, text) in &line.spans {
        let format = TextFormat {
            font_id: FontId::monospace(12.5),
            color: style.color(),
            line_height: Some(18.0),
            ..Default::default()
        };
        if *style == Style::Data {
            append_ansi(&mut job, text, format, ansi);
        } else {
            job.append(text, 0.0, format);
        }
    }
    job
}

fn append_ansi(job: &mut LayoutJob, text: &str, base: TextFormat, ansi: bool) {
    let mut format = base.clone();
    let mut remaining = text;
    while let Some(position) = remaining.find('\u{1b}') {
        if position > 0 {
            job.append(&remaining[..position], 0.0, format.clone());
        }
        remaining = &remaining[position + 1..];
        if let Some(sequence) = remaining.strip_prefix('[') {
            if let Some(end) = sequence.find(|character: char| ('@'..='~').contains(&character)) {
                if ansi && sequence.as_bytes()[end] == b'm' {
                    apply_sgr(&sequence[..end], &base, &mut format);
                }
                remaining = &sequence[end + 1..];
                continue;
            }
        }
        job.append("␛", 0.0, format.clone());
    }
    if !remaining.is_empty() {
        job.append(remaining, 0.0, format);
    }
}

fn apply_sgr(parameters: &str, base: &TextFormat, format: &mut TextFormat) {
    let codes: Vec<u16> = parameters
        .split(';')
        .map(|value| value.parse().unwrap_or(0))
        .collect();
    let mut index = 0;
    while index < codes.len() {
        let code = codes[index];
        match code {
            0 => *format = base.clone(),
            1 => format.extra_letter_spacing = 0.3,
            22 => format.extra_letter_spacing = 0.0,
            4 => format.underline = Stroke::new(1.0_f32, format.color),
            24 => format.underline = Stroke::NONE,
            30..=37 => format.color = color(code - 30),
            90..=97 => format.color = color(code - 90 + 8),
            40..=47 => format.background = color(code - 40),
            100..=107 => format.background = color(code - 100 + 8),
            39 => format.color = base.color,
            49 => format.background = Color32::TRANSPARENT,
            38 | 48 => {
                let parsed = if codes.get(index + 1) == Some(&5) {
                    codes
                        .get(index + 2)
                        .filter(|value| **value <= 255)
                        .map(|value| (color(*value), 2))
                } else if codes.get(index + 1) == Some(&2)
                    && codes
                        .get(index + 2..index + 5)
                        .is_some_and(|values| values.iter().all(|value| *value <= 255))
                {
                    Some((
                        Color32::from_rgb(
                            codes[index + 2] as u8,
                            codes[index + 3] as u8,
                            codes[index + 4] as u8,
                        ),
                        4,
                    ))
                } else {
                    None
                };
                if let Some((color, consumed)) = parsed {
                    if code == 38 {
                        format.color = color;
                    } else {
                        format.background = color;
                    }
                    index += consumed;
                }
            }
            _ => {}
        }
        index += 1;
    }
}

fn color(index: u16) -> Color32 {
    let palette = [
        0x20252b, 0xc83f49, 0x3c884e, 0xa87500, 0x3569b8, 0x8a55a3, 0x147d86, 0xd9dde2, 0x6b7280,
        0xff6b74, 0x62b875, 0xe0b43c, 0x6495e5, 0xb981cf, 0x35aab5, 0xffffff,
    ];
    if index < 16 {
        let value = palette[index as usize];
        return Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8);
    }
    if index < 232 {
        let levels = [0, 95, 135, 175, 215, 255];
        let value = index - 16;
        return Color32::from_rgb(
            levels[(value / 36) as usize],
            levels[(value / 6 % 6) as usize],
            levels[(value % 6) as usize],
        );
    }
    Color32::from_gray((8 + (index.min(255) - 232) * 10) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_true_color_and_strips_controls_for_copy() {
        let line = Line::data("\u{1b}[38;2;1;2;3mhello\u{1b}[0m");
        assert_eq!(line.plain(), "hello");
        assert_eq!(
            layout(&line, true).sections[0].format.color,
            Color32::from_rgb(1, 2, 3)
        );
        assert_eq!(Line::data("bad\u{1b}[").plain(), "bad␛[");
    }

    #[test]
    fn received_brackets_keep_data_color() {
        let lines = data_lines(
            "15:00:22.863",
            true,
            b"start\r\n[  0.001][a.c]\r\n",
            "ASCII",
            false,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain(), "[15:00:22.863] ← ASCII: start␍␊");
        assert_eq!(
            lines[1].plain(),
            format!("{}[  0.001][a.c]␍␊", " ".repeat(24))
        );
        assert!(lines[1]
            .spans
            .iter()
            .all(|(style, _)| *style == Style::Data));
    }

    #[test]
    fn mixed_and_event_lines_match_original_layout() {
        let lines = data_lines("10:00:00.000", false, b"A\nB", "MIXED", false);
        let plain: Vec<_> = lines.iter().map(Line::plain).collect();
        assert_eq!(
            plain,
            ["[10:00:00.000]", "→ HEX: 41 0A 42", "→ ASCII: A␊", "B"]
        );
        let event = event_line("10:00:00.000", "connected", "串口 COM5 已连接");
        assert_eq!(event.plain(), "[10:00:00.000] ◆ 连接: 串口 COM5 已连接");
        assert_eq!(event.spans.last().unwrap().0, Style::Connected);
    }
}
