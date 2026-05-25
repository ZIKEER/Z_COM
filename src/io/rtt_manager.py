from src.io.io_transport import IOTransport
from src.io.jlink_backend import JLinkRttBackend
from src.io.rtt_reader import RttReaderThread
from src.io.daplink_backend import DapLinkBackend

PREFIX_JLINK = 'JLINK:SN='
PREFIX_DAPLINK = 'DAPLINK:ID='


class RttManager(IOTransport):
    """RTT 管理类 — 后端委派模式

    根据端口前缀自动选择 J-Link 或 DAPLink 后端
    """

    def __init__(self):
        super().__init__()
        self.backends = {
            'jlink': JLinkRttBackend(),
            'daplink': DapLinkBackend(),
        }
        self.jlink_backend = self.backends['jlink']
        self.daplink_backend = self.backends['daplink']
        self._active_backend = None
        self.is_connected = False
        self.reader_thread = None
        self.settings = {'speed': 4000, 'frame_timeout': 50}

    def _import_pylink(self):
        return self.jlink_backend._import_pylink()

    def update_settings(self, settings):
        self.settings.update(settings)
        for backend in self.backends.values():
            backend.update_settings(settings)

    @property
    def _backend(self):
        return self._active_backend

    def get_available_devices(self):
        devices = []
        try:
            devices.extend(self.backends['jlink'].get_available_devices())
        except Exception as e:
            print(f"[RTT] J-Link 扫描异常: {e}")

        try:
            dap_devices = DapLinkBackend.get_dap_devices()
            for uid, desc in dap_devices:
                devices.append((uid, desc))
        except Exception as e:
            print(f"[RTT] DAP-Link 扫描异常: {e}")

        return devices

    def connect(self, port_key=None, chip=None, speed=None, reset_flag=None,
                start_address=None, range_size=None):
        if self.is_connected:
            self._disconnect_internal()

        backend = self._resolve_backend(port_key)
        try:
            connect_kwargs = {
                'chip': chip,
                'speed': speed if speed is not None else self.settings.get('speed'),
                'reset_flag': reset_flag,
                'start_address': start_address,
                'range_size': range_size,
            }
            if port_key and port_key.startswith(PREFIX_DAPLINK):
                connect_kwargs['device_id'] = port_key.replace(PREFIX_DAPLINK, '')
            elif port_key and port_key.startswith(PREFIX_JLINK):
                connect_kwargs['serial_no'] = port_key.replace(PREFIX_JLINK, '')

            backend.connect(**connect_kwargs)
            self._active_backend = backend
            self.is_connected = True
            self._start_reader()
            self.connection_changed.emit(True)
            return True
        except Exception:
            self._disconnect_internal(emit_signal=False)
            raise

    def disconnect(self):
        self._disconnect_internal()

    def _disconnect_internal(self, emit_signal=True):
        try:
            if self.reader_thread and self.reader_thread.isRunning():
                self.reader_thread.stop()
                self.reader_thread = None

            if self._backend:
                self._backend.disconnect()
        except Exception:
            pass

        self._active_backend = None
        self.is_connected = False
        if emit_signal:
            self.connection_changed.emit(False)

    def _resolve_backend(self, port_key):
        if port_key and port_key.startswith(PREFIX_DAPLINK):
            return self.backends['daplink']
        return self.backends['jlink']

    def _start_reader(self):
        frame_timeout = self.settings.get('frame_timeout', 50) / 1000.0
        self.reader_thread = RttReaderThread(
            self._active_backend,
            buffer_idx=0,
            read_size=8192,
            read_interval=0.002,
            frame_timeout=frame_timeout,
        )
        self.reader_thread.data_received.connect(self.data_received)
        self.reader_thread.error_occurred.connect(self._on_thread_error)
        self.reader_thread.finished.connect(self._on_thread_finished)
        self.reader_thread.start()

    def _on_thread_error(self, error_msg):
        self.error_occurred.emit(error_msg)

    def _on_thread_finished(self):
        if self.is_connected:
            self.disconnect()

    def send_data(self, data, is_hex=False):
        if not self.is_connected or not self._backend:
            self.error_occurred.emit("RTT 未连接")
            return False

        try:
            if is_hex:
                if isinstance(data, str):
                    hex_str = data.replace(' ', '').replace('\n', '')
                    bytes_data = bytes.fromhex(hex_str)
                else:
                    bytes_data = data
            else:
                if isinstance(data, str):
                    bytes_data = data.encode('utf-8')
                else:
                    bytes_data = data

            self._backend.rtt_write(0, list(bytes_data))
            return True
        except Exception as e:
            self.error_occurred.emit(f"RTT 发送失败: {str(e)}")
            return False

    def get_serial_number(self):
        if self._backend:
            return self._backend.get_serial_number()
        return 0
