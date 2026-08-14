import threading
from PySide6.QtCore import QObject, Signal


class IOTransport(QObject):
    data_received = Signal(bytes)
    connection_changed = Signal(bool)
    error_occurred = Signal(str)

    def __init__(self):
        super().__init__()
        self.is_connected = False
        self.reader_thread = None
        self._lock = threading.Lock()
        self.settings = {}

    # ── 子类必须实现 ──

    def get_available_devices(self):
        raise NotImplementedError

    def _connect_impl(self, **kwargs):
        """子类实现具体连接逻辑，成功返回 True，失败抛异常或返回 False。"""
        raise NotImplementedError

    def _close_resource(self):
        """子类实现底层资源关闭（串口.close / socket.close / jlink.close 等）。"""
        raise NotImplementedError

    def _send_bytes(self, data: bytes) -> bool:
        """子类实现底层字节发送，返回是否成功。"""
        raise NotImplementedError

    # ── 通用生命周期管理 ──

    def open_connection(self, *args, **kwargs):
        with self._lock:
            if self.is_connected:
                self._disconnect_internal()
            try:
                result = self._connect_impl(*args, **kwargs)
                if result is not False:
                    self.is_connected = True
                    self.connection_changed.emit(True)
                return result if result is not None else True
            except Exception as e:
                self._disconnect_internal()
                self.error_occurred.emit(f"连接失败: {str(e)}")
                return False

    def close_connection(self):
        with self._lock:
            return self._disconnect_internal()

    def _disconnect_internal(self):
        try:
            self._stop_reader_thread()
            self._close_resource()
            was_connected = self.is_connected
            self.is_connected = False
            if was_connected:
                self.connection_changed.emit(False)
            return True
        except Exception as e:
            self.error_occurred.emit(f"断开失败: {str(e)}")
            return False

    def _stop_reader_thread(self):
        if self.reader_thread:
            try:
                self.reader_thread.data_received.disconnect()
                self.reader_thread.error_occurred.disconnect()
                self.reader_thread.finished.disconnect()
            except (TypeError, RuntimeError):
                pass
            if self.reader_thread.isRunning():
                self.reader_thread.stop()
            # stop() 返回时线程已退出；这里保留显式 wait 作为生命周期屏障，
            # 防止 QThread 在退出阶段仍被 Qt 销毁。
            self.reader_thread.wait()
            self.reader_thread = None

    def _connect_reader_thread(self, thread):
        """连接 reader 线程信号并启动。"""
        self.reader_thread = thread
        self.reader_thread.data_received.connect(self.data_received)
        self.reader_thread.error_occurred.connect(self._on_thread_error)
        self.reader_thread.finished.connect(self._on_thread_finished)
        self.reader_thread.start()

    def _on_thread_error(self, error_msg):
        self.error_occurred.emit(error_msg)
        self.close_connection()

    def _on_thread_finished(self):
        if self.is_connected:
            self.close_connection()

    # ── 通用配置 ──

    def update_settings(self, settings):
        self.settings.update(settings)
        if 'frame_timeout' in settings and self.reader_thread:
            self.reader_thread.set_frame_timeout(settings['frame_timeout'])

    # ── 通用发送 ──

    def send_data(self, data, is_hex=False):
        if not self.is_connected:
            self.error_occurred.emit("未连接")
            return False
        try:
            bytes_data = self._parse_send_data(data, is_hex)
            return self._send_bytes(bytes_data)
        except Exception as e:
            self.error_occurred.emit(f"发送失败: {str(e)}")
            return False

    @staticmethod
    def _parse_send_data(data, is_hex=False):
        """将用户输入统一转为 bytes。"""
        if is_hex:
            hex_str = data.replace(' ', '').replace('\n', '') if isinstance(data, str) else data.hex()
            return bytes.fromhex(hex_str)
        return data.encode('utf-8') if isinstance(data, str) else data
