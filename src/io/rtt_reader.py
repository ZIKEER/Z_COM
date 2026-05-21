from PySide6.QtCore import Signal, QThread


class RttReaderThread(QThread):
    data_received = Signal(bytes)
    error_occurred = Signal(str)

    def __init__(self, jlink, buffer_idx=0, read_size=8192, read_interval=0.002):
        super().__init__()
        self.jlink = jlink
        self.buffer_idx = buffer_idx
        self.read_size = read_size
        self.read_interval = read_interval
        self.running = False

    def run(self):
        self.running = True
        while self.running:
            try:
                if self.jlink.opened():
                    rtt_data = self.jlink.rtt_read(self.buffer_idx, self.read_size)
                    if rtt_data:
                        self.data_received.emit(bytes(rtt_data))
                self.msleep(int(self.read_interval * 1000))
            except Exception as e:
                if self.running:
                    self.error_occurred.emit(f"RTT\u8bfb\u53d6\u9519\u8bef: {str(e)}")
                    self.msleep(10)

        self.running = False

    def stop(self):
        self.running = False
        self.wait(1000)
        if self.isRunning():
            self.terminate()
