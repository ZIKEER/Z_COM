import socket
import select
import threading
import time
from PySide6.QtCore import Signal, QThread


def send_tcp_all(sock, data, timeout=5.0):
    """完整发送 TCP 数据，兼容非阻塞 socket。"""
    view = memoryview(data)
    deadline = time.monotonic() + timeout
    while view:
        try:
            sent = sock.send(view)
        except BlockingIOError:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("TCP 发送超时")
            _, writable, _ = select.select([], [sock], [], remaining)
            if not writable:
                raise TimeoutError("TCP 发送超时")
            continue
        except InterruptedError:
            continue

        if sent <= 0:
            raise ConnectionError("TCP 连接已关闭")
        view = view[sent:]


class SocketReaderThread(QThread):
    data_received = Signal(bytes)
    error_occurred = Signal(str)
    client_event = Signal(str, tuple)  # ('connected'|'disconnected', (host,port))

    def __init__(self, sock, mode, frame_timeout=50):
        """
        mode: 'tcp_client' | 'tcp_server' | 'udp_server' | 'udp_client'
        """
        super().__init__()
        self._sock = sock
        self._mode = mode
        self._stop_event = threading.Event()
        self._lock = threading.Lock()
        self._current_client = None
        # tcp_server: {fileno: (client_sock, (host, port))}
        self._clients = {}
        self.set_frame_timeout(frame_timeout)

    def set_frame_timeout(self, frame_timeout):
        self._frame_timeout = max(frame_timeout, 1) / 1000.0

    @property
    def current_client(self):
        return self._current_client

    def get_client_count(self):
        return len(self._clients)

    def send_to_current(self, data):
        with self._lock:
            if self._current_client is None:
                return False
            selected = next(
                ((fileno, client_sock) for fileno, (client_sock, addr) in self._clients.items()
                 if addr == self._current_client),
                None,
            )
        if selected is None:
            return False
        fileno, csock = selected
        try:
            send_tcp_all(csock, data)
            return True
        except Exception:
            with self._lock:
                event = self._remove_client(fileno)
            if event:
                self.client_event.emit(*event)
            return False

    def send_to_all(self, data):
        pending_events = []
        with self._lock:
            clients = list(self._clients.items())
        for fileno, (csock, addr) in clients:
            try:
                send_tcp_all(csock, data)
            except Exception:
                with self._lock:
                    evt = self._remove_client(fileno)
                if evt:
                    pending_events.append(evt)
        for event_type, addr in pending_events:
            self.client_event.emit(event_type, addr)

    def _remove_client(self, fileno):
        """移除客户端，返回 (event_type, addr) 或 None。调用方需持有锁。"""
        if fileno in self._clients:
            csock, addr = self._clients.pop(fileno)
            try:
                csock.close()
            except Exception:
                pass
            if self._current_client == addr:
                # 切到下一个客户端
                if self._clients:
                    self._current_client = next(iter(self._clients.values()))[1]
                else:
                    self._current_client = None
            return ('disconnected', addr)
        return None

    def run(self):
        self._stop_event.clear()
        poll_interval = self._frame_timeout
        while not self._stop_event.is_set() and not self.isInterruptionRequested():
            try:
                if self._mode == 'tcp_client':
                    rlist, _, _ = select.select([self._sock], [], [], poll_interval)
                    if rlist:
                        data = self._sock.recv(65536)
                        if data:
                            self.data_received.emit(bytes(data))
                        else:
                            break

                elif self._mode == 'tcp_server':
                    rlist, _, _ = select.select([self._sock], [], [], 0)
                    if rlist:
                        csock, addr = self._sock.accept()
                        csock.setblocking(False)
                        with self._lock:
                            self._clients[csock.fileno()] = (csock, addr)
                            self._current_client = addr
                        self.client_event.emit('connected', addr)

                    with self._lock:
                        client_list = list(self._clients.items())
                    if client_list:
                        socks = [csock for _, (csock, _) in client_list]
                        rlist, _, _ = select.select(socks, [], [], poll_interval)
                        pending_events = []
                        for csock in rlist:
                            fileno = csock.fileno()
                            addr = next((a for fn, (c, a) in client_list if fn == fileno), None)
                            try:
                                data = csock.recv(65536)
                                if data:
                                    with self._lock:
                                        self._current_client = addr
                                    self.data_received.emit(bytes(data))
                                else:
                                    with self._lock:
                                        evt = self._remove_client(fileno)
                                    if evt:
                                        pending_events.append(evt)
                            except Exception:
                                with self._lock:
                                    evt = self._remove_client(fileno)
                                if evt:
                                    pending_events.append(evt)
                        for event_type, addr in pending_events:
                            self.client_event.emit(event_type, addr)
                    else:
                        self.msleep(int(poll_interval * 1000))

                elif self._mode in ('udp_server', 'udp_client'):
                    rlist, _, _ = select.select([self._sock], [], [], poll_interval)
                    if rlist:
                        data, addr = self._sock.recvfrom(65536)
                        if data:
                            self._current_client = addr
                            self.data_received.emit(bytes(data))
                            if self._mode == 'udp_server':
                                self.client_event.emit('connected', addr)

                else:
                    self.msleep(int(poll_interval * 1000))

            except OSError as e:
                if not self._stop_event.is_set():
                    self.error_occurred.emit(str(e))
                break
            except Exception as e:
                if not self._stop_event.is_set():
                    self.error_occurred.emit(str(e))
                break

    def stop(self):
        self._stop_event.set()
        self.requestInterruption()
        with self._lock:
            for fileno, (csock, addr) in list(self._clients.items()):
                try:
                    csock.close()
                except Exception:
                    pass
            self._clients.clear()
            self._current_client = None
        self.wait(1000)
