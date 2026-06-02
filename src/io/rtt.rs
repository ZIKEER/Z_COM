#[cfg(feature = "rtt")]
use super::transport::*;
#[cfg(feature = "rtt")]
use std::sync::mpsc;
#[cfg(feature = "rtt")]
use std::thread;
#[cfg(feature = "rtt")]
use std::time::Duration;

#[cfg(feature = "rtt")]
pub struct RttTransport {
    handle: Option<TransportHandle>,
    frame_timeout: Duration,
    chip: String,
    speed: u32,
    reset: bool,
    serial_no: Option<String>,
}

#[cfg(feature = "rtt")]
impl RttTransport {
    pub fn new(chip: &str, speed: u32, reset: bool, serial_no: Option<&str>, frame_timeout_ms: u32) -> Self {
        RttTransport { handle: None, frame_timeout: Duration::from_millis(frame_timeout_ms as u64), chip: chip.to_string(), speed, reset, serial_no: serial_no.map(|s| s.to_string()) }
    }
}

#[cfg(feature = "rtt")]
impl IOTransport for RttTransport {
    fn connect(&mut self) -> Result<TransportHandle, TransportError> {
        use probe_rs::probe::list::list_probes;
        use probe_rs::{Session, Permissions};

        let probes = list_probes();
        let probe_info = if let Some(ref sn) = self.serial_no {
            probes.into_iter().find(|p| p.serial_number.as_deref() == Some(sn.as_str())).ok_or(TransportError::Config("Probe not found".to_string()))?
        } else {
            probes.into_iter().next().ok_or(TransportError::Config("No probes found".to_string()))?
        };

        let mut session = Session::attach(probe_info, &self.chip, Permissions::default())?;
        if self.reset { session.core(0)?.reset()?; }
        let mut rtt = probe_rs::rtt::Rtt::attach(&session)?;
        let up_channel = rtt.up_channel(0).ok_or(TransportError::Config("No RTT up channel".to_string()))?;
        let (tx, rx) = mpsc::channel();
        let frame_timeout = self.frame_timeout;
        let join_handle = thread::spawn(move || { rtt_reader_loop(up_channel, tx, frame_timeout); });
        Ok(TransportHandle { rx, join_handle: Some(join_handle), event_rx: None })
    }

    fn disconnect(&mut self) -> Result<(), TransportError> { self.handle = None; Ok(()) }
    fn send_bytes(&self, _data: &[u8]) -> Result<(), TransportError> { Err(TransportError::Other("RTT send not yet implemented".to_string())) }
    fn is_connected(&self) -> bool { self.handle.is_some() }
    fn device_list(&self) -> Vec<DeviceEntry> {
        use probe_rs::probe::list::list_probes;
        list_probes().iter().map(|p| DeviceEntry { id: p.serial_number.clone().unwrap_or_default(), display: format!("{} - {}", p.identifier, p.serial_number.as_deref().unwrap_or("")) }).collect()
    }
    fn set_frame_timeout(&mut self, ms: u32) { self.frame_timeout = Duration::from_millis(ms as u64); }
}

#[cfg(feature = "rtt")]
fn rtt_reader_loop(mut up_channel: probe_rs::rtt::UpChannel, tx: mpsc::Sender<Vec<u8>>, frame_timeout: Duration) {
    use std::time::Instant;
    let mut buffer = Vec::new();
    let mut last_receive = Instant::now();
    let mut buffer_start = Instant::now();
    let mut byte_buf = [0u8; 8192];

    loop {
        match up_channel.read(&mut byte_buf) {
            Ok(n) if n > 0 => {
                let now = Instant::now();
                if !buffer.is_empty() && now.duration_since(last_receive) > frame_timeout { flush(&mut buffer, &tx); }
                if buffer.is_empty() { buffer_start = now; }
                buffer.extend_from_slice(&byte_buf[..n]);
                last_receive = now;
                if buffer.len() >= 4096 || now.duration_since(buffer_start) >= frame_timeout { flush(&mut buffer, &tx); }
            }
            Ok(_) => {
                if !buffer.is_empty() && last_receive.elapsed() > frame_timeout { flush(&mut buffer, &tx); }
                thread::sleep(Duration::from_millis(2));
            }
            Err(_) => { if !buffer.is_empty() { flush(&mut buffer, &tx); } break; }
        }
    }
}

#[cfg(feature = "rtt")]
fn flush(buffer: &mut Vec<u8>, tx: &mpsc::Sender<Vec<u8>>) {
    if !buffer.is_empty() { let _ = tx.send(buffer.clone()); buffer.clear(); }
}
