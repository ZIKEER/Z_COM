use super::transport::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Server,
    Client,
}

/// Command sent from the main thread to the IO thread for writing data
enum WriteCommand {
    /// Send to a specific address (TCP server mode)
    ToAddr(Vec<u8>, SocketAddr),
    /// Send to the default target (TCP client / UDP)
    ToDefault(Vec<u8>),
}

pub struct SocketTransport {
    protocol: Protocol,
    role: Role,
    host: String,
    port: u16,
    frame_timeout: Duration,
    connected: bool,
    /// Channel to send write commands to the IO thread
    write_tx: Option<mpsc::Sender<WriteCommand>>,
}

impl SocketTransport {
    pub fn new(protocol: Protocol, role: Role, host: &str, port: u16, frame_timeout_ms: u32) -> Self {
        SocketTransport {
            protocol,
            role,
            host: host.to_string(),
            port,
            frame_timeout: Duration::from_millis(frame_timeout_ms as u64),
            connected: false,
            write_tx: None,
        }
    }
}

impl IOTransport for SocketTransport {
    fn connect(&mut self) -> Result<TransportHandle, TransportError> {
        let (data_tx, data_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let (write_tx, write_rx) = mpsc::channel();
        let host = self.host.clone();
        let port = self.port;

        let join_handle = match (self.protocol, self.role) {
            (Protocol::Tcp, Role::Server) => {
                let listener = TcpListener::bind(format!("{}:{}", host, port))?;
                listener.set_nonblocking(true)?;
                thread::spawn(move || {
                    tcp_server_loop(listener, data_tx, event_tx, write_rx);
                })
            }
            (Protocol::Tcp, Role::Client) => {
                let stream = TcpStream::connect_timeout(
                    &format!("{}:{}", host, port)
                        .parse()
                        .map_err(|_| TransportError::Config("Invalid address".to_string()))?,
                    Duration::from_secs(5),
                )?;
                let write_stream = stream
                    .try_clone()
                    .map_err(|e| TransportError::Other(format!("Clone error: {}", e)))?;
                thread::spawn(move || {
                    tcp_client_loop(stream, write_stream, data_tx, write_rx);
                })
            }
            (Protocol::Udp, Role::Server) => {
                let socket = UdpSocket::bind(format!("{}:{}", host, port))?;
                socket.set_nonblocking(true)?;
                let write_socket = socket
                    .try_clone()
                    .map_err(|e| TransportError::Other(format!("Clone error: {}", e)))?;
                thread::spawn(move || {
                    udp_server_loop(socket, write_socket, data_tx, event_tx, write_rx);
                })
            }
            (Protocol::Udp, Role::Client) => {
                let socket = UdpSocket::bind("0.0.0.0:0")?;
                socket.set_nonblocking(true)?;
                let remote_addr: SocketAddr = format!("{}:{}", host, port)
                    .parse()
                    .map_err(|_| TransportError::Config("Invalid address".to_string()))?;
                let write_socket = socket
                    .try_clone()
                    .map_err(|e| TransportError::Other(format!("Clone error: {}", e)))?;
                thread::spawn(move || {
                    udp_client_loop(socket, write_socket, remote_addr, data_tx, write_rx);
                })
            }
        };

        self.connected = true;
        self.write_tx = Some(write_tx);
        Ok(TransportHandle {
            rx: data_rx,
            join_handle: Some(join_handle),
            event_rx: Some(event_rx),
        })
    }

    fn disconnect(&mut self) -> Result<(), TransportError> {
        self.write_tx = None;
        self.connected = false;
        Ok(())
    }

    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        if let Some(ref tx) = self.write_tx {
            tx.send(WriteCommand::ToDefault(data.to_vec()))
                .map_err(|_| TransportError::Other("IO thread disconnected".to_string()))
        } else {
            Err(TransportError::NotConnected)
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn device_list(&self) -> Vec<DeviceEntry> {
        vec![
            DeviceEntry {
                id: "TCP_SERVER".to_string(),
                display: "TCP Server".to_string(),
            },
            DeviceEntry {
                id: "TCP_CLIENT".to_string(),
                display: "TCP Client".to_string(),
            },
            DeviceEntry {
                id: "UDP_SERVER".to_string(),
                display: "UDP Server".to_string(),
            },
            DeviceEntry {
                id: "UDP_CLIENT".to_string(),
                display: "UDP Client".to_string(),
            },
        ]
    }

    fn set_frame_timeout(&mut self, ms: u32) {
        self.frame_timeout = Duration::from_millis(ms as u64);
    }
}

// --- IO Thread Loops ---

fn tcp_server_loop(
    listener: TcpListener,
    data_tx: mpsc::Sender<Vec<u8>>,
    event_tx: mpsc::Sender<TransportEvent>,
    write_rx: mpsc::Receiver<WriteCommand>,
) {
    let mut clients: HashMap<SocketAddr, TcpStream> = HashMap::new();
    let mut buffer = [0u8; 65536];

    loop {
        // Accept new connections
        match listener.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(true).ok();
                let _ = event_tx.send(TransportEvent::ClientConnected(addr.to_string()));
                clients.insert(addr, stream);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        // Process write commands (non-blocking)
        while let Ok(cmd) = write_rx.try_recv() {
            match cmd {
                WriteCommand::ToAddr(ref data, addr) => {
                    if let Some(stream) = clients.get_mut(&addr) {
                        let _ = stream.write_all(data);
                    }
                }
                WriteCommand::ToDefault(ref data) => {
                    // Send to the most recently connected client
                    if let Some((_, stream)) = clients.iter_mut().next() {
                        let _ = stream.write_all(data);
                    }
                }
            }
        }

        // Read from clients
        let mut to_remove = Vec::new();
        for (addr, stream) in clients.iter_mut() {
            match stream.read(&mut buffer) {
                Ok(0) => to_remove.push(*addr),
                Ok(n) => {
                    let _ = data_tx.send(buffer[..n].to_vec());
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => to_remove.push(*addr),
            }
        }
        for addr in to_remove {
            clients.remove(&addr);
            let _ = event_tx.send(TransportEvent::ClientDisconnected(addr.to_string()));
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn tcp_client_loop(
    mut read_stream: TcpStream,
    mut write_stream: TcpStream,
    data_tx: mpsc::Sender<Vec<u8>>,
    write_rx: mpsc::Receiver<WriteCommand>,
) {
    read_stream.set_nonblocking(true).ok();
    let mut buffer = [0u8; 65536];

    loop {
        // Read data
        match read_stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let _ = data_tx.send(buffer[..n].to_vec());
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        // Process write commands (non-blocking)
        while let Ok(cmd) = write_rx.try_recv() {
            let data = match cmd {
                WriteCommand::ToDefault(data) => data,
                WriteCommand::ToAddr(data, _) => data,
            };
            if write_stream.write_all(&data).is_err() {
                break;
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn udp_server_loop(
    read_socket: UdpSocket,
    write_socket: UdpSocket,
    data_tx: mpsc::Sender<Vec<u8>>,
    event_tx: mpsc::Sender<TransportEvent>,
    write_rx: mpsc::Receiver<WriteCommand>,
) {
    let mut buffer = [0u8; 65536];
    let mut last_client: Option<SocketAddr> = None;

    loop {
        // Read data
        match read_socket.recv_from(&mut buffer) {
            Ok((n, addr)) => {
                last_client = Some(addr);
                let _ = event_tx.send(TransportEvent::ClientConnected(addr.to_string()));
                let _ = data_tx.send(buffer[..n].to_vec());
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        // Process write commands (non-blocking)
        while let Ok(cmd) = write_rx.try_recv() {
            let data = match cmd {
                WriteCommand::ToDefault(data) => data,
                WriteCommand::ToAddr(data, _) => data,
            };
            if let Some(addr) = last_client {
                let _ = write_socket.send_to(&data, addr);
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn udp_client_loop(
    read_socket: UdpSocket,
    write_socket: UdpSocket,
    remote_addr: SocketAddr,
    data_tx: mpsc::Sender<Vec<u8>>,
    write_rx: mpsc::Receiver<WriteCommand>,
) {
    let mut buffer = [0u8; 65536];

    loop {
        // Read data
        match read_socket.recv_from(&mut buffer) {
            Ok((n, _addr)) => {
                let _ = data_tx.send(buffer[..n].to_vec());
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        // Process write commands (non-blocking)
        while let Ok(cmd) = write_rx.try_recv() {
            let data = match cmd {
                WriteCommand::ToDefault(data) => data,
                WriteCommand::ToAddr(data, _) => data,
            };
            let _ = write_socket.send_to(&data, remote_addr);
        }

        thread::sleep(Duration::from_millis(10));
    }
}
