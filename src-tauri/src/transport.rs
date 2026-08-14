use std::{
    fs,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use probe_rs::{
    Permissions,
    config::Registry,
    probe::{DebugProbeSelector, WireProtocol, list::Lister},
    rtt::Rtt,
};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use tauri::{AppHandle, Emitter, Manager};

use crate::models::{ConnectRequest, SerialSettings, TransportEvent};

const EVENT_NAME: &str = "transport-event";
const READ_BUFFER_SIZE: usize = 8192;
const FRAME_EMIT_THRESHOLD: usize = 4096;

pub(crate) enum WorkerCommand {
    Write(Vec<u8>),
    ReconfigureSerial(SerialSettings, Sender<Result<(), String>>),
    Stop,
}

struct Worker {
    commands: Sender<WorkerCommand>,
    handle: JoinHandle<()>,
    is_serial: bool,
}

#[derive(Default)]
pub struct TransportManager {
    worker: Option<Worker>,
}

impl TransportManager {
    pub fn connect(&mut self, app: AppHandle, request: ConnectRequest) {
        self.disconnect();
        let is_serial = request.transport == "serial";
        let (commands, receiver) = mpsc::channel();
        let handle = thread::spawn(move || {
            let result = match request.transport.as_str() {
                "serial" => run_serial(&app, &request, &receiver),
                "socket" => run_socket(&app, &request, &receiver),
                "probe" if request.device_id.starts_with("segger:") => {
                    crate::segger::run_rtt(&app, &request, &receiver)
                }
                "probe" => run_probe_rtt(&app, &request, &receiver),
                other => Err(format!("不支持的传输类型: {other}")),
            };
            if let Err(message) = result {
                emit(&app, "error", "system", Vec::new(), message);
            }
            emit(&app, "disconnected", "system", Vec::new(), "连接已关闭");
        });
        self.worker = Some(Worker {
            commands,
            handle,
            is_serial,
        });
    }

    pub fn send(&self, bytes: Vec<u8>) -> Result<(), String> {
        let worker = self.worker.as_ref().ok_or("当前没有活动连接")?;
        worker
            .commands
            .send(WorkerCommand::Write(bytes))
            .map_err(|_| "连接线程已退出".to_string())
    }

    pub fn reconfigure_serial(&self, settings: SerialSettings) -> Result<(), String> {
        let worker = self.worker.as_ref().ok_or("当前没有活动连接")?;
        if !worker.is_serial {
            return Err("当前活动连接不是串口".into());
        }
        let (response_sender, response_receiver) = mpsc::channel();
        worker
            .commands
            .send(WorkerCommand::ReconfigureSerial(settings, response_sender))
            .map_err(|_| "串口连接线程已退出".to_string())?;
        response_receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "串口参数更新超时".to_string())?
    }

    pub fn disconnect(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.commands.send(WorkerCommand::Stop);
            let _ = worker.handle.join();
        }
    }
}

impl Drop for TransportManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}

pub(crate) struct FrameBuffer {
    bytes: Vec<u8>,
    started: Option<Instant>,
    last_received: Option<Instant>,
    timeout: Duration,
}

impl FrameBuffer {
    pub(crate) fn new(timeout_ms: u64) -> Self {
        Self {
            bytes: Vec::new(),
            started: None,
            last_received: None,
            timeout: Duration::from_millis(timeout_ms.max(1)),
        }
    }

    pub(crate) fn push(&mut self, data: &[u8]) {
        let now = Instant::now();
        self.started.get_or_insert(now);
        self.last_received = Some(now);
        self.bytes.extend_from_slice(data);
    }

    fn set_timeout(&mut self, timeout_ms: u64) {
        self.timeout = Duration::from_millis(timeout_ms.max(1));
    }

    pub(crate) fn should_flush(&self) -> bool {
        if self.bytes.is_empty() {
            return false;
        }
        let now = Instant::now();
        self.bytes.len() >= FRAME_EMIT_THRESHOLD
            || self
                .started
                .is_some_and(|value| now.duration_since(value) >= self.timeout)
            || self
                .last_received
                .is_some_and(|value| now.duration_since(value) >= self.timeout)
    }

    pub(crate) fn flush(&mut self, app: &AppHandle) {
        if self.bytes.is_empty() {
            return;
        }
        emit(app, "data", "received", std::mem::take(&mut self.bytes), "");
        self.started = None;
        self.last_received = None;
    }
}

fn run_serial(
    app: &AppHandle,
    request: &ConnectRequest,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    let mut port = serialport::new(&request.device_id, request.baud_rate)
        .data_bits(parse_data_bits(request.data_bits)?)
        .stop_bits(parse_stop_bits(request.stop_bits)?)
        .parity(parse_parity(&request.parity)?)
        .flow_control(parse_flow_control(&request.flow_control)?)
        .timeout(Duration::from_millis(10))
        .open()
        .map_err(|error| format!("打开串口失败: {error}"))?;

    emit(
        app,
        "connected",
        "system",
        Vec::new(),
        format!("串口 {} 已连接", request.device_id),
    );
    let mut frame = FrameBuffer::new(request.frame_timeout);
    let mut read_buffer = [0_u8; READ_BUFFER_SIZE];
    loop {
        if handle_serial_commands(app, commands, port.as_mut(), &mut frame)? {
            break;
        }
        match port.read(&mut read_buffer) {
            Ok(size) if size > 0 => frame.push(&read_buffer[..size]),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => return Err(format!("串口读取失败: {error}")),
        }
        if frame.should_flush() {
            frame.flush(app);
        }
    }
    frame.flush(app);
    Ok(())
}

fn handle_serial_commands(
    app: &AppHandle,
    commands: &Receiver<WorkerCommand>,
    port: &mut dyn SerialPort,
    frame: &mut FrameBuffer,
) -> Result<bool, String> {
    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Write(bytes)) => {
                port.write_all(&bytes)
                    .map_err(|error| format!("串口发送失败: {error}"))?;
                emit(app, "data", "sent", bytes, "");
            }
            Ok(WorkerCommand::ReconfigureSerial(settings, response)) => {
                let result = apply_serial_settings(port, &settings);
                match &result {
                    Ok(()) => {
                        frame.set_timeout(settings.frame_timeout);
                        emit(
                            app,
                            "info",
                            "system",
                            Vec::new(),
                            format!(
                                "串口参数已更新：{} baud，{} 数据位，{} 停止位，{}，{}",
                                settings.baud_rate,
                                settings.data_bits,
                                settings.stop_bits,
                                settings.parity,
                                settings.flow_control
                            ),
                        );
                    }
                    Err(error) => emit(
                        app,
                        "warning",
                        "system",
                        Vec::new(),
                        format!("串口参数更新失败，已恢复原值：{error}"),
                    ),
                }
                let _ = response.send(result);
            }
            Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => return Ok(true),
            Err(TryRecvError::Empty) => return Ok(false),
        }
    }
}

fn apply_serial_settings(
    port: &mut dyn SerialPort,
    settings: &SerialSettings,
) -> Result<(), String> {
    let data_bits = parse_data_bits(settings.data_bits)?;
    let stop_bits = parse_stop_bits(settings.stop_bits)?;
    let parity = parse_parity(&settings.parity)?;
    let flow_control = parse_flow_control(&settings.flow_control)?;
    let old_baud_rate = port.baud_rate().map_err(|error| error.to_string())?;
    let old_data_bits = port.data_bits().map_err(|error| error.to_string())?;
    let old_stop_bits = port.stop_bits().map_err(|error| error.to_string())?;
    let old_parity = port.parity().map_err(|error| error.to_string())?;
    let old_flow_control = port.flow_control().map_err(|error| error.to_string())?;
    let result = port
        .set_baud_rate(settings.baud_rate)
        .and_then(|_| port.set_data_bits(data_bits))
        .and_then(|_| port.set_stop_bits(stop_bits))
        .and_then(|_| port.set_parity(parity))
        .and_then(|_| port.set_flow_control(flow_control));
    if let Err(error) = result {
        let _ = port.set_baud_rate(old_baud_rate);
        let _ = port.set_data_bits(old_data_bits);
        let _ = port.set_stop_bits(old_stop_bits);
        let _ = port.set_parity(old_parity);
        let _ = port.set_flow_control(old_flow_control);
        return Err(error.to_string());
    }
    Ok(())
}

fn run_socket(
    app: &AppHandle,
    request: &ConnectRequest,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    match (
        request.socket_protocol.as_str(),
        request.socket_role.as_str(),
    ) {
        ("TCP", "Server") => run_tcp_server(app, request, commands),
        ("TCP", "Client") => run_tcp_client(app, request, commands),
        ("UDP", "Server") => run_udp(app, request, commands, true),
        ("UDP", "Client") => run_udp(app, request, commands, false),
        _ => Err("Socket 协议或角色无效".into()),
    }
}

fn run_tcp_client(
    app: &AppHandle,
    request: &ConnectRequest,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    let address = resolve_address(&request.socket_host, request.socket_port)?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .map_err(|error| format!("TCP 连接失败: {error}"))?;
    stream
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    emit(
        app,
        "connected",
        "system",
        Vec::new(),
        format!("TCP 客户端已连接 {address}"),
    );
    run_tcp_stream(app, commands, &mut stream, request.frame_timeout)
}

fn run_tcp_stream(
    app: &AppHandle,
    commands: &Receiver<WorkerCommand>,
    stream: &mut TcpStream,
    frame_timeout: u64,
) -> Result<(), String> {
    let mut frame = FrameBuffer::new(frame_timeout);
    let mut read_buffer = [0_u8; READ_BUFFER_SIZE];
    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Write(bytes)) => {
                write_tcp(stream, &bytes)?;
                emit(app, "data", "sent", bytes, "");
            }
            Ok(WorkerCommand::ReconfigureSerial(_, response)) => {
                let _ = response.send(Err("当前活动连接不是串口".into()));
            }
            Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }
        match stream.read(&mut read_buffer) {
            Ok(0) => return Err("TCP 对端已关闭连接".into()),
            Ok(size) => frame.push(&read_buffer[..size]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) => return Err(format!("TCP 读取失败: {error}")),
        }
        if frame.should_flush() {
            frame.flush(app);
        }
    }
    frame.flush(app);
    Ok(())
}

fn run_tcp_server(
    app: &AppHandle,
    request: &ConnectRequest,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    let listener = TcpListener::bind((&*request.socket_host, request.socket_port))
        .map_err(|error| format!("TCP 监听失败: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    emit(
        app,
        "connected",
        "system",
        Vec::new(),
        format!(
            "TCP 服务端正在监听 {}:{}",
            request.socket_host, request.socket_port
        ),
    );
    let mut client: Option<(TcpStream, SocketAddr)> = None;
    let mut frame = FrameBuffer::new(request.frame_timeout);
    let mut read_buffer = [0_u8; READ_BUFFER_SIZE];
    loop {
        match listener.accept() {
            Ok((stream, address)) => {
                stream
                    .set_nonblocking(true)
                    .map_err(|error| error.to_string())?;
                let replaced_address = client.as_ref().map(|(_, address)| *address);
                if replaced_address.is_some() {
                    frame.flush(app);
                    if let Some((old_stream, _)) = client.as_ref() {
                        let _ = old_stream.shutdown(Shutdown::Both);
                    }
                }
                client = Some((stream, address));
                emit(
                    app,
                    "peer",
                    "system",
                    Vec::new(),
                    if let Some(old_address) = replaced_address {
                        format!("新客户端 {address} 已连接并顶替旧客户端 {old_address}")
                    } else {
                        format!("客户端已连接: {address}")
                    },
                );
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(format!("TCP 接收连接失败: {error}")),
        }

        match commands.try_recv() {
            Ok(WorkerCommand::Write(bytes)) => {
                if let Some((stream, _)) = client.as_mut() {
                    write_tcp(stream, &bytes)?;
                    emit(app, "data", "sent", bytes, "");
                } else {
                    emit(
                        app,
                        "warning",
                        "system",
                        Vec::new(),
                        "当前没有已连接的 TCP 客户端",
                    );
                }
            }
            Ok(WorkerCommand::ReconfigureSerial(_, response)) => {
                let _ = response.send(Err("当前活动连接不是串口".into()));
            }
            Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }

        if let Some((stream, address)) = client.as_mut() {
            match stream.read(&mut read_buffer) {
                Ok(0) => {
                    emit(
                        app,
                        "peer",
                        "system",
                        Vec::new(),
                        format!("客户端已断开: {address}"),
                    );
                    client = None;
                }
                Ok(size) => frame.push(&read_buffer[..size]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(format!("TCP 读取失败: {error}")),
            }
        }
        if frame.should_flush() {
            frame.flush(app);
        }
        thread::sleep(Duration::from_millis(2));
    }
    frame.flush(app);
    Ok(())
}

fn run_udp(
    app: &AppHandle,
    request: &ConnectRequest,
    commands: &Receiver<WorkerCommand>,
    is_server: bool,
) -> Result<(), String> {
    let socket = if is_server {
        UdpSocket::bind((&*request.socket_host, request.socket_port))
    } else {
        UdpSocket::bind("0.0.0.0:0")
    }
    .map_err(|error| format!("UDP 打开失败: {error}"))?;
    socket
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let configured_peer = if is_server {
        None
    } else {
        Some(resolve_address(&request.socket_host, request.socket_port)?)
    };
    let mut current_peer = configured_peer;
    emit(
        app,
        "connected",
        "system",
        Vec::new(),
        if is_server {
            format!(
                "UDP 服务端正在监听 {}:{}",
                request.socket_host, request.socket_port
            )
        } else {
            format!(
                "UDP 客户端已就绪 {}:{}",
                request.socket_host, request.socket_port
            )
        },
    );
    let mut frame = FrameBuffer::new(request.frame_timeout);
    let mut read_buffer = [0_u8; 65_535];
    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Write(bytes)) => {
                if let Some(peer) = current_peer {
                    socket
                        .send_to(&bytes, peer)
                        .map_err(|error| format!("UDP 发送失败: {error}"))?;
                    emit(app, "data", "sent", bytes, "");
                } else {
                    emit(
                        app,
                        "warning",
                        "system",
                        Vec::new(),
                        "尚未收到 UDP 客户端数据，无法确定发送地址",
                    );
                }
            }
            Ok(WorkerCommand::ReconfigureSerial(_, response)) => {
                let _ = response.send(Err("当前活动连接不是串口".into()));
            }
            Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }
        match socket.recv_from(&mut read_buffer) {
            Ok((size, peer)) => {
                current_peer = Some(peer);
                frame.push(&read_buffer[..size]);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) => return Err(format!("UDP 读取失败: {error}")),
        }
        if frame.should_flush() {
            frame.flush(app);
        }
    }
    frame.flush(app);
    Ok(())
}

fn run_probe_rtt(
    app: &AppHandle,
    request: &ConnectRequest,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    if request.probe_chip.trim().is_empty() {
        return Err("连接调试探针前必须填写目标芯片型号".into());
    }
    let selector: DebugProbeSelector = request
        .device_id
        .parse()
        .map_err(|error| format!("调试探针标识无效: {error}"))?;
    let lister = Lister::new();
    let mut probe = lister.open(selector).map_err(probe_driver_error)?;
    probe
        .select_protocol(WireProtocol::Swd)
        .map_err(probe_driver_error)?;
    probe
        .set_speed(request.probe_speed)
        .map_err(probe_driver_error)?;
    let target_directory = app
        .try_state::<crate::AppState>()
        .ok_or("无法读取应用状态")?
        .config_directory
        .join("probe_rs_targets");
    let (registry, custom_targets) = load_probe_registry(&target_directory)?;
    let mut session = probe
        .attach_with_registry(
            request.probe_chip.as_str(),
            Permissions::default(),
            &registry,
        )
        .map_err(|error| format!("连接目标芯片失败: {error}"))?;

    if request.probe_reset {
        session
            .core(0)
            .and_then(|mut core| core.reset())
            .map_err(|error| format!("目标芯片复位失败: {error}"))?;
        thread::sleep(Duration::from_millis(100));
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut rtt = loop {
        let result = session
            .core(0)
            .map_err(|error| error.to_string())
            .and_then(|mut core| Rtt::attach(&mut core).map_err(|error| error.to_string()));
        match result {
            Ok(rtt) => break rtt,
            Err(error) if Instant::now() < deadline => {
                if matches!(
                    commands.try_recv(),
                    Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected)
                ) {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(100));
                let _ = error;
            }
            Err(error) => return Err(format!("未找到 RTT 控制块: {error}")),
        }
    };
    if rtt.up_channel(0).is_none() {
        return Err("目标 RTT 没有可用的上行通道 0".into());
    }
    emit(
        app,
        "connected",
        "system",
        Vec::new(),
        if custom_targets
            .iter()
            .any(|target| target.eq_ignore_ascii_case(&request.probe_chip))
        {
            format!("调试探针 RTT 已连接，自定义目标 {}", request.probe_chip)
        } else {
            format!("调试探针 RTT 已连接，目标芯片 {}", request.probe_chip)
        },
    );

    let mut frame = FrameBuffer::new(request.frame_timeout);
    let mut read_buffer = [0_u8; READ_BUFFER_SIZE];
    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Write(bytes)) => {
                write_rtt(&mut session, &mut rtt, &bytes)?;
                emit(app, "data", "sent", bytes, "");
            }
            Ok(WorkerCommand::ReconfigureSerial(_, response)) => {
                let _ = response.send(Err("当前活动连接不是串口".into()));
            }
            Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }
        let size = {
            let mut core = session
                .core(0)
                .map_err(|error| format!("访问目标内核失败: {error}"))?;
            rtt.up_channel(0)
                .ok_or("RTT 上行通道 0 已失效")?
                .read(&mut core, &mut read_buffer)
                .map_err(|error| format!("RTT 读取失败: {error}"))?
        };
        if size > 0 {
            frame.push(&read_buffer[..size]);
        } else {
            thread::sleep(Duration::from_millis(2));
        }
        if frame.should_flush() {
            frame.flush(app);
        }
    }
    frame.flush(app);
    Ok(())
}

fn write_rtt(session: &mut probe_rs::Session, rtt: &mut Rtt, bytes: &[u8]) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut offset = 0;
    while offset < bytes.len() {
        let written = {
            let mut core = session.core(0).map_err(|error| error.to_string())?;
            rtt.down_channel(0)
                .ok_or("目标 RTT 没有可用的下行通道 0")?
                .write(&mut core, &bytes[offset..])
                .map_err(|error| format!("RTT 发送失败: {error}"))?
        };
        if written == 0 {
            if Instant::now() >= deadline {
                return Err("RTT 发送超时，目标缓冲区可能已满".into());
            }
            thread::sleep(Duration::from_millis(1));
        } else {
            offset += written;
        }
    }
    Ok(())
}

pub(crate) fn list_custom_probe_targets(directory: &Path) -> Result<Vec<String>, String> {
    load_probe_registry(directory).map(|(_, targets)| targets)
}

fn load_probe_registry(directory: &Path) -> Result<(Registry, Vec<String>), String> {
    let mut registry = Registry::from_builtin_families();
    let mut paths = yaml_files(directory)?;
    paths.sort();

    let mut custom_families = Vec::new();
    for path in paths {
        let yaml = fs::read_to_string(&path)
            .map_err(|error| format!("读取自定义 MCU 描述 {} 失败: {error}", path.display()))?;
        let family_name = registry
            .add_target_family_from_yaml(&yaml)
            .map_err(|error| format!("加载自定义 MCU 描述 {} 失败: {error}", path.display()))?;
        if !custom_families
            .iter()
            .any(|name: &String| name.eq_ignore_ascii_case(&family_name))
        {
            custom_families.push(family_name);
        }
    }
    let mut custom_targets = registry
        .families()
        .iter()
        .filter(|family| {
            custom_families
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&family.name))
        })
        .flat_map(|family| family.variants().iter().map(|target| target.name.clone()))
        .collect::<Vec<_>>();
    custom_targets.sort_by_key(|name| name.to_ascii_lowercase());
    custom_targets.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    Ok((registry, custom_targets))
}

fn yaml_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(directory)
        .map_err(|error| format!("读取自定义 MCU 目录 {} 失败: {error}", directory.display()))?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let path = entry.path();
                let is_yaml = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        extension.eq_ignore_ascii_case("yaml")
                            || extension.eq_ignore_ascii_case("yml")
                    });
                is_yaml.then_some(Ok(path))
            }
            Err(error) => Some(Err(format!("读取自定义 MCU 目录项失败: {error}"))),
        })
        .collect()
}

fn probe_driver_error(error: probe_rs::probe::DebugProbeError) -> String {
    format!(
        "调试探针访问失败: {error}。Windows 下部分探针需要 WinUSB 驱动；切换驱动前请评估与厂商工具的兼容性"
    )
}

fn resolve_address(host: &str, port: u16) -> Result<SocketAddr, String> {
    (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("无法解析地址 {host}:{port}: {error}"))?
        .next()
        .ok_or_else(|| format!("地址 {host}:{port} 没有可用结果"))
}

fn write_tcp(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut offset = 0;
    while offset < bytes.len() {
        match stream.write(&bytes[offset..]) {
            Ok(0) => return Err("TCP 连接已关闭".into()),
            Ok(size) => offset += size,
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(format!("TCP 发送失败: {error}")),
        }
    }
    Ok(())
}

fn parse_data_bits(value: u8) -> Result<DataBits, String> {
    match value {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        _ => Err(format!("不支持的数据位: {value}")),
    }
}

fn parse_stop_bits(value: f32) -> Result<StopBits, String> {
    if (value - 1.0).abs() < f32::EPSILON {
        Ok(StopBits::One)
    } else if (value - 2.0).abs() < f32::EPSILON {
        Ok(StopBits::Two)
    } else {
        Err(format!("当前串口后端不支持停止位: {value}"))
    }
}

fn parse_parity(value: &str) -> Result<Parity, String> {
    match value.to_ascii_lowercase().as_str() {
        "none" | "无" => Ok(Parity::None),
        "odd" | "奇校验" => Ok(Parity::Odd),
        "even" | "偶校验" => Ok(Parity::Even),
        _ => Err(format!("不支持的校验方式: {value}")),
    }
}

fn parse_flow_control(value: &str) -> Result<FlowControl, String> {
    match value.to_ascii_lowercase().as_str() {
        "none" | "无" => Ok(FlowControl::None),
        "software" | "xon/xoff" => Ok(FlowControl::Software),
        "hardware" | "rts/cts" => Ok(FlowControl::Hardware),
        _ => Err(format!("不支持的流控方式: {value}")),
    }
}

pub(crate) fn emit(
    app: &AppHandle,
    kind: &str,
    direction: &str,
    bytes: Vec<u8>,
    message: impl Into<String>,
) {
    let message = message.into();
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(mut logger) = state.logger.lock() {
            if kind == "data" {
                logger.log_data(direction, &bytes);
            } else if !message.is_empty() {
                logger.log_event(&message);
            }
        }
    }
    let _ = app.emit(
        EVENT_NAME,
        TransportEvent {
            kind: kind.into(),
            direction: direction.into(),
            bytes,
            message,
        },
    );
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::list_custom_probe_targets;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "z-com-probe-target-test-{}-{suffix}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn loads_custom_rtt_target_from_yaml() {
        let directory = TestDirectory::new();
        fs::write(
            directory.0.join("mychip.yaml"),
            r#"
name: MyChip Series
generated_from_pack: false
variants:
- name: MYCHIP123
  cores:
  - name: main
    type: armv7em
    core_access_options: !Arm
      ap: !v1 0
  memory_map:
  - !Ram
    name: RAM
    range:
      start: 0x20000000
      end: 0x20020000
    cores:
    - main
  flash_algorithms: []
flash_algorithms: []
"#,
        )
        .unwrap();

        let targets = list_custom_probe_targets(&directory.0).unwrap();

        assert_eq!(targets, vec!["MYCHIP123"]);
    }
}
