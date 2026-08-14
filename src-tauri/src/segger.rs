use std::{
    env,
    ffi::{CStr, CString, c_char, c_int, c_uint, c_void},
    path::PathBuf,
    ptr,
    sync::{Mutex, OnceLock, TryLockError},
    thread,
    time::{Duration, Instant},
};

use libloading::Library;
use tauri::AppHandle;

use crate::{
    models::{ConnectRequest, DeviceEntry},
    transport::{FrameBuffer, WorkerCommand, emit},
};
use std::sync::mpsc::{Receiver, TryRecvError};

const JLINK_HOST_USB: c_int = 1;
const JLINK_INTERFACE_SWD: c_int = 1;
const RTT_START: c_int = 0;
const RTT_STOP: c_int = 1;
const RTT_GET_NUM_BUFFERS: c_int = 3;
const RTT_DIRECTION_UP: c_int = 0;
const READ_BUFFER_SIZE: usize = 8192;

static JLINK_ACCESS: OnceLock<Mutex<()>> = OnceLock::new();

#[repr(C)]
#[derive(Clone, Copy)]
struct JLinkConnectInfo {
    serial_number: u32,
    connection: u8,
    _alignment: [u8; 3],
    usb_address: u32,
    ip_address: [u8; 16],
    time_ms: i32,
    time_us: u64,
    hardware_version: u32,
    mac_address: [u8; 6],
    product: [c_char; 32],
    nickname: [c_char; 32],
    firmware: [c_char; 112],
    is_dhcp_assigned_ip: c_char,
    is_dhcp_assigned_ip_valid: c_char,
    num_ip_connections: c_char,
    num_ip_connections_valid: c_char,
    padding: [u8; 34],
}

impl Default for JLinkConnectInfo {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

type EmuGetList = unsafe extern "system" fn(c_int, *mut JLinkConnectInfo, c_int) -> c_int;
type EmuSelectByUsbSerial = unsafe extern "system" fn(c_uint) -> c_int;
type OpenEx = unsafe extern "system" fn(*mut c_void, *mut c_void) -> *const c_char;
type Close = unsafe extern "system" fn();
type TifSelect = unsafe extern "system" fn(c_int) -> c_int;
type SetSpeed = unsafe extern "system" fn(c_uint);
type ExecCommand = unsafe extern "system" fn(*const c_char, *mut c_char, c_int) -> c_int;
type Connect = unsafe extern "system" fn() -> c_int;
type SetResetDelay = unsafe extern "system" fn(c_int);
type Reset = unsafe extern "system" fn() -> c_int;
type Go = unsafe extern "system" fn();
type RttControl = unsafe extern "system" fn(c_int, *mut c_void) -> c_int;
type RttRead = unsafe extern "system" fn(c_uint, *mut u8, c_uint) -> c_int;
type RttWrite = unsafe extern "system" fn(c_uint, *const u8, c_uint) -> c_int;

struct JLinkLibrary {
    _library: Library,
    emu_get_list: EmuGetList,
    emu_select_by_usb_serial: EmuSelectByUsbSerial,
    open_ex: OpenEx,
    close: Close,
    tif_select: TifSelect,
    set_speed: SetSpeed,
    exec_command: ExecCommand,
    connect: Connect,
    set_reset_delay: SetResetDelay,
    reset: Reset,
    go: Go,
    rtt_control: RttControl,
    rtt_read: RttRead,
    rtt_write: RttWrite,
}

impl JLinkLibrary {
    fn load() -> Result<Self, String> {
        let mut errors = Vec::new();
        for path in library_candidates() {
            let library = match unsafe { Library::new(&path) } {
                Ok(library) => library,
                Err(error) => {
                    errors.push(format!("{}: {error}", path.display()));
                    continue;
                }
            };
            unsafe {
                macro_rules! symbol {
                    ($name:literal, $kind:ty) => {
                        *library
                            .get::<$kind>(concat!($name, "\0").as_bytes())
                            .map_err(|error| format!("SEGGER J-Link SDK 缺少 {}: {error}", $name))?
                    };
                }
                return Ok(Self {
                    emu_get_list: symbol!("JLINKARM_EMU_GetList", EmuGetList),
                    emu_select_by_usb_serial: symbol!(
                        "JLINKARM_EMU_SelectByUSBSN",
                        EmuSelectByUsbSerial
                    ),
                    open_ex: symbol!("JLINKARM_OpenEx", OpenEx),
                    close: symbol!("JLINKARM_Close", Close),
                    tif_select: symbol!("JLINKARM_TIF_Select", TifSelect),
                    set_speed: symbol!("JLINKARM_SetSpeed", SetSpeed),
                    exec_command: symbol!("JLINKARM_ExecCommand", ExecCommand),
                    connect: symbol!("JLINKARM_Connect", Connect),
                    set_reset_delay: symbol!("JLINKARM_SetResetDelay", SetResetDelay),
                    reset: symbol!("JLINKARM_Reset", Reset),
                    go: symbol!("JLINKARM_Go", Go),
                    rtt_control: symbol!("JLINK_RTTERMINAL_Control", RttControl),
                    rtt_read: symbol!("JLINK_RTTERMINAL_Read", RttRead),
                    rtt_write: symbol!("JLINK_RTTERMINAL_Write", RttWrite),
                    _library: library,
                });
            }
        }
        Err(format!(
            "未找到 SEGGER J-Link SDK，请安装 J-Link Software Pack。尝试路径: {}",
            errors.join("；")
        ))
    }
}

struct JLinkSession {
    api: JLinkLibrary,
    serial_number: u32,
    chip: CString,
    speed: u32,
    opened: bool,
    rtt_started: bool,
}

impl JLinkSession {
    fn new(serial_number: u32, chip: &str, speed: u32) -> Result<Self, String> {
        let chip = CString::new(chip.trim()).map_err(|_| "目标芯片名称包含无效字符")?;
        Ok(Self {
            api: JLinkLibrary::load()?,
            serial_number,
            chip,
            speed,
            opened: false,
            rtt_started: false,
        })
    }

    fn open(&mut self, reset: bool) -> Result<(), String> {
        unsafe {
            (self.api.close)();
            let selected = (self.api.emu_select_by_usb_serial)(self.serial_number);
            if selected < 0 {
                return Err(format!("未找到 J-Link，序列号 {}", self.serial_number));
            }
            let open_error = (self.api.open_ex)(ptr::null_mut(), ptr::null_mut());
            if !open_error.is_null() {
                return Err(format!(
                    "打开 J-Link 失败: {}",
                    CStr::from_ptr(open_error).to_string_lossy()
                ));
            }
            self.opened = true;
            if (self.api.tif_select)(JLINK_INTERFACE_SWD) != 0 {
                return Err("J-Link 不支持或无法选择 SWD 接口".into());
            }
            (self.api.set_speed)(self.speed);
            self.select_device()?;
            let connected = (self.api.connect)();
            if connected < 0 {
                return Err(format!("J-Link 连接目标芯片失败，错误码 {connected}"));
            }
            if reset {
                (self.api.set_reset_delay)(0);
                let result = (self.api.reset)();
                if result < 0 {
                    return Err(format!("J-Link 复位目标失败，错误码 {result}"));
                }
                (self.api.go)();
            }
            self.start_rtt()?;
        }
        Ok(())
    }

    unsafe fn select_device(&self) -> Result<(), String> {
        let command = CString::new(format!("Device = {}", self.chip.to_string_lossy()))
            .map_err(|_| "目标芯片名称包含无效字符")?;
        let mut error_buffer = [0_i8; 1024];
        unsafe {
            (self.api.exec_command)(
                command.as_ptr(),
                error_buffer.as_mut_ptr(),
                error_buffer.len() as c_int,
            );
        }
        if error_buffer[0] != 0 {
            return Err(format!(
                "SEGGER 不支持该目标芯片: {}",
                unsafe { CStr::from_ptr(error_buffer.as_ptr()) }
                    .to_string_lossy()
                    .trim()
            ));
        }
        Ok(())
    }

    fn start_rtt(&mut self) -> Result<(), String> {
        let result = unsafe { (self.api.rtt_control)(RTT_START, ptr::null_mut()) };
        if result < 0 {
            return Err(format!("启动 SEGGER RTT 失败，错误码 {result}"));
        }
        self.rtt_started = true;
        Ok(())
    }

    fn wait_for_rtt(&self, commands: &Receiver<WorkerCommand>) -> Result<bool, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let mut direction = RTT_DIRECTION_UP;
            let result = unsafe {
                (self.api.rtt_control)(
                    RTT_GET_NUM_BUFFERS,
                    (&mut direction as *mut c_int).cast::<c_void>(),
                )
            };
            if result > 0 {
                return Ok(true);
            }
            if matches!(
                commands.try_recv(),
                Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected)
            ) {
                return Ok(false);
            }
            if Instant::now() >= deadline {
                return Err(format!("未找到 RTT 控制块，SEGGER 错误码 {result}"));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, i32> {
        let result = unsafe { (self.api.rtt_read)(0, buffer.as_mut_ptr(), buffer.len() as c_uint) };
        if result < 0 {
            Err(result)
        } else {
            Ok(result as usize)
        }
    }

    fn write(&self, bytes: &[u8]) -> Result<usize, i32> {
        let result = unsafe { (self.api.rtt_write)(0, bytes.as_ptr(), bytes.len() as c_uint) };
        if result < 0 {
            Err(result)
        } else {
            Ok(result as usize)
        }
    }

    fn recover_rtt(&mut self) {
        unsafe {
            if self.rtt_started {
                (self.api.rtt_control)(RTT_STOP, ptr::null_mut());
            }
        }
        self.rtt_started = false;
        thread::sleep(Duration::from_millis(100));
        let _ = self.start_rtt();
    }

    fn reopen(&mut self) -> Result<(), String> {
        unsafe {
            if self.rtt_started {
                (self.api.rtt_control)(RTT_STOP, ptr::null_mut());
            }
            if self.opened {
                (self.api.close)();
            }
        }
        self.rtt_started = false;
        self.opened = false;
        thread::sleep(Duration::from_millis(200));
        self.open(false)
    }
}

impl Drop for JLinkSession {
    fn drop(&mut self) {
        unsafe {
            if self.rtt_started {
                (self.api.rtt_control)(RTT_STOP, ptr::null_mut());
            }
            if self.opened {
                (self.api.close)();
            }
        }
    }
}

pub(crate) fn list_devices() -> Result<Vec<DeviceEntry>, String> {
    let access = JLINK_ACCESS.get_or_init(|| Mutex::new(()));
    let _guard = match access.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::WouldBlock) => return Ok(Vec::new()),
        Err(TryLockError::Poisoned(error)) => error.into_inner(),
    };
    let api = JLinkLibrary::load()?;
    let count = unsafe { (api.emu_get_list)(JLINK_HOST_USB, ptr::null_mut(), 0) };
    if count < 0 {
        return Err(format!("枚举 J-Link 失败，错误码 {count}"));
    }
    let mut entries = vec![JLinkConnectInfo::default(); count as usize];
    let found = unsafe { (api.emu_get_list)(JLINK_HOST_USB, entries.as_mut_ptr(), count) };
    if found < 0 {
        return Err(format!("枚举 J-Link 失败，错误码 {found}"));
    }
    Ok(entries
        .into_iter()
        .take(found as usize)
        .map(|info| {
            let product = unsafe { CStr::from_ptr(info.product.as_ptr()) }
                .to_string_lossy()
                .trim()
                .to_string();
            let product = if product.is_empty() {
                "J-Link".into()
            } else {
                product
            };
            DeviceEntry {
                id: format!("segger:{}", info.serial_number),
                label: format!("{product} (SEGGER, SN={})", info.serial_number),
                transport: "probe".into(),
                probe_kind: Some("SEGGER J-Link SDK".into()),
                serial_number: Some(info.serial_number.to_string()),
            }
        })
        .collect())
}

pub(crate) fn run_rtt(
    app: &AppHandle,
    request: &ConnectRequest,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    if request.probe_chip.trim().is_empty() {
        return Err("连接 J-Link 前必须填写 SEGGER 目标芯片型号".into());
    }
    let serial_number = request
        .device_id
        .strip_prefix("segger:")
        .ok_or("J-Link 标识无效")?
        .parse::<u32>()
        .map_err(|_| "J-Link 序列号无效")?;
    let access = JLINK_ACCESS.get_or_init(|| Mutex::new(()));
    let _guard = access
        .try_lock()
        .map_err(|_| "当前程序已有 J-Link 会话正在使用".to_string())?;
    let mut session = JLinkSession::new(serial_number, &request.probe_chip, request.probe_speed)?;
    session.open(request.probe_reset)?;
    if !session.wait_for_rtt(commands)? {
        return Ok(());
    }
    emit(
        app,
        "connected",
        "system",
        Vec::new(),
        format!("J-Link RTT 已连接，目标芯片 {}", request.probe_chip),
    );

    let mut frame = FrameBuffer::new(request.frame_timeout);
    let mut read_buffer = [0_u8; READ_BUFFER_SIZE];
    let mut next_recovery_notice = Instant::now();
    let mut consecutive_errors = 0_u32;
    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Write(bytes)) => match write_all(&session, &bytes) {
                Ok(()) => emit(app, "data", "sent", bytes, ""),
                Err(error) => {
                    emit(app, "warning", "system", Vec::new(), error);
                    session.recover_rtt();
                }
            },
            Ok(WorkerCommand::ReconfigureSerial(_, response)) => {
                let _ = response.send(Err("当前活动连接不是串口".into()));
            }
            Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }
        match session.read(&mut read_buffer) {
            Ok(size) if size > 0 => {
                consecutive_errors = 0;
                frame.push(&read_buffer[..size]);
            }
            Ok(_) => {
                consecutive_errors = 0;
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => {
                consecutive_errors += 1;
                if Instant::now() >= next_recovery_notice {
                    emit(
                        app,
                        "warning",
                        "system",
                        Vec::new(),
                        format!("RTT 暂时不可用，正在等待目标恢复（SEGGER 错误码 {error}）"),
                    );
                    next_recovery_notice = Instant::now() + Duration::from_secs(2);
                }
                if consecutive_errors >= 10 {
                    if session.reopen().is_ok() {
                        consecutive_errors = 0;
                    }
                } else {
                    session.recover_rtt();
                }
            }
        }
        if frame.should_flush() {
            frame.flush(app);
        }
    }
    frame.flush(app);
    Ok(())
}

fn write_all(session: &JLinkSession, bytes: &[u8]) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut offset = 0;
    while offset < bytes.len() {
        match session.write(&bytes[offset..]) {
            Ok(0) if Instant::now() >= deadline => {
                return Err("J-Link RTT 发送超时，目标缓冲区可能已满".into());
            }
            Ok(0) => thread::sleep(Duration::from_millis(1)),
            Ok(written) => offset += written,
            Err(error) => return Err(format!("J-Link RTT 发送失败，错误码 {error}")),
        }
    }
    Ok(())
}

fn library_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = env::var_os("JLINK_PATH") {
        let path = PathBuf::from(path);
        paths.push(if path.is_dir() {
            path.join(library_name())
        } else {
            path
        });
    }
    #[cfg(target_os = "windows")]
    {
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = env::var_os(variable) {
                paths.push(
                    PathBuf::from(root)
                        .join("SEGGER/JLink")
                        .join(library_name()),
                );
            }
        }
    }
    #[cfg(target_os = "linux")]
    paths.push(PathBuf::from("/opt/SEGGER/JLink").join(library_name()));
    #[cfg(target_os = "macos")]
    paths.push(PathBuf::from("/Applications/SEGGER/JLink").join(library_name()));
    paths.push(PathBuf::from(library_name()));
    paths
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn library_name() -> &'static str {
    "JLink_x64.dll"
}

#[cfg(all(target_os = "windows", target_pointer_width = "32"))]
fn library_name() -> &'static str {
    "JLinkARM.dll"
}

#[cfg(target_os = "linux")]
fn library_name() -> &'static str {
    "libjlinkarm.so"
}

#[cfg(target_os = "macos")]
fn library_name() -> &'static str {
    "libjlinkarm.dylib"
}

#[cfg(test)]
mod tests {
    use super::{JLinkConnectInfo, JLinkLibrary, library_candidates, list_devices};

    #[test]
    fn connect_info_matches_segger_abi() {
        assert_eq!(std::mem::size_of::<JLinkConnectInfo>(), 264);
        assert_eq!(std::mem::align_of::<JLinkConnectInfo>(), 8);
    }

    #[test]
    fn installed_sdk_can_be_loaded() {
        if library_candidates().iter().any(|path| path.is_file()) {
            assert!(JLinkLibrary::load().is_ok());
            assert!(list_devices().is_ok());
        }
    }
}
