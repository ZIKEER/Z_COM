/// Data display modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Hex,
    Ascii,
    Mixed,
}

/// Control character map: maps bytes 0x00-0x1F and 0x7F to Unicode Control Pictures (U+2400-U+2421)
const CONTROL_CHAR_MAP: [char; 128] = [
    '\u{2400}', '\u{2401}', '\u{2402}', '\u{2403}', '\u{2404}', '\u{2405}', '\u{2406}', '\u{2407}',
    '\u{2408}', '\u{2409}', '\u{240A}', '\u{240B}', '\u{240C}', '\u{240D}', '\u{240E}', '\u{240F}',
    '\u{2410}', '\u{2411}', '\u{2412}', '\u{2413}', '\u{2414}', '\u{2415}', '\u{2416}', '\u{2417}',
    '\u{2418}', '\u{2419}', '\u{241A}', '\u{241B}', '\u{241C}', '\u{241D}', '\u{241E}', '\u{241F}',
    ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
    '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '\u{2421}',
];

/// Convert bytes to space-separated uppercase hex string
pub fn bytes_to_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Convert bytes to ASCII representation with control character visualization
pub fn bytes_to_ascii(data: &[u8]) -> String {
    let mut result = String::new();
    for &b in data {
        if (b as usize) < 128 {
            result.push(CONTROL_CHAR_MAP[b as usize]);
            if b == 0x0A {
                result.push('\n');
            }
        } else {
            result.push_str(&format!("\\x{:02X}", b));
        }
    }
    result
}

/// Format data for display based on mode
pub fn format_display(data: &[u8], mode: DisplayMode) -> String {
    match mode {
        DisplayMode::Hex => bytes_to_hex(data),
        DisplayMode::Ascii => bytes_to_ascii(data),
        DisplayMode::Mixed => {
            let hex = bytes_to_hex(data);
            let ascii = bytes_to_ascii(data);
            format!("{}\n  → {}", hex, ascii)
        }
    }
}

/// Validate hex input text, returns true if valid
pub fn validate_hex_input(text: &str) -> bool {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return true;
    }
    hex::decode(&cleaned).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_hex() {
        assert_eq!(bytes_to_hex(&[0x4F, 0x4B]), "4F 4B");
        assert_eq!(bytes_to_hex(&[]), "");
        assert_eq!(bytes_to_hex(&[0x00, 0xFF]), "00 FF");
    }

    #[test]
    fn test_bytes_to_ascii() {
        assert_eq!(bytes_to_ascii(&[0x4F, 0x4B]), "OK");
        assert_eq!(bytes_to_ascii(&[0x0D, 0x0A]), "\u{240D}\u{240A}\n");
    }

    #[test]
    fn test_format_display_hex() {
        let result = format_display(&[0x4F, 0x4B], DisplayMode::Hex);
        assert_eq!(result, "4F 4B");
    }

    #[test]
    fn test_format_display_ascii() {
        let result = format_display(&[0x4F, 0x4B], DisplayMode::Ascii);
        assert_eq!(result, "OK");
    }

    #[test]
    fn test_format_display_mixed() {
        let result = format_display(&[0x4F, 0x4B], DisplayMode::Mixed);
        assert!(result.contains("4F 4B"));
        assert!(result.contains("OK"));
    }

    #[test]
    fn test_validate_hex_input_valid() {
        assert!(validate_hex_input("4F 4B"));
        assert!(validate_hex_input("4F4B"));
        assert!(validate_hex_input(""));
    }

    #[test]
    fn test_validate_hex_input_invalid() {
        assert!(!validate_hex_input("ZZ"));
        assert!(!validate_hex_input("4F 4"));
    }
}
