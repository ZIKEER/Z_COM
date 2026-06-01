import os
import threading
from datetime import datetime

MAX_LOG_FILE_SIZE = 50 * 1024 * 1024  # 50MB


class Logger:
    _global_counter = 0
    _counter_lock = threading.Lock()

    def __init__(self, log_dir=None, instance_id=1):
        if log_dir:
            self.log_dir = log_dir
        elif instance_id > 1:
            self.log_dir = os.path.join(f"instance_{instance_id}", "logs")
        else:
            self.log_dir = "logs"
        os.makedirs(self.log_dir, exist_ok=True)
        self.current_log_file = ""
        self._buffer = []
        self._lock = threading.Lock()
        self._update_log_file()

    def _update_log_file(self):
        with Logger._counter_lock:
            Logger._global_counter += 1
            counter = Logger._global_counter
        timestamp = datetime.now().strftime("%Y-%m-%d_%H%M%S")
        suffix = f"_{counter}" if counter > 1 else ""
        self.current_log_file = os.path.join(
            self.log_dir, f"log_{timestamp}{suffix}.txt"
        )

    def _rotate_log_file(self):
        self._update_log_file()

    def log(self, timestamp, direction, hex_str, ascii_str):
        arrow = "←" if direction == "RECEIVE" else "→"
        entry = f"[{timestamp}]\n {arrow} HEX: {hex_str}\n {arrow} ASCII: {ascii_str}\n"
        with self._lock:
            self._buffer.append(entry)

    def log_event(self, text):
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
        entry = f"[{timestamp}] {text}\n"
        with self._lock:
            self._buffer.append(entry)

    def flush(self):
        with self._lock:
            if not self._buffer:
                return
            data = ''.join(self._buffer)
            self._buffer.clear()
        try:
            with open(self.current_log_file, 'a', encoding='utf-8') as f:
                f.write(data)
            try:
                if os.path.getsize(self.current_log_file) >= MAX_LOG_FILE_SIZE:
                    self._rotate_log_file()
            except OSError:
                pass
        except Exception as e:
            with self._lock:
                self._buffer.insert(0, data)
            print(f"日志写入失败: {e}")
