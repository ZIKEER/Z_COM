import os
from datetime import datetime
from src.core.data_handler import DataHandler


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
        self._update_log_file()

    def _update_log_file(self):
        date_str = datetime.now().strftime("%Y-%m-%d")
        filename = f"log_{date_str}.txt"
        self.current_log_file = os.path.join(self.log_dir, filename)

    def log(self, data, direction, timestamp, display_mode='ASCII'):
        self._update_log_file()

        arrow = "\u2190" if direction == "RECEIVE" else "\u2192"

        ascii_data = DataHandler.bytes_to_ascii(data)
        hex_data = DataHandler.bytes_to_hex(data)

        log_entry = f"[{timestamp}]\n"
        log_entry += f" {arrow} HEX: {hex_data}\n"
        log_entry += f" {arrow} ASCII: {ascii_data}\n"

        with open(self.current_log_file, 'a', encoding='utf-8') as f:
            f.write(log_entry)
