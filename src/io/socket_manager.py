import socket
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
    seen = set()
    return [x for x in ips if not (x in seen or seen.add(x))]


class SocketManager(IOTransport):
    client_event = Signal(str, tuple)

    def __init__(self):
        super().__init__()
        self.sock = None
        self.server_sock = None
        self._mode = None
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

    def open_connection(self, host, port, protocol='TCP', role='Client'):
        return super().open_connection(host, port, protocol=protocol, role=role)

    def _connect_impl(self, host, port, protocol='TCP', role='Client'):
        self._open_socket(host, port, protocol, role)
        frame_timeout = self.settings.get('frame_timeout', 50)
        thread = SocketReaderThread(
            self.server_sock if self._mode == 'tcp_server' else self.sock,
            self._mode, frame_timeout,
        )
        thread.client_event.connect(self.client_event)
        self._connect_reader_thread(thread)

    def _open_socket(self, host, port, protocol, role):
        """创建并配置底层 socket，设置 self._mode。"""
        if protocol == 'TCP' and role == 'Server':
            self.server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            self.server_sock.setblocking(False)
            self.server_sock.bind((host, port))
            self.server_sock.listen(5)
            self._mode = 'tcp_server'

        elif protocol == 'TCP' and role == 'Client':
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(5)
            self.sock.connect((host, port))
            self.sock.setblocking(False)
            self._mode = 'tcp_client'

        elif protocol == 'UDP' and role == 'Server':
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            self.sock.setblocking(False)
            self.sock.bind((host, port))
            self._mode = 'udp_server'

        elif protocol == 'UDP' and role == 'Client':
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            self.sock.setblocking(False)
            self._remote_addr = (host, port)
            self._mode = 'udp_client'

        else:
            raise ValueError(f"Unknown mode: {protocol}/{role}")

    def _close_resource(self):
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

    def _send_bytes(self, data: bytes) -> bool:
        if self._mode == 'tcp_client':
            self.sock.send(data)
        elif self._mode == 'tcp_server':
            if not self.reader_thread or not self.reader_thread.send_to_current(data):
                self.error_occurred.emit("没有已连接的客户端")
                return False
        elif self._mode in ('udp_server', 'udp_client'):
            if self._mode == 'udp_client':
                self.sock.sendto(data, self._remote_addr)
            else:
                if self.reader_thread and self.reader_thread.current_client:
                    self.sock.sendto(data, self.reader_thread.current_client)
                else:
                    self.error_occurred.emit("没有客户端地址")
                    return False
        return True
