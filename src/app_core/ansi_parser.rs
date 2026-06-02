use std::fmt::Write;

/// ANSI color codes for foreground
const FG_COLORS: &[(u32, &str)] = &[
    (30, "#000000"), (31, "#AA0000"), (32, "#00AA00"), (33, "#AA5500"),
    (34, "#0000AA"), (35, "#AA00AA"), (36, "#00AAAA"), (37, "#AAAAAA"),
    (90, "#555555"), (91, "#FF5555"), (92, "#55FF55"), (93, "#FFFF55"),
    (94, "#5555FF"), (95, "#FF55FF"), (96, "#55FFFF"), (97, "#FFFFFF"),
];

/// Escape HTML special characters
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "<br>")
}

/// Parse ANSI SGR parameters and return CSS styles
fn parse_sgr(params: &str) -> Option<Vec<(String, String)>> {
    if params.is_empty() {
        return None; // Treat as reset
    }

    let parts: Vec<u32> = params
        .split(';')
        .filter_map(|s| s.parse().ok())
        .collect();

    if parts.is_empty() {
        return None;
    }

    let mut styles = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            0 => return None,
            1 => styles.push(("font-weight".to_string(), "bold".to_string())),
            4 => styles.push(("text-decoration".to_string(), "underline".to_string())),
            30..=37 => {
                if let Some((_, color)) = FG_COLORS.iter().find(|(code, _)| *code == parts[i]) {
                    styles.push(("color".to_string(), color.to_string()));
                }
            }
            90..=97 => {
                if let Some((_, color)) = FG_COLORS.iter().find(|(code, _)| *code == parts[i]) {
                    styles.push(("color".to_string(), color.to_string()));
                }
            }
            40..=47 => {
                let bg_idx = (parts[i] - 40) as usize;
                if bg_idx < 8 {
                    styles.push(("background-color".to_string(), FG_COLORS[bg_idx].1.to_string()));
                }
            }
            100..=107 => {
                let bg_idx = (parts[i] - 100 + 8) as usize;
                if bg_idx < 16 {
                    styles.push(("background-color".to_string(), FG_COLORS[bg_idx].1.to_string()));
                }
            }
            38 if i + 1 < parts.len() && parts[i + 1] == 2 && i + 4 < parts.len() => {
                styles.push(("color".to_string(), format!("#{:02X}{:02X}{:02X}", parts[i+2], parts[i+3], parts[i+4])));
                i += 5;
                continue;
            }
            38 if i + 1 < parts.len() && parts[i + 1] == 5 => {
                i += 2;
                continue;
            }
            48 if i + 1 < parts.len() && parts[i + 1] == 2 && i + 4 < parts.len() => {
                styles.push(("background-color".to_string(), format!("#{:02X}{:02X}{:02X}", parts[i+2], parts[i+3], parts[i+4])));
                i += 5;
                continue;
            }
            48 if i + 1 < parts.len() && parts[i + 1] == 5 => {
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }

    Some(styles)
}

/// Convert bytes with ANSI escape sequences to HTML
pub fn bytes_to_html(data: &[u8], to_ascii: fn(&[u8]) -> String) -> String {
    let mut result = String::new();
    let mut current_styles: Vec<(String, String)> = Vec::new();
    let mut span_open = false;
    let mut i = 0;

    while i < data.len() {
        if data[i] == 0x1B && i + 1 < data.len() && data[i + 1] == b'[' {
            i += 2;
            let mut params = String::new();
            while i < data.len() && (data[i] >= 0x30 && data[i] <= 0x3F) {
                params.push(data[i] as char);
                i += 1;
            }
            while i < data.len() && (data[i] >= 0x20 && data[i] <= 0x2F) {
                i += 1;
            }
            if i < data.len() && (data[i] >= 0x40 && data[i] <= 0x7E) {
                let terminator = data[i] as char;
                i += 1;

                if terminator == 'm' {
                    if let Some(new_styles) = parse_sgr(&params) {
                        if span_open {
                            result.push_str("</span>");
                            span_open = false;
                        }
                        current_styles = new_styles;
                        if !current_styles.is_empty() {
                            let style_str: String = current_styles
                                .iter()
                                .map(|(k, v)| format!("{}:{};", k, v))
                                .collect();
                            write!(result, "<span style=\"{}\">", style_str).unwrap();
                            span_open = true;
                        }
                    }
                }
            }
        } else {
            let text = to_ascii(&[data[i]]);
            result.push_str(&escape_html(&text));
            i += 1;
        }
    }

    if span_open {
        result.push_str("</span>");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::data_handler::bytes_to_ascii;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("a<b>c"), "a&lt;b&gt;c");
        assert_eq!(escape_html("a&b"), "a&amp;b");
        assert_eq!(escape_html("a\nb"), "a<br>b");
    }

    #[test]
    fn test_parse_sgr_reset() {
        assert_eq!(parse_sgr("0"), None);
        assert_eq!(parse_sgr(""), None);
    }

    #[test]
    fn test_parse_sgr_bold() {
        let styles = parse_sgr("1").unwrap();
        assert_eq!(styles, vec![("font-weight".to_string(), "bold".to_string())]);
    }

    #[test]
    fn test_parse_sgr_fg_color() {
        let styles = parse_sgr("31").unwrap();
        assert_eq!(styles, vec![("color".to_string(), "#AA0000".to_string())]);
    }

    #[test]
    fn test_parse_sgr_truecolor() {
        let styles = parse_sgr("38;2;255;128;0").unwrap();
        assert_eq!(styles, vec![("color".to_string(), "#FF8000".to_string())]);
    }

    #[test]
    fn test_bytes_to_html_no_escape() {
        let html = bytes_to_html(b"Hello", bytes_to_ascii);
        assert_eq!(html, "Hello");
    }

    #[test]
    fn test_bytes_to_html_with_color() {
        let data = b"\x1b[31mRed\x1b[0m";
        let html = bytes_to_html(data, bytes_to_ascii);
        assert!(html.contains("<span style=\"color:#AA0000;\">"));
        assert!(html.contains("Red"));
        assert!(html.contains("</span>"));
    }
}
