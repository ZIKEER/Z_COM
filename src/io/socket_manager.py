import socket
import threading
from PySide6.QtCore import Signal
from src.io.io_transport import IOTransport
from src.io.socket_reader import SocketReaderThread


def get_local_ips():
    """获取本机所有 IPv4 地址"""
    ips = []
    try:
        hostname = socket.gethostname()
        for info in socket.getaddrinfo(hostname, None):
            addr = info[4][0]
            if addr and not addr.startswith('127.') and '.' in addr:
                ips.append(addr)
    except Exception:
        pass
    ips.append('0.0.0.0')
    ips.append('127.0.0.1')
    # deduplicate preserving order
    seen = set()
    return [x for x in ips if not (x in seen or seen.add(x))]


class SocketManager(IOTransport):
    client_event = Signal(str, tuple)  # ('connected'|'disconnected', (host,port))

    def __init__(self):
        super().__init__()
        self.sock = None
        self.server_sock = None
        self.reader_thread = None
        self._mode = None
        self.is_connected = False
        self._lock = threading.Lock()
        self._remote_addr = None

        self.settings = {
            'host': '0.0.0.0',
            'port': 8080,
            'protocol': 'TCP',
            'role': 'Server',
            'frame_timeout': 50,
        }

    def get_available_devices(self):
        return get_local_ips()

    @property
    def current_client(self):
        if self.reader_thread:
            return self.reader_thread.current_client
        return None

    @property
    def mode(self):
        return self._mode

    def update_settings(self, settings):
        self.settings.update(settings)

    def connect(self, host, port, protocol='TCP', role='Client'):
        with self._lock:
            if self.is_connected:
                self._disconnect_internal()

            try:
                frame_timeout = self.settings.get('frame_timeout', 50)
                if protocol == 'TCP' and role == 'Server':
                    self.server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                    self.server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                    self.server_sock.setblocking(False)
                    self.server_sock.bind((host, port))
                    self.server_sock.listen(5)
                    self._mode = 'tcp_server'
                    self.reader_thread = SocketReaderThread(self.server_sock, 'tcp_server', frame_timeout)

                elif protocol == 'TCP' and role == 'Client':
                    self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                    self.sock.settimeout(5)
                    self.sock.connect((host, port))
                    self.sock.setblocking(False)
                    self._mode = 'tcp_client'
                    self.reader_thread = SocketReaderThread(self.sock, 'tcp_client', frame_timeout)

                elif protocol == 'UDP' and role == 'Server':
                    self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                    self.sock.setblocking(False)
                    self.sock.bind((host, port))
                    self._mode = 'udp_server'
                    self.reader_thread = SocketReaderThread(self.sock, 'udp', frame_timeout)

                elif protocol == 'UDP' and role == 'Client':
                    self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                    self.sock.setblocking(False)
                    self._mode = 'udp_client'
                    self._remote_addr = (host, port)
                    self.reader_thread = SocketReaderThread(self.sock, 'udp', frame_timeout)

                else:
                    raise ValueError(f"Unknown mode: {protocol}/{role}")

                self.reader_thread.data_received.connect(self.data_received)
                self.reader_thread.error_occurred.connect(self._on_thread_error)
                self.reader_thread.finished.connect(self._on_thread_finished)
                self.reader_thread.client_event.connect(self.client_event)
                self.reader_thread.start()

                self.is_connected = True
                self.connection_changed.emit(True)
                return True

            except Exception as e:
                self._disconnect_internal()
                self.error_occurred.emit(f"Socket 连接失败: {str(e)}")
                return False

    def disconnect(self):
        with self._lock:
            return self._disconnect_internal()

    def _disconnect_internal(self):
        try:
            if self.reader_thread and self.reader_thread.isRunning():
                try:
                    self.reader_thread.data_received.disconnect()
                    self.reader_thread.error_occurred.disconnect()
                    self.reader_thread.finished.disconnect()
                    self.reader_thread.client_event.disconnect()
                except (TypeError, RuntimeError):
                    pass
                self.reader_thread.stop()
                self.reader_thread.wait(2000)

            if self.reader_thread:
                self.reader_thread = None

            if self.server_sock:
                try:
                    self.server_sock.close()
                except Exception:
                    pass
                self.server_sock = None

            if self.sock:
                try:
                    self.sock.close()
                except Exception:
                    pass
                self.sock = None

            self._remote_addr = None
            self._mode = None
            self.is_connected = False
            self.connection_changed.emit(False)
            return True
        except Exception as e:
            self.error_occurred.emit(f"断开失败: {str(e)}")
            return False

    def _on_thread_error(self, error_msg):
        self.error_occurred.emit(error_msg)
        self.disconnect()

    def _on_thread_finished(self):
        if self.is_connected:
            self.disconnect()

    def send_data(self, data, is_hex=False):
        if not self.is_connected:
            self.error_occurred.emit("Socket 未连接")
            return False

        try:
            if is_hex:
                if isinstance(data, str):
                    hex_str = data.replace(' ', '').replace('\n', '')
                    bytes_data = bytes.fromhex(hex_str)
                else:
                    bytes_data = data
            else:
                bytes_data = data.encode('utf-8') if isinstance(data, str) else data

            if self._mode == 'tcp_client':
                self.sock.send(bytes_data)
            elif self._mode == 'tcp_server':
                if not self.reader_thread or not self.reader_thread.send_to_current(bytes_data):
                    self.error_occurred.emit("没有已连接的客户端")
                    return False
            elif self._mode in ('udp_server', 'udp_client'):
                if self._mode == 'udp_client':
                    self.sock.sendto(bytes_data, self._remote_addr)
                else:
                    if self.reader_thread and self.reader_thread.current_client:
                        self.sock.sendto(bytes_data, self.reader_thread.current_client)
                    else:
                        self.error_occurred.emit("没有客户端地址")
                        return False
            return True
        except Exception as e:
            self.error_occurred.emit(f"Socket 发送失败: {str(e)}")
            return False
