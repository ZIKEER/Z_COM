import struct
import threading
import time

from src.core.config_manager import get_known_dap_vendor_ids, is_dap_device
from src.io.cmsis_dap import CmsisDapProtocol
from src.io.rtt_backend import RttBackend


RTT_CB_ACID = 0
RTT_CB_MAX_UP = 16
RTT_CB_MAX_DOWN = 20
RTT_CB_AUP = 24

RTT_BUFFER_SIZE = 24

RB_PBUFFER = 0
RB_SIZE = 4
RB_WROFF = 8
RB_RDOFF = 12


def _normalize_device_id(serial, vid, pid, path):
    if serial:
        return serial.replace(':', '_').replace(' ', '_')
    path_hash = abs(hash(path)) & 0xFFFFFFFF
    return f"{vid:04X}_{pid:04X}_{path_hash:08X}"


def _parse_optional_int(value, default_value):
    if value is None:
        return default_value
    text = str(value).strip()
    if not text:
        return default_value
    return int(text, 0)


def _normalize_speed_hz(speed):
    """兼容当前 UI 的 kHz 输入习惯。"""
    value = int(speed)
    if value <= 0:
        raise ValueError("SWD 速度必须大于 0")
    # RTT 设置框当前显示为 kHz，用户输入 4000 实际期望 4 MHz。
    if value < 100_000:
        return value * 1000
    return value


def _build_speed_fallbacks(speed_hz):
    fallbacks = [speed_hz]
    for candidate in (2_000_000, 1_000_000, 500_000, 200_000, 100_000, 50_000):
        if candidate < speed_hz and candidate not in fallbacks:
            fallbacks.append(candidate)
    return fallbacks


class DapLinkBackend(RttBackend):
    """DAPLink RTT 后端。"""

    DEFAULT_SEARCH_START = 0x20000000
    DEFAULT_SEARCH_RANGE = 0x100000

    def __init__(self):
        self._dev = CmsisDapProtocol()
        self._lock = threading.RLock()
        self._connected = False
        self._device_id = ''
        self._rtt_cb_addr = 0
        self._max_up = 0
        self._max_down = 0
        self.settings = {}

    def update_settings(self, settings):
        self.settings.update(settings)

    @classmethod
    def _enumerate_raw_devices(cls):
        import hid

        devices = []
        known_vids = get_known_dap_vendor_ids()
        for info in hid.enumerate():
            vid = info.get('vendor_id', 0)
            pid = info.get('product_id', 0)
            usage_page = info.get('usage_page', 0)
            usage = info.get('usage', 0)
            path = info.get('path')
            product = info.get('product_string', '') or 'CMSIS-DAP'
            if not path:
                continue
            if not is_dap_device(vid, pid):
                continue
            if usage_page not in (0xFF00, 0x0001) and vid not in known_vids:
                continue
            if usage_page == 0xFF00 and usage not in (0, 1):
                continue

            serial = info.get('serial_number', '') or ''
            device_id = _normalize_device_id(serial, vid, pid, path)
            devices.append({
                'vid': vid,
                'pid': pid,
                'path': path,
                'serial': serial,
                'product': product,
                'device_id': device_id,
                'is_cmsis_dap': 'cmsis-dap' in product.lower(),
            })
        return devices

    @classmethod
    def get_dap_devices(cls):
        try:
            seen = {}
            for item in cls._enumerate_raw_devices():
                current = seen.get(item['device_id'])
                if current is None or (item['is_cmsis_dap'] and not current['is_cmsis_dap']):
                    seen[item['device_id']] = item
            devices = []
            for item in seen.values():
                product = item['product'] or 'CMSIS-DAP'
                devices.append((item['device_id'], f"{product} (ID={item['device_id']})"))
            return devices
        except ImportError:
            return []
        except Exception:
            return []

    def get_available_devices(self):
        return self.get_dap_devices()

    def _select_device(self, device_id=None):
        candidates = self._enumerate_raw_devices()
        if device_id:
            candidates = [item for item in candidates if item['device_id'] == device_id]
        if not candidates:
            raise IOError("未找到 CMSIS-DAP 设备")
        candidates.sort(key=lambda item: (0 if item['is_cmsis_dap'] else 1, item['product']))
        return candidates[0]

    def connect(self, device_id=None, chip=None, speed=None, reset_flag=None,
                start_address=None, range_size=None):
        with self._lock:
            if self._connected:
                self._disconnect_internal()

            target = self._select_device(device_id)
            if not self._dev.transport.open_path(target['path']):
                raise IOError("打开 CMSIS-DAP HID 设备失败")

            try:
                requested_speed = speed if speed is not None else self.settings.get('speed', 4000)
                speed_hz = _normalize_speed_hz(requested_speed)
                reset_flag = reset_flag if reset_flag is not None else self.settings.get('reset', False)
                start_address = _parse_optional_int(
                    start_address if start_address is not None else self.settings.get('start_address', ''),
                    self.DEFAULT_SEARCH_START,
                )
                range_size = _parse_optional_int(
                    range_size if range_size is not None else self.settings.get('range_size', ''),
                    self.DEFAULT_SEARCH_RANGE,
                )

                self._connect_with_fallback_speeds(reset_flag, speed_hz)
                self._rtt_cb_addr = self._poll_find_rtt_cb(start_address, range_size)
                if self._rtt_cb_addr == 0:
                    raise IOError("未找到 RTT 控制块 (SEGGER RTT)")

                self._max_up = self._read32(self._rtt_cb_addr + RTT_CB_MAX_UP)
                self._max_down = self._read32(self._rtt_cb_addr + RTT_CB_MAX_DOWN)
                self._device_id = target['device_id']
                self._connected = True
                return True
            except Exception:
                self._disconnect_internal()
                raise

    def _connect_with_fallback_speeds(self, reset_flag, speed_hz):
        last_error = None
        for candidate in _build_speed_fallbacks(speed_hz):
            try:
                self._dev.connect_swd(reset=reset_flag, speed_hz=candidate)
                self.settings['last_dap_speed_hz'] = candidate
                return candidate
            except Exception as exc:
                last_error = exc
                try:
                    self._dev.disconnect_swd()
                except Exception:
                    pass

        if last_error is not None:
            raise IOError(
                f"SWD 建链失败，已尝试速率: {', '.join(str(v) for v in _build_speed_fallbacks(speed_hz))} Hz; "
                f"最后错误: {last_error}"
            ) from last_error
        raise IOError("SWD 建链失败")

    def disconnect(self):
        with self._lock:
            return self._disconnect_internal()

    def _disconnect_internal(self):
        try:
            self._dev.disconnect_swd()
            self._dev.close()
        except Exception:
            pass
        self._connected = False
        self._device_id = ''
        self._rtt_cb_addr = 0
        self._max_up = 0
        self._max_down = 0
        return True

    def is_opened(self):
        return self._connected and self._dev.is_open()

    def get_serial_number(self):
        return self._device_id or 0

    def rtt_read(self, buffer_idx=0, read_size=8192):
        if not self._connected or buffer_idx >= self._max_up:
            return b''
        with self._lock:
            rb_addr = self._rtt_cb_addr + RTT_CB_AUP + buffer_idx * RTT_BUFFER_SIZE
            rb_data = self._read_mem_block32(rb_addr, 5)
            p_buffer = rb_data[0]
            size = rb_data[1]
            wr_off = rb_data[2]
            rd_off = rb_data[3]

            if p_buffer == 0 or size == 0 or wr_off == rd_off:
                return b''

            if rd_off < wr_off:
                count = min(wr_off - rd_off, read_size)
                data = self._read_rtt_mem(p_buffer + rd_off, count)
                new_rd_off = rd_off + count
            else:
                count1 = min(size - rd_off, read_size)
                data = self._read_rtt_mem(p_buffer + rd_off, count1)
                if len(data) < count1:
                    count1 = len(data)
                remaining = read_size - count1
                if remaining > 0 and wr_off > 0:
                    count2 = min(wr_off, remaining)
                    data += self._read_rtt_mem(p_buffer, count2)
                    new_rd_off = count2
                else:
                    new_rd_off = (rd_off + count1) % size

            self._write32(rb_addr + RB_RDOFF, new_rd_off)
            return bytes(data)

    def rtt_write(self, buffer_idx=0, data=None):
        if data is None or not self._connected or buffer_idx >= self._max_down:
            return 0
        with self._lock:
            rb_addr = self._rtt_cb_addr + RTT_CB_AUP + self._max_up * RTT_BUFFER_SIZE + buffer_idx * RTT_BUFFER_SIZE
            rb_data = self._read_mem_block32(rb_addr, 5)
            p_buffer = rb_data[0]
            size = rb_data[1]
            wr_off = rb_data[2]
            rd_off = rb_data[3]
            if p_buffer == 0 or size == 0:
                return 0

            if wr_off >= rd_off:
                available = size - (wr_off - rd_off) - 1
            else:
                available = rd_off - wr_off - 1

            payload = bytes(data) if not isinstance(data, bytes) else data
            count = min(len(payload), max(available, 0))
            if count <= 0:
                return 0

            to_write = payload[:count]
            if wr_off + count <= size:
                self._write_rtt_mem(p_buffer + wr_off, to_write)
                new_wr_off = wr_off + count
            else:
                first_len = size - wr_off
                self._write_rtt_mem(p_buffer + wr_off, to_write[:first_len])
                self._write_rtt_mem(p_buffer, to_write[first_len:])
                new_wr_off = (wr_off + count) % size

            self._write32(rb_addr + RB_WROFF, new_wr_off)
            return count

    def _poll_find_rtt_cb(self, start, length, timeout_sec=1.5):
        deadline = time.time() + max(0.5, timeout_sec)
        while time.time() < deadline:
            addr = self._find_rtt_cb(start, length)
            if addr:
                return addr
            time.sleep(0.1)
        return 0

    def _find_rtt_cb(self, start, length):
        step = 1024
        for offset in range(0, length, step):
            chunk_words = self._read_mem_block32(start + offset, step // 4)
            raw = b''.join(struct.pack('<I', word & 0xFFFFFFFF) for word in chunk_words)
            index = raw.find(b'SEGGER RTT')
            if index >= 0:
                return start + offset + index
        return 0

    def _read32(self, addr):
        return self._dev.read32(addr)

    def _write32(self, addr, value):
        self._dev.write32(addr, value)

    def _read_mem_block32(self, addr, count):
        return self._dev.read_mem_block32(addr, count)

    def _read_rtt_mem(self, addr, count):
        if count <= 0:
            return b''
        aligned_start = addr & ~0x3
        offset = addr - aligned_start
        aligned_count = (offset + count + 3) // 4
        words = self._dev.read_mem_block32(aligned_start, aligned_count)
        raw = b''.join(struct.pack('<I', word & 0xFFFFFFFF) for word in words)
        return raw[offset:offset + count]

    def _write_rtt_mem(self, addr, data):
        if not data:
            return
        aligned_start = addr & ~0x3
        offset = addr - aligned_start
        end = offset + len(data)
        aligned_count = (end + 3) // 4
        words = self._dev.read_mem_block32(aligned_start, aligned_count)
        raw = bytearray()
        for word in words:
            raw.extend(struct.pack('<I', word & 0xFFFFFFFF))
        raw[offset:end] = data
        new_words = []
        for pos in range(0, len(raw), 4):
            new_words.append(struct.unpack_from('<I', raw, pos)[0])
        self._dev.write_mem_block32(aligned_start, new_words)
