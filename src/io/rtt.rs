use super::transport::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Command to send data to the target via the IO thread
enum WriteCommand {
    Data(Vec<u8>),
    Stop,
}

pub struct RttTransport {
    frame_timeout: Duration,
    chip: String,
    speed: u32,
    reset: bool,
    serial_no: Option<String>,
    /// Channel to send write commands to the IO thread
    write_tx: Option<mpsc::Sender<WriteCommand>>,
}

impl RttTransport {
    pub fn new(
        chip: &str,
        speed: u32,
        reset: bool,
        serial_no: Option<&str>,
        frame_timeout_ms: u32,
    ) -> Self {
        RttTransport {
            frame_timeout: Duration::from_millis(frame_timeout_ms as u64),
            chip: chip.to_string(),
            speed,
            reset,
            serial_no: serial_no.map(|s| s.to_string()),
            write_tx: None,
        }
    }
}

impl IOTransport for RttTransport {
    fn connect(&mut self) -> Result<TransportHandle, TransportError> {
        use probe_rs::probe::list::Lister;
        use probe_rs::Permissions;

        let lister = Lister::new();
        let probes = lister.list_all();

        let probe = if let Some(ref sn) = self.serial_no {
            probes
                .into_iter()
                .find(|p| p.serial_number.as_deref() == Some(sn.as_str()))
                .ok_or(TransportError::Config("Probe not found".to_string()))?
        } else {
            probes
                .into_iter()
                .next()
                .ok_or(TransportError::Config("No probes found".to_string()))?
        };

        let chip = self.chip.clone();
        let reset = self.reset;
        let frame_timeout = self.frame_timeout;

        log::info!("[RTT] Connecting to chip: '{}'", chip);

        // Open probe
        let probe_handle = probe
            .open()
            .map_err(|e| TransportError::Other(format!("Open probe failed: {}", e)))?;

        // Attach to target
        let mut session = probe_handle
            .attach(&chip, Permissions::default())
            .map_err(|e| {
                log::error!("[RTT] Failed to attach to chip '{}': {}", chip, e);
                TransportError::Other(format!("Attach failed: {}", e))
            })?;

        // Reset target if requested
        if reset {
            let mut core = session
                .core(0)
                .map_err(|e| TransportError::Other(format!("Core access failed: {}", e)))?;
            core.reset()
                .map_err(|e| TransportError::Other(format!("Reset failed: {}", e)))?;
            drop(core);
        }

        // Get core handle and attach to RTT
        let mut core = session
            .core(0)
            .map_err(|e| TransportError::Other(format!("Core access failed: {}", e)))?;

        let rtt = probe_rs::rtt::Rtt::attach(&mut core)
            .map_err(|e| TransportError::Other(format!("RTT attach failed: {}", e)))?;

        // Drop core before moving session into thread
        drop(core);

        // Create channels
        let (data_tx, data_rx) = mpsc::channel();
        let (write_tx, write_rx) = mpsc::channel();

        // Spawn IO thread - owns session and rtt
        thread::spawn(move || {
            rtt_io_loop(session, rtt, data_tx, write_rx, frame_timeout);
        });

        self.write_tx = Some(write_tx);

        Ok(TransportHandle {
            rx: data_rx,
            join_handle: None,
            event_rx: None,
        })
    }

    fn disconnect(&mut self) -> Result<(), TransportError> {
        if let Some(tx) = self.write_tx.take() {
            let _ = tx.send(WriteCommand::Stop);
        }
        Ok(())
    }

    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if let Some(ref tx) = self.write_tx {
            tx.send(WriteCommand::Data(data.to_vec()))
                .map_err(|_| TransportError::Other("RTT IO thread disconnected".to_string()))
        } else {
            Err(TransportError::NotConnected)
        }
    }

    fn is_connected(&self) -> bool {
        self.write_tx.is_some()
    }

    fn device_list(&self) -> Vec<DeviceEntry> {
        use probe_rs::probe::list::Lister;
        let lister = Lister::new();
        let probes = lister.list_all();
        log::info!("[RTT] Found {} probes", probes.len());
        probes
            .iter()
            .map(|p| DeviceEntry {
                id: p.serial_number.clone().unwrap_or_default(),
                display: format!(
                    "{} - {}",
                    p.identifier,
                    p.serial_number.as_deref().unwrap_or("")
                ),
            })
            .collect()
    }

    fn set_frame_timeout(&mut self, ms: u32) {
        self.frame_timeout = Duration::from_millis(ms as u64);
    }
}

/// Combined IO loop - handles both reading and writing on a single thread
/// because probe_rs::Core is not Send
fn rtt_io_loop(
    mut session: probe_rs::Session,
    mut rtt: probe_rs::rtt::Rtt,
    data_tx: mpsc::Sender<Vec<u8>>,
    write_rx: mpsc::Receiver<WriteCommand>,
    frame_timeout: Duration,
) {
    use std::time::Instant;

    // Get core handle
    let mut core = match session.core(0) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut buffer = Vec::new();
    let mut last_receive = Instant::now();
    let mut buffer_start = Instant::now();
    let mut byte_buf = [0u8; 8192];

    loop {
        // Process write commands (non-blocking)
        while let Ok(cmd) = write_rx.try_recv() {
            match cmd {
                WriteCommand::Data(data) => {
                    if let Some(dc) = rtt.down_channel(0) {
                        let _ = dc.write(&mut core, &data);
                    }
                }
                WriteCommand::Stop => {
                    if !buffer.is_empty() {
                        flush(&mut buffer, &data_tx);
                    }
                    return;
                }
            }
        }

        // Read data from RTT
        if let Some(uc) = rtt.up_channel(0) {
            match uc.read(&mut core, &mut byte_buf) {
                Ok(n) if n > 0 => {
                    let now = Instant::now();
                    if !buffer.is_empty() && now.duration_since(last_receive) > frame_timeout {
                        flush(&mut buffer, &data_tx);
                    }
                    if buffer.is_empty() {
                        buffer_start = now;
                    }
                    buffer.extend_from_slice(&byte_buf[..n]);
                    last_receive = now;
                    if buffer.len() >= 4096
                        || now.duration_since(buffer_start) >= frame_timeout
                    {
                        flush(&mut buffer, &data_tx);
                    }
                }
                Ok(_) => {
                    // No data available
                    if !buffer.is_empty() && last_receive.elapsed() > frame_timeout {
                        flush(&mut buffer, &data_tx);
                    }
                    thread::sleep(Duration::from_millis(2));
                }
                Err(_) => {
                    if !buffer.is_empty() {
                        flush(&mut buffer, &data_tx);
                    }
                    break;
                }
            }
        } else {
            thread::sleep(Duration::from_millis(2));
        }
    }
}

fn flush(buffer: &mut Vec<u8>, tx: &mpsc::Sender<Vec<u8>>) {
    if !buffer.is_empty() {
        let _ = tx.send(buffer.clone());
        buffer.clear();
    }
}
