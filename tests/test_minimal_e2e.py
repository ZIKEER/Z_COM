"""Minimal E2E test - just test server connection and disconnect"""
import socket
import sys
import pytest
from PySide6.QtCore import QTimer
from src.io.socket_manager import SocketManager


def _find_free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class TestSimpleE2E:
    def test_connect_disconnect(self, qapp, qtbot):
        port = _find_free_port()
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()
        assert server.is_connected is False

    def test_connect_disconnect_2(self, qapp, qtbot):
        port = _find_free_port()
        server = SocketManager()
        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")
        assert server.is_connected is True
        server.disconnect()
        assert server.is_connected is False

    def test_send_receive(self, qapp, qtbot):
        received = []
        def on_data(data):
            received.append(data)

        port = _find_free_port()
        server = SocketManager()
        server.data_received.connect(on_data)

        with qtbot.waitSignal(server.connection_changed, timeout=3000):
            server.connect("127.0.0.1", port, "TCP", "Server")

        client = SocketManager()
        with qtbot.waitSignal(client.connection_changed, timeout=3000):
            client.connect("127.0.0.1", port, "TCP", "Client")

        client.send_data(b"hello")

        with qtbot.waitSignal(server.data_received, timeout=3000):
            pass

        assert len(received) > 0
        assert b"hello" in received[0]

        client.disconnect()
        server.disconnect()
