import serial
import serial.tools.list_ports
from src.io.io_transport import IOTransport
from src.io.serial_reader import SerialReaderThread

_STOPBITS_MAP = {
    1: serial.STOPBITS_ONE,
    1.5: serial.STOPBITS_ONE_POINT_FIVE,
    2: serial.STOPBITS_TWO,
}
_PARITY_MAP = {
    'None': serial.PARITY_NONE,
    'Even': serial.PARITY_EVEN,
    'Odd': serial.PARITY_ODD,
    'Mark': serial.PARITY_MARK,
    'Space': serial.PARITY_SPACE,
}


class SerialManager(IOTransport):

    def __init__(self):
        super().__init__()
        self.serial = serial.Serial()
        self.settings = {
            'baudrate': 115200,
            'databits': 8,
            'stopbits': 1,
            'parity': 'None',
            'flowcontrol': 'None',
            'frame_timeout': 50,
        }

    def get_available_ports(self):
        ports = serial.tools.list_ports.comports()
        return [(p.device, p.description) for p in ports]

    def get_available_devices(self):
        return self.get_available_ports()

    def _apply_serial_params(self):
        """将 settings 中的参数应用到 serial 对象。"""
        self.serial.baudrate = self.settings['baudrate']
        self.serial.bytesize = self.settings['databits']
        self.serial.stopbits = _STOPBITS_MAP.get(self.settings['stopbits'], serial.STOPBITS_ONE)
        self.serial.parity = _PARITY_MAP.get(self.settings['parity'], serial.PARITY_NONE)

    def reconfigure(self):
        with self._lock:
            if not self.is_connected or not self.serial.is_open:
                return False
            try:
                self._apply_serial_params()
                return True
            except Exception as e:
                self.error_occurred.emit(f"重新配置失败: {str(e)}")
                return False

    def _connect_impl(self, port):
        self.serial.port = port
        self._apply_serial_params()
        self.serial.open()
        try:
            frame_timeout = self.settings.get('frame_timeout', 50)
            thread = SerialReaderThread(self.serial, frame_timeout)
            self._connect_reader_thread(thread)
        except Exception:
            self.serial.close()
            raise

    def _close_resource(self):
        if self.serial.is_open:
            self.serial.close()

    def _send_bytes(self, data: bytes) -> bool:
        self.serial.write(data)
        return True
