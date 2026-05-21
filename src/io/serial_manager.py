import serial
import serial.tools.list_ports
from src.io.io_transport import IOTransport
from src.io.serial_reader import SerialReaderThread


class SerialManager(IOTransport):

    def __init__(self):
        super().__init__()
        self.serial = serial.Serial()
        self.is_connected = False
        self.reader_thread = None
        self._lock = __import__('threading').Lock()

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

    def update_settings(self, settings):
        self.settings.update(settings)

    def reconfigure(self):
        with self._lock:
            if not self.is_connected or not self.serial.is_open:
                return False

            try:
                self.serial.baudrate = self.settings['baudrate']
                self.serial.bytesize = self.settings['databits']
                stopbits_map = {1: serial.STOPBITS_ONE, 1.5: serial.STOPBITS_ONE_POINT_FIVE, 2: serial.STOPBITS_TWO}
                self.serial.stopbits = stopbits_map.get(self.settings['stopbits'], serial.STOPBITS_ONE)
                parity_map = {
                    'None': serial.PARITY_NONE, 'Even': serial.PARITY_EVEN,
                    'Odd': serial.PARITY_ODD, 'Mark': serial.PARITY_MARK,
                    'Space': serial.PARITY_SPACE,
                }
                self.serial.parity = parity_map.get(self.settings['parity'], serial.PARITY_NONE)
                return True
            except Exception as e:
                self.error_occurred.emit(f"\u91cd\u65b0\u914d\u7f6e\u5931\u8d25: {str(e)}")
                return False

    def connect(self, port):
        with self._lock:
            if self.is_connected:
                self._disconnect_internal()

            try:
                self.serial.port = port
                self.serial.baudrate = self.settings['baudrate']
                self.serial.bytesize = self.settings['databits']
                stopbits_map = {1: serial.STOPBITS_ONE, 1.5: serial.STOPBITS_ONE_POINT_FIVE, 2: serial.STOPBITS_TWO}
                self.serial.stopbits = stopbits_map.get(self.settings['stopbits'], serial.STOPBITS_ONE)
                parity_map = {
                    'None': serial.PARITY_NONE, 'Even': serial.PARITY_EVEN,
                    'Odd': serial.PARITY_ODD, 'Mark': serial.PARITY_MARK,
                    'Space': serial.PARITY_SPACE,
                }
                self.serial.parity = parity_map.get(self.settings['parity'], serial.PARITY_NONE)
                self.serial.open()
                self.is_connected = True

                frame_timeout = self.settings.get('frame_timeout', 50)
                self.reader_thread = SerialReaderThread(self.serial, frame_timeout)
                self.reader_thread.data_received.connect(self.data_received)
                self.reader_thread.error_occurred.connect(self._on_thread_error)
                self.reader_thread.finished.connect(self._on_thread_finished)
                self.reader_thread.start()

                self.connection_changed.emit(True)
                return True
            except Exception as e:
                self._disconnect_internal()
                self.error_occurred.emit(f"\u8fde\u63a5\u5931\u8d25: {str(e)}")
                return False

    def disconnect(self):
        with self._lock:
            return self._disconnect_internal()

    def _disconnect_internal(self):
        try:
            if self.reader_thread and self.reader_thread.isRunning():
                self.reader_thread.stop()
                self.reader_thread = None

            if self.serial.is_open:
                self.serial.close()

            self.is_connected = False
            self.connection_changed.emit(False)
            return True
        except Exception as e:
            self.error_occurred.emit(f"\u65ad\u5f00\u5931\u8d25: {str(e)}")
            return False

    def _on_thread_error(self, error_msg):
        self.error_occurred.emit(error_msg)
        self.disconnect()

    def _on_thread_finished(self):
        if self.is_connected:
            self.disconnect()

    def send_data(self, data, is_hex=False):
        try:
            if is_hex:
                hex_str = data.replace(' ', '').replace('\n', '')
                bytes_data = bytes.fromhex(hex_str)
            else:
                bytes_data = data

            self.serial.write(bytes_data)
            return True
        except Exception as e:
            self.error_occurred.emit(str(e))
            return False
