from abc import ABC, abstractmethod


class RttBackend(ABC):
    """RTT 后端抽象接口"""

    @abstractmethod
    def get_available_devices(self):
        pass

    @abstractmethod
    def connect(self, **kwargs):
        pass

    @abstractmethod
    def disconnect(self):
        pass

    @abstractmethod
    def rtt_read(self, buffer_idx=0, read_size=8192):
        pass

    @abstractmethod
    def rtt_write(self, buffer_idx=0, data=None):
        pass

    @abstractmethod
    def is_opened(self):
        pass

    @abstractmethod
    def get_serial_number(self):
        pass

    @abstractmethod
    def update_settings(self, settings):
        pass
