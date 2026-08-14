use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use chrono::Local;

const MAX_LOG_SIZE: u64 = 50 * 1024 * 1024;

pub struct Logger {
    directory: PathBuf,
    path: PathBuf,
    writer: BufWriter<File>,
    last_flush: Instant,
    sequence: u32,
}

impl Logger {
    pub fn new(data_root: &Path) -> io::Result<Self> {
        let directory = data_root.join("logs");
        fs::create_dir_all(&directory)?;
        let (path, writer) = open_log(&directory, 0)?;
        Ok(Self {
            directory,
            path,
            writer,
            last_flush: Instant::now(),
            sequence: 0,
        })
    }

    pub fn log_data(&mut self, direction: &str, bytes: &[u8]) {
        let arrow = if direction == "received" { "<-" } else { "->" };
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let hex = bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii = String::from_utf8_lossy(bytes)
            .chars()
            .flat_map(|value| value.escape_default())
            .collect::<String>();
        let _ = writeln!(
            self.writer,
            "[{timestamp}]\n {arrow} HEX: {hex}\n {arrow} ASCII: {ascii}"
        );
        self.maintain();
    }

    pub fn log_event(&mut self, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(self.writer, "[{timestamp}] {message}");
        self.maintain();
    }

    fn maintain(&mut self) {
        if self.last_flush.elapsed() >= Duration::from_secs(1) {
            let _ = self.writer.flush();
            self.last_flush = Instant::now();
            if fs::metadata(&self.path).is_ok_and(|metadata| metadata.len() >= MAX_LOG_SIZE) {
                self.sequence += 1;
                if let Ok((path, writer)) = open_log(&self.directory, self.sequence) {
                    self.path = path;
                    self.writer = writer;
                }
            }
        }
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}

fn open_log(directory: &Path, sequence: u32) -> io::Result<(PathBuf, BufWriter<File>)> {
    let suffix = if sequence == 0 {
        String::new()
    } else {
        format!("_{sequence}")
    };
    let path = directory.join(format!(
        "log_{}{}.txt",
        Local::now().format("%Y-%m-%d_%H%M%S"),
        suffix
    ));
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((path, BufWriter::new(file)))
}
