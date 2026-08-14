use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use chrono::Local;

pub struct Logger {
    directory: PathBuf,
    date: String,
    writer: BufWriter<File>,
    last_flush: Instant,
}

impl Logger {
    pub fn new(data_root: &Path) -> io::Result<Self> {
        let directory = data_root.join("logs");
        fs::create_dir_all(&directory)?;
        let date = current_date();
        let writer = open_log(&directory, &date)?;
        Ok(Self {
            directory,
            date,
            writer,
            last_flush: Instant::now(),
        })
    }

    pub fn log_data(&mut self, direction: &str, bytes: &[u8]) {
        self.switch_date_if_needed();
        let arrow = if direction == "received" {
            "←"
        } else {
            "→"
        };
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let hex = bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii = display_ascii(bytes);
        let _ = writeln!(
            self.writer,
            "[{timestamp}]\n {arrow} HEX: {hex}\n {arrow} ASCII: {ascii}"
        );
        self.maintain();
    }

    pub fn log_event(&mut self, message: &str) {
        self.switch_date_if_needed();
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(self.writer, "[{timestamp}] {message}");
        self.maintain();
    }

    fn maintain(&mut self) {
        if self.last_flush.elapsed() >= Duration::from_secs(1) {
            let _ = self.writer.flush();
            self.last_flush = Instant::now();
        }
    }

    fn switch_date_if_needed(&mut self) {
        let date = current_date();
        if date == self.date {
            return;
        }
        if let Ok(writer) = open_log(&self.directory, &date) {
            let _ = self.writer.flush();
            self.writer = writer;
            self.date = date;
            self.last_flush = Instant::now();
        }
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}

fn current_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn open_log(directory: &Path, date: &str) -> io::Result<BufWriter<File>> {
    let path = directory.join(format!("log_{date}.txt"));
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok(BufWriter::new(file))
}

fn display_ascii(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &byte in bytes {
        match byte {
            0x00..=0x1f => {
                if let Some(symbol) = char::from_u32(0x2400 + u32::from(byte)) {
                    result.push(symbol);
                }
                if byte == b'\n' {
                    result.push('\n');
                }
            }
            0x20..=0x7e => result.push(char::from(byte)),
            0x7f => result.push('␡'),
            _ => result.push_str(&format!("\\x{byte:02x}")),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::display_ascii;

    #[test]
    fn formats_control_and_non_ascii_bytes_for_logs() {
        assert_eq!(
            display_ascii(&[0x00, b'A', b'\n', 0x7f, 0x80]),
            "␀A␊\n␡\\x80"
        );
    }
}
