import time
from PySide6.QtCore import Signal, QThread

EMIT_THRESHOLD = 4096


class RttReaderThread(QThread):
    data_received = Signal(bytes)
    error_occurred = Signal(str)

    def __init__(self, jlink, buffer_idx=0, read_size=8192, read_interval=0.002,
                 frame_timeout=0.05):
        super().__init__()
        self.jlink = jlink
        self.buffer_idx = buffer_idx
        self.read_size = read_size
        self.read_interval = read_interval
        self.frame_timeout = frame_timeout
        self.running = False
        self._buffer = bytearray()
        self._last_receive_time = 0

    def run(self):
        self.running = True
        while self.running:
            try:
                if self.jlink.opened():
                    rtt_data = self.jlink.rtt_read(self.buffer_idx, self.read_size)
                    if rtt_data:
                        self._buffer.extend(rtt_data)
                        self._last_receive_time = time.time()

                    if self._buffer:
                        now = time.time()
                        if (len(self._buffer) >= EMIT_THRESHOLD or
                            (now - self._last_receive_time) >= self.frame_timeout):
                            self.data_received.emit(bytes(self._buffer))
                            self._buffer.clear()
                self.msleep(int(self.read_interval * 1000))
            except Exception as e:
                if self.running:
                    self.error_occurred.emit(f"RTT读取错误: {str(e)}")
                    self.msleep(10)

        if self._buffer:
            self.data_received.emit(bytes(self._buffer))
            self._buffer.clear()
        self.running = False

    def stop(self):
        self.running = False
        self.wait(1000)
        if self.isRunning():
            self.terminate()
