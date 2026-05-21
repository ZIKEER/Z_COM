import socket, pytest
from src.io.socket_manager import SocketManager

class TestSimple:
    def test_connect(self, qapp, qtbot):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()

    def test_connect2(self, qapp, qtbot):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()

    def test_connect3(self, qapp, qtbot):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()

    def test_connect4(self, qapp, qtbot):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()

    def test_connect5(self, qapp, qtbot):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()

    def test_connect6(self, qapp, qtbot):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()
