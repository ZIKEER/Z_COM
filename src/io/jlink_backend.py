import re
import threading

from src.io.rtt_backend import RttBackend


def _parse_optional_int(value):
    if value is None:
        return None
    text = str(value).strip()
    if not text:
        return None
    return int(text, 0)


class JLinkRttBackend(RttBackend):
    """J-Link RTT 后端。"""

    def __init__(self):
        self.jlink = None
        self.is_connected = False
        self._lock = threading.Lock()
        self.settings = {}

    def update_settings(self, settings):
        self.settings.update(settings)

    def _import_pylink(self):
        try:
            import pylink
            return pylink
        except ImportError:
            return None

    def get_available_devices(self):
        pylink = self._import_pylink()
        if pylink is None:
            return []

        devices = []
        try:
            jlink_temp = pylink.JLink()
            try:
                connected_emulators = jlink_temp.connected_emulators()
            except Exception:
                connected_emulators = []

            for emu in connected_emulators:
                try:
                    desc = str(emu)
                    match = re.search(r'Serial No\.\s*(\d+)', desc)
                    if not match:
                        continue
                    serial_no = int(match.group(1))
                    try:
                        jlink_temp.open(serial_no=serial_no)
                        name = getattr(jlink_temp, 'product_name', 'J-Link')
                        jlink_temp.close()
                        devices.append((serial_no, f"{name} (SN={serial_no})"))
                    except Exception:
                        try:
                            jlink_temp.close()
                        except Exception:
                            pass
                        devices.append((serial_no, desc))
                except Exception:
                    continue

            if not devices:
                try:
                    jlink_temp.open()
                    serial_no = jlink_temp.serial_number
                    name = getattr(jlink_temp, 'product_name', 'J-Link')
                    jlink_temp.close()
                    devices.append((serial_no, f"{name} (SN={serial_no})"))
                except Exception:
                    try:
                        jlink_temp.close()
                    except Exception:
                        pass
        except Exception:
            pass
        return devices

    def connect(self, serial_no=None, chip=None, speed=None, reset_flag=None,
                start_address=None, range_size=None):
        pylink = self._import_pylink()
        if pylink is None:
            raise RuntimeError("未安装 pylink-square，无法使用 J-Link RTT")

        with self._lock:
            if self.is_connected:
                self._disconnect_internal()

            chip = chip or self.settings.get('chip', 'nRF52840_xxAA')
            speed = int(speed or self.settings.get('speed', 4000))
            reset_flag = reset_flag if reset_flag is not None else self.settings.get('reset', False)
            start_address = _parse_optional_int(
                start_address if start_address is not None else self.settings.get('start_address', '')
            )

            try:
                self.jlink = pylink.JLink()
                if serial_no:
                    self.jlink.open(serial_no=int(serial_no))
                else:
                    self.jlink.open()

                self.jlink.set_tif(pylink.enums.JLinkInterfaces.SWD)
                self.jlink.set_speed(speed)
                self.jlink.connect(chip)

                if reset_flag:
                    self.jlink.reset(ms=10, halt=False)

                self.jlink.rtt_start(start_address)
                self.is_connected = True
                return True
            except Exception:
                self._disconnect_internal()
                raise

    def disconnect(self):
        with self._lock:
            return self._disconnect_internal()

    def _disconnect_internal(self):
        try:
            if self.jlink and hasattr(self.jlink, 'opened') and self.jlink.opened():
                try:
                    self.jlink.rtt_stop()
                except Exception:
                    pass
                self.jlink.close()
            self.jlink = None
            self.is_connected = False
            return True
        except Exception:
            return False

    def rtt_read(self, buffer_idx=0, read_size=8192):
        if self.jlink and self.jlink.opened():
            return self.jlink.rtt_read(buffer_idx, read_size)
        return b''

    def rtt_write(self, buffer_idx=0, data=None):
        if data is None:
            data = []
        if self.jlink and self.jlink.opened():
            if isinstance(data, bytes):
                data = list(data)
            self.jlink.rtt_write(buffer_idx, data)
            return len(data)
        return 0

    def is_opened(self):
        return self.is_connected and self.jlink is not None

    def get_serial_number(self):
        if self.jlink and hasattr(self.jlink, 'opened') and self.jlink.opened():
            return self.jlink.serial_number
        return 0
