import os


class Logger:
    def __init__(self, log_dir=None, instance_id=1):
        if log_dir:
            self.log_dir = log_dir
        elif instance_id > 1:
            self.log_dir = os.path.join(f"instance_{instance_id}", "logs")
        else:
            self.log_dir = "logs"
        os.makedirs(self.log_dir, exist_ok=True)
        self.current_log_file = None
        self._buffer = []
        self._update_log_file()

    def _update_log_file(self):
        from datetime import datetime
        date_str = datetime.now().strftime("%Y-%m-%d")
        self.current_log_file = os.path.join(self.log_dir, f"log_{date_str}.txt")

    def log(self, timestamp, direction, hex_str, ascii_str):
        arrow = "\u2190" if direction == "RECEIVE" else "\u2192"
        entry = f"[{timestamp}]\n {arrow} HEX: {hex_str}\n {arrow} ASCII: {ascii_str}\n"
        self._buffer.append(entry)

    def flush(self):
        if not self._buffer:
            return
        self._update_log_file()
        try:
            with open(self.current_log_file, 'a', encoding='utf-8') as f:
                f.write(''.join(self._buffer))
        except Exception as e:
            print(f"\u65e5\u5fd7\u5199\u5165\u5931\u8d25: {e}")
        self._buffer.clear()
