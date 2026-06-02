use super::transport::*;
use std::net::{TcpListener, TcpStream, UdpSocket, SocketAddr};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::io::Read;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol { Tcp, Udp }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role { Server, Client }

pub struct SocketTransport {
    protocol: Protocol, role: Role, host: String, port: u16,
    frame_timeout: Duration, connected: bool,
}

impl SocketTransport {
    pub fn new(protocol: Protocol, role: Role, host: &str, port: u16, frame_timeout_ms: u32) -> Self {
        SocketTransport { protocol, role, host: host.to_string(), port, frame_timeout: Duration::from_millis(frame_timeout_ms as u64), connected: false }
    }
}

impl IOTransport for SocketTransport {
    fn connect(&mut self) -> Result<TransportHandle, TransportError> {
        let (tx, rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let frame_timeout = self.frame_timeout;
        let host = self.host.clone();
        let port = self.port;

        let join_handle = match (self.protocol, self.role) {
            (Protocol::Tcp, Role::Server) => {
                let listener = TcpListener::bind(format!("{}:{}", host, port))?;
                listener.set_nonblocking(true)?;
                thread::spawn(move || { tcp_server_loop(listener, tx, event_tx, frame_timeout); })
            }
            (Protocol::Tcp, Role::Client) => {
                let stream = TcpStream::connect_timeout(&format!("{}:{}", host, port).parse().unwrap(), Duration::from_secs(5))?;
                thread::spawn(move || { tcp_client_loop(stream, tx, frame_timeout); })
            }
            (Protocol::Udp, Role::Server) => {
                let socket = UdpSocket::bind(format!("{}:{}", host, port))?;
                socket.set_nonblocking(true)?;
                thread::spawn(move || { udp_server_loop(socket, tx, event_tx, frame_timeout); })
            }
            (Protocol::Udp, Role::Client) => {
                let socket = UdpSocket::bind("0.0.0.0:0")?;
                socket.set_nonblocking(true)?;
                let remote_addr: SocketAddr = format!("{}:{}", host, port).parse().map_err(|_| TransportError::Config("Invalid address".to_string()))?;
                thread::spawn(move || { udp_client_loop(socket, remote_addr, tx, frame_timeout); })
            }
        };

        self.connected = true;
        Ok(TransportHandle { rx, join_handle: Some(join_handle), event_rx: Some(event_rx) })
    }

    fn disconnect(&mut self) -> Result<(), TransportError> { self.connected = false; Ok(()) }
    fn send_bytes(&mut self, _data: &[u8]) -> Result<(), TransportError> { Err(TransportError::Other("Send not yet implemented".to_string())) }
    fn is_connected(&self) -> bool { self.connected }
    fn device_list(&self) -> Vec<DeviceEntry> {
        vec![
            DeviceEntry { id: "TCP_SERVER".to_string(), display: "TCP Server".to_string() },
            DeviceEntry { id: "TCP_CLIENT".to_string(), display: "TCP Client".to_string() },
            DeviceEntry { id: "UDP_SERVER".to_string(), display: "UDP Server".to_string() },
            DeviceEntry { id: "UDP_CLIENT".to_string(), display: "UDP Client".to_string() },
        ]
    }
    fn set_frame_timeout(&mut self, ms: u32) { self.frame_timeout = Duration::from_millis(ms as u64); }
}

fn tcp_server_loop(listener: TcpListener, tx: mpsc::Sender<Vec<u8>>, event_tx: mpsc::Sender<TransportEvent>, _frame_timeout: Duration) {
    let mut clients: HashMap<SocketAddr, TcpStream> = HashMap::new();
    let mut buffer = [0u8; 65536];
    loop {
        match listener.accept() {
            Ok((stream, addr)) => { stream.set_nonblocking(true).ok(); let _ = event_tx.send(TransportEvent::ClientConnected(addr.to_string())); clients.insert(addr, stream); }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        let mut to_remove = Vec::new();
        for (addr, stream) in clients.iter_mut() {
            match stream.read(&mut buffer) {
                Ok(0) => { to_remove.push(*addr); }
                Ok(n) => { let _ = tx.send(buffer[..n].to_vec()); }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => { to_remove.push(*addr); }
            }
        }
        for addr in to_remove { clients.remove(&addr); let _ = event_tx.send(TransportEvent::ClientDisconnected(addr.to_string())); }
        thread::sleep(Duration::from_millis(10));
    }
}

fn tcp_client_loop(mut stream: TcpStream, tx: mpsc::Sender<Vec<u8>>, _frame_timeout: Duration) {
    let mut buffer = [0u8; 65536];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => { let _ = tx.send(buffer[..n].to_vec()); }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => { thread::sleep(Duration::from_millis(10)); }
            Err(_) => break,
        }
    }
}

fn udp_server_loop(socket: UdpSocket, tx: mpsc::Sender<Vec<u8>>, event_tx: mpsc::Sender<TransportEvent>, _frame_timeout: Duration) {
    let mut buffer = [0u8; 65536];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((n, addr)) => { let _ = event_tx.send(TransportEvent::ClientConnected(addr.to_string())); let _ = tx.send(buffer[..n].to_vec()); }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => { thread::sleep(Duration::from_millis(10)); }
            Err(_) => break,
        }
    }
}

fn udp_client_loop(socket: UdpSocket, _remote_addr: SocketAddr, tx: mpsc::Sender<Vec<u8>>, _frame_timeout: Duration) {
    let mut buffer = [0u8; 65536];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((n, _addr)) => { let _ = tx.send(buffer[..n].to_vec()); }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => { thread::sleep(Duration::from_millis(10)); }
            Err(_) => break,
        }
    }
}
