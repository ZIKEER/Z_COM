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
        self.last_receive_time = 0
        self.buffer_start_time = 0
        self._lock = threading.Lock()
        self.set_frame_timeout(frame_timeout)

    def set_frame_timeout(self, frame_timeout):
        self.frame_timeout = max(frame_timeout, 1) / 1000.0

    def _emit_buffer(self):
        if not self.buffer:
            return
        self.data_received.emit(bytes(self.buffer))
        self.buffer.clear()
        self.last_receive_time = 0
        self.buffer_start_time = 0

    def run(self):
        self.running = True
        while self.running:
            try:
                if not self.serial_port.is_open:
                    break

                if self.serial_port.in_waiting:
                    with self._lock:
                        data = self.serial_port.read(self.serial_port.in_waiting)
                    current_time = time.monotonic()

                    if self.buffer and (current_time - self.last_receive_time) > self.frame_timeout:
                        self._emit_buffer()

                    if not self.buffer:
                        self.buffer_start_time = current_time

                    self.buffer.extend(data)
                    self.last_receive_time = current_time

                    if (current_time - self.buffer_start_time) >= self.frame_timeout:
                        self._emit_buffer()
                else:
                    if self.buffer and self.last_receive_time > 0:
                        if (time.monotonic() - self.last_receive_time) > self.frame_timeout:
                            self._emit_buffer()

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
            self._emit_buffer()
        self.wait(1000)
        if self.isRunning():
            self.terminate()
