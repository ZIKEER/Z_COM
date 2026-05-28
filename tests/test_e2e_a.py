import socket, pytest
from src.io.socket_manager import SocketManager
def port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]

class TestA:
    def test_a(self, qapp, qtbot):
        p = port()
        srv = SocketManager()
        srv.data_received.connect(lambda d: None)
        with qtbot.waitSignal(srv.connection_changed, timeout=3000):
            srv.open_connection("127.0.0.1", p, "TCP", "Server")
        cli = SocketManager()
        with qtbot.waitSignal(cli.connection_changed, timeout=3000):
            cli.open_connection("127.0.0.1", p, "TCP", "Client")
        with qtbot.waitSignal(srv.data_received, timeout=3000):
            cli.send_data(b"x")
        cli.close_connection()
        srv.close_connection()
