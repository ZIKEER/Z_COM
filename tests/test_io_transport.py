"""验证 IOTransport 抽象基类的接口定义"""
from src.io.io_transport import IOTransport


def test_abstract_methods_raise():
    t = IOTransport()
    methods = [
        ("update_settings", {"settings": {}}),
        ("connect",),
        ("disconnect",),
        ("send_data", {"data": b""}),
        ("get_available_devices",),
    ]
    for item in methods:
        method_name = item[0]
        args = item[1] if len(item) > 1 else {}
        try:
            getattr(t, method_name)(**args) if args else getattr(t, method_name)()
        except NotImplementedError:
            pass
        else:
            assert False, f"{method_name} should raise NotImplementedError"


def test_io_transport_is_qobject():
    from PySide6.QtCore import QObject
    assert issubclass(IOTransport, QObject)


def test_io_transport_signals():
    t = IOTransport()
    assert hasattr(t, "data_received")
    assert hasattr(t, "connection_changed")
    assert hasattr(t, "error_occurred")
