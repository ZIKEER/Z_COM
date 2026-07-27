use std::sync::mpsc;
use std::thread::JoinHandle;

#[derive(Debug)]
pub enum TransportError {
    Io(std::io::Error),
    Probe(probe_rs::Error),
    NotConnected,
    Config(String),
    Other(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Io(e) => write!(f, "IO error: {}", e),
            TransportError::Probe(e) => write!(f, "Probe error: {}", e),
            TransportError::NotConnected => write!(f, "Not connected"),
            TransportError::Config(msg) => write!(f, "Config error: {}", msg),
            TransportError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for TransportError {}
impl From<std::io::Error> for TransportError { fn from(e: std::io::Error) -> Self { TransportError::Io(e) } }
impl From<probe_rs::Error> for TransportError { fn from(e: probe_rs::Error) -> Self { TransportError::Probe(e) } }

#[derive(Debug, Clone)]
pub struct DeviceEntry { pub id: String, pub display: String }

#[derive(Debug, Clone)]
pub enum TransportEvent {
    ClientConnected(String),
    ClientDisconnected(String),
    Error(String),
}

pub struct TransportHandle {
    pub rx: mpsc::Receiver<Vec<u8>>,
    pub join_handle: Option<JoinHandle<()>>,
    pub event_rx: Option<mpsc::Receiver<TransportEvent>>,
}

pub trait IOTransport: Send {
    fn connect(&mut self) -> Result<TransportHandle, TransportError>;
    fn disconnect(&mut self) -> Result<(), TransportError>;
    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn is_connected(&self) -> bool;
    fn device_list(&self) -> Vec<DeviceEntry>;
    fn set_frame_timeout(&mut self, ms: u32);
}
