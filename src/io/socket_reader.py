import socket
import threading
from PySide6.QtCore import Signal, QThread


class SocketReaderThread(QThread):
    data_received = Signal(bytes)
    error_occurred = Signal(str)
    client_event = Signal(str, tuple)  # ('connected'|'disconnected', (host,port))

    def __init__(self, sock, mode):
        """
        mode: 'tcp_client' | 'tcp_server' | 'udp'
        """
        super().__init__()
        self._sock = sock
        self._mode = mode
        self._stop_event = threading.Event()
        self._lock = threading.Lock()
        self._current_client = None
        # tcp_server: {fileno: (client_sock, (host, port))}
        self._clients = {}

    @property
    def current_client(self):
        return self._current_client

    def get_client_count(self):
        return len(self._clients)

    def send_to_current(self, data):
        with self._lock:
            if self._current_client is None:
                return False
            for fileno, (csock, addr) in self._clients.items():
                if addr == self._current_client:
                    try:
                        csock.send(data)
                        return True
                    except Exception:
                        return False
            return False

    def send_to_all(self, data):
        with self._lock:
            for fileno, (csock, addr) in list(self._clients.items()):
                try:
                    csock.send(data)
                except Exception:
                    self._remove_client(fileno)

    def _remove_client(self, fileno):
        if fileno in self._clients:
            csock, addr = self._clients.pop(fileno)
            try:
                csock.close()
            except Exception:
                pass
            self.client_event.emit('disconnected', addr)
            if self._current_client == addr:
                # 切到下一个客户端
                if self._clients:
                    self._current_client = next(iter(self._clients.values()))[1]
                else:
                    self._current_client = None

    def run(self):
        self._stop_event.clear()
        while not self._stop_event.is_set():
            try:
                if self._mode == 'tcp_client':
                    try:
                        data = self._sock.recv(65536)
                        if data:
                            self.data_received.emit(bytes(data))
                        else:
                            break
                    except BlockingIOError:
                        self.msleep(20)

                elif self._mode == 'tcp_server':
                    try:
                        csock, addr = self._sock.accept()
                        csock.setblocking(False)
                        with self._lock:
                            self._clients[csock.fileno()] = (csock, addr)
                            self._current_client = addr
                        self.client_event.emit('connected', addr)
                    except BlockingIOError:
                        pass

                    with self._lock:
                        client_list = list(self._clients.items())
                    for fileno, (csock, addr) in client_list:
                        try:
                            data = csock.recv(65536)
                            if data:
                                with self._lock:
                                    self._current_client = addr
                                self.data_received.emit(bytes(data))
                            else:
                                with self._lock:
                                    self._remove_client(fileno)
                        except BlockingIOError:
                            pass
                        except Exception:
                            with self._lock:
                                self._remove_client(fileno)

                    self.msleep(20)

                elif self._mode == 'udp':
                    try:
                        data, addr = self._sock.recvfrom(65536)
                        if data:
                            self._current_client = addr
                            self.data_received.emit(bytes(data))
                    except BlockingIOError:
                        self.msleep(20)

                else:
                    self.msleep(20)

            except OSError as e:
                if not self._stop_event.is_set():
                    self.error_occurred.emit(str(e))
                    self.msleep(20)
            except Exception as e:
                if not self._stop_event.is_set():
                    self.error_occurred.emit(str(e))
                    self.msleep(20)

    def stop(self):
        self._stop_event.set()
        try:
            self._sock.close()
        except Exception:
            pass
