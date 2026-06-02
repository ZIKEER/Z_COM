use super::transport::*;
use serialport::available_ports;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub struct SerialTransport {
    port: Option<Box<dyn serialport::SerialPort>>,
    frame_timeout: Duration,
    port_name: String,
    baudrate: u32,
}

impl SerialTransport {
    pub fn new(port_name: &str, baudrate: u32, frame_timeout_ms: u32) -> Self {
        SerialTransport { port: None, frame_timeout: Duration::from_millis(frame_timeout_ms as u64), port_name: port_name.to_string(), baudrate }
    }
}

impl IOTransport for SerialTransport {
    fn connect(&mut self) -> Result<TransportHandle, TransportError> {
        let port = serialport::new(&self.port_name, self.baudrate)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| TransportError::Other(format!("Failed to open serial port: {}", e)))?;
        let (tx, rx) = mpsc::channel();
        let frame_timeout = self.frame_timeout;
        let port_clone = port.try_clone().map_err(|e| TransportError::Other(format!("Clone error: {}", e)))?;
        let join_handle = thread::spawn(move || { reader_loop(port_clone, tx, frame_timeout); });
        self.port = Some(port);
        Ok(TransportHandle { rx, join_handle: Some(join_handle), event_rx: None })
    }

    fn disconnect(&mut self) -> Result<(), TransportError> { self.port = None; Ok(()) }
    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if let Some(ref mut port) = self.port { port.write_all(data)?; Ok(()) } else { Err(TransportError::NotConnected) }
    }
    fn is_connected(&self) -> bool { self.port.is_some() }
    fn device_list(&self) -> Vec<DeviceEntry> {
        available_ports().unwrap_or_default().iter().map(|p| DeviceEntry {
            id: p.port_name.clone(),
            display: format!("{} - {:?}", p.port_name, p.port_type)
        }).collect()
    }
    fn set_frame_timeout(&mut self, ms: u32) { self.frame_timeout = Duration::from_millis(ms as u64); }
}

fn reader_loop(mut port: Box<dyn serialport::SerialPort>, tx: mpsc::Sender<Vec<u8>>, frame_timeout: Duration) {
    let mut buffer = Vec::new();
    let mut last_receive = Instant::now();
    let mut buffer_start = Instant::now();
    let mut byte_buf = [0u8; 4096];

    loop {
        match port.read(&mut byte_buf) {
            Ok(n) if n > 0 => {
                let now = Instant::now();
                if !buffer.is_empty() && now.duration_since(last_receive) > frame_timeout { flush(&mut buffer, &tx); }
                if buffer.is_empty() { buffer_start = now; }
                buffer.extend_from_slice(&byte_buf[..n]);
                last_receive = now;
                if now.duration_since(buffer_start) >= frame_timeout { flush(&mut buffer, &tx); }
            }
            Ok(_) => {
                if !buffer.is_empty() && last_receive.elapsed() > frame_timeout { flush(&mut buffer, &tx); }
                thread::sleep(Duration::from_millis(10));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                if !buffer.is_empty() && last_receive.elapsed() > frame_timeout { flush(&mut buffer, &tx); }
            }
            Err(_) => { if !buffer.is_empty() { flush(&mut buffer, &tx); } break; }
        }
    }
}

fn flush(buffer: &mut Vec<u8>, tx: &mpsc::Sender<Vec<u8>>) {
    if !buffer.is_empty() { let _ = tx.send(buffer.clone()); buffer.clear(); }
}
