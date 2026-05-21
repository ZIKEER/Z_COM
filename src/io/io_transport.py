from PySide6.QtCore import QObject, Signal


class IOTransport(QObject):
    data_received = Signal(bytes)
    connection_changed = Signal(bool)
    error_occurred = Signal(str)

    def update_settings(self, settings):
        raise NotImplementedError

    def connect(self, **kwargs):
        raise NotImplementedError

    def disconnect(self):
        raise NotImplementedError

    def send_data(self, data, is_hex=False):
        raise NotImplementedError

    def get_available_devices(self):
        raise NotImplementedError
