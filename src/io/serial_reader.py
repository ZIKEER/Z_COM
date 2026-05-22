import time
import threading
from PySide6.QtCore import Signal, QThread
import serial


class SerialReaderThread(QThread):
    data_received = Signal(bytes)
    error_occurred = Signal(str)

    def __init__(self, serial_port, frame_timeout=50):
        super().__init__()
        self.serial_port = serial_port
        self.running = False
        self.buffer = bytearray()
        self.frame_timeout = frame_timeout / 1000.0
        self.last_receive_time = 0
        self._lock = threading.Lock()

    def run(self):
        self.running = True
        while self.running:
            try:
                if not self.serial_port.is_open:
                    break

                if self.serial_port.in_waiting:
                    with self._lock:
                        data = self.serial_port.read(self.serial_port.in_waiting)
                    current_time = time.time()

                    if self.buffer and (current_time - self.last_receive_time) > self.frame_timeout:
                        self.data_received.emit(bytes(self.buffer))
                        self.buffer.clear()

                    self.buffer.extend(data)
                    self.last_receive_time = current_time
                else:
                    if self.buffer and self.last_receive_time > 0:
                        if (time.time() - self.last_receive_time) > self.frame_timeout:
                            self.data_received.emit(bytes(self.buffer))
                            self.buffer.clear()

                    self.msleep(10)
            except serial.SerialException as e:
                self.error_occurred.emit(f"\u4e32\u53e3\u9519\u8bef: {str(e)}")
                break
            except Exception as e:
                if self.running:
                    self.error_occurred.emit(f"\u8bfb\u53d6\u9519\u8bef: {str(e)}")
                    self.msleep(10)

        self.running = False

    def stop(self):
        self.running = False
        with self._lock:
            if self.buffer:
                self.data_received.emit(bytes(self.buffer))
                self.buffer.clear()
        self.wait(1000)
        if self.isRunning():
            self.terminate()
