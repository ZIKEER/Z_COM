"""验证 IOTransport 抽象基类的接口定义"""
from src.io.io_transport import IOTransport


def test_abstract_methods_raise():
    """子类必须实现的模板方法应抛出 NotImplementedError。"""
    t = IOTransport()
    abstract_methods = [
        ("get_available_devices",),
        ("_connect_impl",),
        ("_close_resource",),
        ("_send_bytes", {"data": b""}),
    ]
    for item in abstract_methods:
        method_name = item[0]
        args = item[1] if len(item) > 1 else {}
        try:
            getattr(t, method_name)(**args) if args else getattr(t, method_name)()
        except NotImplementedError:
            pass
        else:
            assert False, f"{method_name} should raise NotImplementedError"


def test_default_lifecycle_methods_exist():
    """基类提供默认的生命周期和配置方法。"""
    t = IOTransport()
    assert callable(getattr(t, 'open_connection', None))
    assert callable(getattr(t, 'close_connection', None))
    assert callable(getattr(t, 'update_settings', None))
    assert callable(getattr(t, 'send_data', None))
    assert callable(getattr(t, '_parse_send_data', None))


def test_io_transport_is_qobject():
    from PySide6.QtCore import QObject
    assert issubclass(IOTransport, QObject)


def test_io_transport_signals():
    t = IOTransport()
    assert hasattr(t, "data_received")
    assert hasattr(t, "connection_changed")
    assert hasattr(t, "error_occurred")
