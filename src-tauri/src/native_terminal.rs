use eframe::egui::{Color32, FontId, Stroke, TextFormat, text::LayoutJob};

pub(crate) fn layout(text: &str, ansi: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let base = TextFormat {
        font_id: FontId::monospace(12.5),
        color: Color32::from_gray(32),
        line_height: Some(18.0),
        ..Default::default()
    };
    let mut format = base.clone();
    let mut remaining = text;
    if let Some(end) = text
        .strip_prefix('[')
        .and_then(|value| value.find(']').map(|index| index + 1))
    {
        let mut timestamp = base.clone();
        timestamp.color = Color32::from_rgb(0, 139, 139);
        job.append(&text[..=end], 0.0, timestamp);
        remaining = &text[end + 1..];
    }
    if let Some(position) = remaining.find("ASCII:").or_else(|| remaining.find("HEX:")) {
        if position < 8 {
            let end = position
                + if remaining[position..].starts_with("ASCII:") {
                    6
                } else {
                    4
                };
            let mut prefix = base.clone();
            prefix.color = Color32::from_rgb(31, 111, 235);
            job.append(&remaining[..position], 0.0, prefix);
            let mut label = base.clone();
            label.color = if end - position == 6 {
                Color32::from_rgb(60, 136, 78)
            } else {
                Color32::from_rgb(168, 117, 0)
            };
            job.append(&remaining[position..end], 0.0, label);
            remaining = &remaining[end..];
        }
    }
    while let Some(position) = remaining.find('\u{1b}') {
        if position > 0 {
            job.append(&remaining[..position], 0.0, format.clone());
        }
        remaining = &remaining[position + 1..];
        if let Some(sequence) = remaining.strip_prefix('[') {
            if let Some(end) = sequence.find(|character: char| ('@'..='~').contains(&character)) {
                if ansi && sequence.as_bytes()[end] == b'm' {
                    let codes: Vec<u16> = sequence[..end]
                        .split(';')
                        .map(|value| value.parse().unwrap_or(0))
                        .collect();
                    let mut index = 0;
                    while index < codes.len() {
                        let code = codes[index];
                        match code {
                            0 => format = base.clone(),
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
                                    && codes.get(index + 2..index + 5).is_some_and(|values| {
                                        values.iter().all(|value| *value <= 255)
                                    })
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
                remaining = &sequence[end + 1..];
                continue;
            }
        }
        job.append("␛", 0.0, format.clone());
    }
    if !remaining.is_empty() {
        job.append(remaining, 0.0, format);
    }
    job
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

pub(crate) fn plain(text: &str) -> String {
    layout(text, false).text
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_true_color_and_strips_controls_for_copy() {
        let text = "\u{1b}[38;2;1;2;3mhello\u{1b}[0m";
        assert_eq!(plain(text), "hello");
        assert_eq!(
            layout(text, true).sections[0].format.color,
            Color32::from_rgb(1, 2, 3)
        );
        assert_eq!(plain("bad\u{1b}["), "bad␛[");
    }
}
