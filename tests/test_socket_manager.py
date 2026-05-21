import socket
import threading
import time
from unittest.mock import MagicMock, patch

import src.io.socket_manager as sock_mod


class TestGetLocalIps:
    def test_returns_list_of_strings(self):
        from src.io.socket_manager import get_local_ips
        ips = get_local_ips()
        assert isinstance(ips, list)
        assert len(ips) > 0
        assert all(isinstance(ip, str) for ip in ips)
        assert "0.0.0.0" in ips
        assert "127.0.0.1" in ips

    def test_no_duplicates(self):
        from src.io.socket_manager import get_local_ips
        ips = get_local_ips()
        assert len(ips) == len(set(ips))


class SocketManagerUnit:
    """Helper: unit-test-level SocketManager with mocked reader thread"""

    @staticmethod
    def create(qapp, qtbot, host, port, protocol, role, expected_signal="connection_changed"):
        from src.io.socket_manager import SocketManager
        import src.io.socket_manager as sock_mod

        mgr = SocketManager()

        with patch.object(sock_mod, "SocketReaderThread") as mock_cls:
            mock_thread = MagicMock()
            mock_cls.return_value = mock_thread

            if expected_signal == "connection_changed":
                sig = mgr.connection_changed
            else:
                sig = mgr.error_occurred

            with qtbot.waitSignal(sig, timeout=3000):
                result = mgr.connect(host, port, protocol, role)

        return mgr, result


class TestSocketManager:
    def test_init(self, qapp):
        from src.io.socket_manager import SocketManager
        mgr = SocketManager()
        assert mgr.is_connected is False
        assert mgr.mode is None

    def test_update_settings(self, qapp):
        from src.io.socket_manager import SocketManager
        mgr = SocketManager()
        mgr.update_settings({"host": "192.168.1.1", "port": 1234})
        assert mgr.settings["host"] == "192.168.1.1"
        assert mgr.settings["port"] == 1234

    def test_tcp_client_connect_fail(self, qapp, qtbot):
        from src.io.socket_manager import SocketManager
        mgr = SocketManager()
        with qtbot.waitSignal(mgr.error_occurred, timeout=5000):
            result = mgr.connect("127.0.0.1", 1, "TCP", "Client")
        assert result is False
        assert mgr.is_connected is False

    def test_udp_server_bind(self, qapp, qtbot):
        mgr, result = SocketManagerUnit.create(qapp, qtbot, "127.0.0.1", 0, "UDP", "Server")
        assert result is True
        assert mgr.mode == "udp_server"
        mgr.disconnect()

    def test_udp_client(self, qapp, qtbot):
        mgr, result = SocketManagerUnit.create(qapp, qtbot, "127.0.0.1", 9999, "UDP", "Client")
        assert result is True
        assert mgr.mode == "udp_client"
        mgr.disconnect()

    def test_disconnect_cleanup(self, qapp, qtbot):
        mgr, _ = SocketManagerUnit.create(qapp, qtbot, "127.0.0.1", 0, "UDP", "Server")

        with qtbot.waitSignal(mgr.connection_changed, timeout=2000):
            mgr.disconnect()
        assert mgr.is_connected is False
        assert mgr.mode is None


class TestSocketLoopback:
    """实际 TCP 回环测试 — 使用 qtbot.waitSignal 避免阻塞事件循环"""

    def test_tcp_echo(self, qapp, qtbot):
        from src.io.socket_manager import SocketManager

        received = []
        def on_data(data):
            received.append(data)

        server = SocketManager()
        server.data_received.connect(on_data)

        port = _find_free_port()

        with qtbot.waitSignal(server.connection_changed, timeout=2000):
            server.connect("127.0.0.1", port, "TCP", "Server")

        client_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        client_sock.settimeout(3)
        client_sock.connect(("127.0.0.1", port))

        client_sock.send(b"ping")

        with qtbot.waitSignal(server.data_received, timeout=3000):
            pass

        assert len(received) > 0
        assert received[0] == b"ping"

        client_sock.close()
        server.disconnect()

    def test_udp_echo(self, qapp, qtbot):
        from src.io.socket_manager import SocketManager

        received = []
        def on_data(data):
            received.append(data)

        server = SocketManager()
        server.data_received.connect(on_data)

        port = _find_free_port()

        with qtbot.waitSignal(server.connection_changed, timeout=2000):
            server.connect("127.0.0.1", port, "UDP", "Server")

        client_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        client_sock.sendto(b"hello", ("127.0.0.1", port))

        with qtbot.waitSignal(server.data_received, timeout=3000):
            pass

        assert len(received) > 0
        assert received[0] == b"hello"

        client_sock.close()
        server.disconnect()

    def test_tcp_client_send(self, qapp, qtbot):
        from src.io.socket_manager import SocketManager

        received = []
        def on_data(data):
            received.append(data)

        server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        port = _find_free_port()
        server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server_sock.bind(("127.0.0.1", port))
        server_sock.listen(1)
        server_sock.settimeout(3)

        client = SocketManager()
        client.data_received.connect(on_data)

        with qtbot.waitSignal(client.connection_changed, timeout=2000):
            client.connect("127.0.0.1", port, "TCP", "Client")

        conn, addr = server_sock.accept()
        conn.send(b"world")

        with qtbot.waitSignal(client.data_received, timeout=3000):
            pass

        assert len(received) > 0
        assert received[0] == b"world"

        conn.close()
        client.disconnect()
        server_sock.close()


def _find_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
