"""端到端收发测试：Socket TCP/UDP 回环 — 所有 wait 均使用 qtbot.waitSignal"""
import socket
import time
import pytest

from src.io.socket_manager import SocketManager


_port_counter = [0]


def _find_free_port():
    _port_counter[0] += 1
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class TestTcpClientServerE2E:
    def test_server_receives_from_client(self, qapp, qtbot):
        received = []
        def on_data(data):
            received.append(data)

        port = _find_free_port()
        server = SocketManager()
        server.data_received.connect(on_data)

        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")

        client = SocketManager()

        with qtbot.waitSignal(server.client_event, timeout=3000):
            client.connect("127.0.0.1", port, "TCP", "Client")

        with qtbot.waitSignal(server.data_received, timeout=3000):
            client.send_data(b"hello from client")

        assert len(received) > 0
        assert b"hello from client" in received[0]

        client.disconnect()
        server.disconnect()

    def test_client_receives_from_server(self, qapp, qtbot):
        received = []
        def on_data(data):
            received.append(data)

        port = _find_free_port()
        server = SocketManager()

        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")

        client = SocketManager()
        client.data_received.connect(on_data)

        with qtbot.waitSignal(server.client_event, timeout=3000):
            client.connect("127.0.0.1", port, "TCP", "Client")

        with qtbot.waitSignal(client.data_received, timeout=3000):
            server.send_data(b"hello from server")

        assert len(received) > 0
        assert b"hello from server" in received[0]

        client.disconnect()
        server.disconnect()

    def test_bidirectional(self, qapp, qtbot):
        server_received = []
        client_received = []

        def on_server_data(data):
            server_received.append(data)
        def on_client_data(data):
            client_received.append(data)

        port = _find_free_port()
        server = SocketManager()
        server.data_received.connect(on_server_data)

        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")

        client = SocketManager()
        client.data_received.connect(on_client_data)

        with qtbot.waitSignal(server.client_event, timeout=3000):
            client.connect("127.0.0.1", port, "TCP", "Client")

        with qtbot.waitSignal(server.data_received, timeout=3000):
            client.send_data(b"ping")

        with qtbot.waitSignal(client.data_received, timeout=3000):
            server.send_data(b"pong")

        assert len(server_received) == 1
        assert server_received[0] == b"ping"
        assert len(client_received) == 1
        assert client_received[0] == b"pong"

        client.disconnect()
        server.disconnect()


class TestUdpE2E:
    def test_udp_server_receives(self, qapp, qtbot):
        received = []
        def on_data(data):
            received.append(data)

        port = _find_free_port()
        server = SocketManager()
        server.data_received.connect(on_data)

        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "UDP", "Server")

        client_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        client_sock.sendto(b"hello udp", ("127.0.0.1", port))

        with qtbot.waitSignal(server.data_received, timeout=3000):
            pass

        assert len(received) > 0
        assert b"hello udp" in received[0]

        client_sock.close()
        server.disconnect()


class TestConnectionSignals:
    def test_server_connect_signal(self, qapp, qtbot):
        """Server 连接时发出 connection_changed(True)"""
        port = _find_free_port()
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()

    def test_client_connect_signal(self, qapp, qtbot):
        """Client 连接时发出 connection_changed(True)"""
        port = _find_free_port()

        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")

        client = SocketManager()
        with qtbot.waitSignal(client.connection_changed, timeout=5000):
            client.connect("127.0.0.1", port, "TCP", "Client")

        assert server.is_connected is True
        assert client.is_connected is True

        client.disconnect()
        server.disconnect()

    def test_client_event_signal(self, qapp, qtbot):
        """Server 收到客户端连接时发出 client_event"""
        events = []
        def on_event(event_type, addr):
            events.append((event_type, addr))

        port = _find_free_port()
        server = SocketManager()
        server.client_event.connect(on_event)

        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")

        client = SocketManager()

        with qtbot.waitSignal(server.client_event, timeout=5000):
            client.connect("127.0.0.1", port, "TCP", "Client")

        assert len(events) > 0
        assert events[0][0] == "connected"

        client.disconnect()
        server.disconnect()


if __name__ == '__main__':
    # verify the test(s) can be run
    import sys
    from PySide6.QtWidgets import QApplication
    app = QApplication.instance() or QApplication(sys.argv)
    t = TestConnectionSignals()
    t.test_server_connect_signal(None, qtbot=None)  # won't work
    print("done")
