import socket, pytest
from src.io.socket_manager import SocketManager
def port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]

class TestB:
    def test_b(self, qapp, qtbot):
        p = port()
        srv = SocketManager()
        cli_ev = []
        srv.client_event.connect(lambda t,a: cli_ev.append((t,a)))
        with qtbot.waitSignal(srv.connection_changed, timeout=3000):
            srv.connect("127.0.0.1", p, "TCP", "Server")
        cli = SocketManager()
        cli.data_received.connect(lambda d: None)
        with qtbot.waitSignal(srv.client_event, timeout=3000):
            cli.connect("127.0.0.1", p, "TCP", "Client")
        with qtbot.waitSignal(cli.data_received, timeout=3000):
            srv.send_data(b"x")
        cli.disconnect()
        srv.disconnect()
