import struct
import threading
import time


DP_IDCODE = 0x00
DP_CTRL_STAT = 0x04
DP_SELECT = 0x08
DP_RDBUFF = 0x0C
DP_ABORT = 0x00

AP_CSW = 0x00
AP_TAR = 0x04
AP_DRW = 0x0C

CSW_8BIT = 0x23000000 | (0 << 4)
CSW_32BIT = 0x23000000 | (2 << 4)
CSW_ADDR_INC = 1

DAP_INFO = 0x00
DAP_CONNECT = 0x02
DAP_DISCONNECT = 0x03
DAP_TRANSFER_CONFIGURE = 0x04
DAP_TRANSFER = 0x05
DAP_TRANSFER_BLOCK = 0x06
DAP_SWJ_PINS = 0x10
DAP_SWJ_CLOCK = 0x11
DAP_SWJ_SEQUENCE = 0x12
DAP_SWD_CONFIGURE = 0x13

ACK_OK = 0x01
ACK_WAIT = 0x02
ACK_FAULT = 0x04
ACK_NO_ACK = 0x07

CTRLSTAT_CDBGPWRUPREQ = 1 << 28
CTRLSTAT_CDBGPWRUPACK = 1 << 29
CTRLSTAT_CSYSPWRUPREQ = 1 << 30
CTRLSTAT_CSYSPWRUPACK = 1 << 31

ABORT_STKCMPCLR = 1 << 1
ABORT_STKERRCLR = 1 << 2
ABORT_WDERRCLR = 1 << 3
ABORT_ORUNERRCLR = 1 << 4

SWD_SWITCH_SEQUENCE = b'\x9E\xE7'


def encode_request_byte(apndp, rnw, reg_addr):
    return (apndp & 1) | ((rnw & 1) << 1) | (((reg_addr >> 2) & 0x3) << 2)


def _bytes_to_words(data):
    values = []
    for offset in range(0, len(data), 4):
        values.append(struct.unpack_from('<I', data, offset)[0])
    return values


class HidApiTransport:
    """CMSIS-DAP v1 HID 传输层。"""

    PACKET_SIZE = 64

    def __init__(self):
        self._dev = None
        self._hid = None

    def _import_hid(self):
        if self._hid is not None:
            return self._hid
        try:
            import hid
            self._hid = hid
            return hid
        except ImportError:
            return None

    @staticmethod
    def enumerate(vid=0, pid=0):
        try:
            import hid
            return hid.enumerate(vid, pid)
        except ImportError:
            return []

    def open_path(self, path):
        hid = self._import_hid()
        if hid is None:
            raise ImportError("hidapi 库未安装")
        self._dev = hid.device()
        try:
            self._dev.open_path(path)
            self._dev.set_nonblocking(True)
            return True
        except Exception:
            self._dev = None
            return False

    def write(self, data):
        if self._dev is None:
            raise IOError("HID 设备未打开")
        payload = bytes(data)
        payload = payload[:self.PACKET_SIZE]
        buf = b'\x00' + payload.ljust(self.PACKET_SIZE, b'\x00')
        self._dev.write(buf)

    def read(self, timeout_ms=500):
        if self._dev is None:
            raise IOError("HID 设备未打开")
        deadline = time.time() + max(timeout_ms, 1) / 1000.0
        while time.time() < deadline:
            buf = self._dev.read(self.PACKET_SIZE + 1, timeout_ms=50)
            if buf:
                return bytes(buf)
            time.sleep(0.002)
        return None

    def close(self):
        if self._dev is not None:
            try:
                self._dev.close()
            except Exception:
                pass
            self._dev = None

    @property
    def is_open(self):
        return self._dev is not None


class CmsisDapProtocol:
    """CMSIS-DAP 协议 + SWD 内存访问。"""

    def __init__(self):
        self.transport = HidApiTransport()
        self._lock = threading.RLock()
        self._timeout = 500
        self._retry_count = 100

    def close(self):
        with self._lock:
            self.transport.close()

    def is_open(self):
        return self.transport.is_open

    def _send_cmd(self, payload, timeout=None):
        with self._lock:
            self.transport.write(payload)
            resp = self.transport.read(timeout_ms=timeout or self._timeout)
            if resp is None:
                raise TimeoutError("CMSIS-DAP 响应超时")
            if not resp:
                raise IOError("CMSIS-DAP 返回空响应")
            if resp[0] != payload[0]:
                raise IOError(
                    f"CMSIS-DAP 响应命令不匹配: send=0x{payload[0]:02X}, recv=0x{resp[0]:02X}"
                )
            return resp

    def dap_info(self, info_id):
        resp = self._send_cmd([DAP_INFO, info_id])
        if len(resp) < 2:
            return b''
        length = resp[1]
        return bytes(resp[2:2 + length])

    def dap_connect(self, port=1):
        resp = self._send_cmd([DAP_CONNECT, port])
        return len(resp) >= 2 and resp[1] == port

    def dap_disconnect(self):
        resp = self._send_cmd([DAP_DISCONNECT])
        return len(resp) >= 2 and resp[1] == 0

    def dap_transfer_configure(self, idle=0, retry=100, match_retry=0):
        payload = [DAP_TRANSFER_CONFIGURE] + list(struct.pack('<BHB', idle, retry, match_retry))
        resp = self._send_cmd(payload)
        self._retry_count = retry
        return len(resp) >= 2 and resp[1] == 0

    def dap_swj_clock(self, speed_hz):
        resp = self._send_cmd([DAP_SWJ_CLOCK] + list(struct.pack('<I', int(speed_hz))))
        return len(resp) >= 2 and resp[1] == 0

    def dap_swj_pins(self, value, select, wait_us=0):
        payload = [DAP_SWJ_PINS] + list(struct.pack('<BBI', value, select, wait_us))
        resp = self._send_cmd(payload)
        return len(resp) >= 2

    def dap_swj_sequence(self, data):
        payload = [DAP_SWJ_SEQUENCE, len(data) * 8] + list(data)
        resp = self._send_cmd(payload)
        return len(resp) >= 2 and resp[1] == 0

    def dap_swd_configure(self, cfg=0):
        resp = self._send_cmd([DAP_SWD_CONFIGURE, cfg])
        return len(resp) >= 2 and resp[1] == 0

    def dap_reset_target(self):
        self.dap_swj_pins(0x00, 0x80, 50_000)
        time.sleep(0.05)
        self.dap_swj_pins(0x80, 0x80, 100_000)
        time.sleep(0.1)
        return True

    def _parse_transfer_response(self, resp):
        if len(resp) < 3:
            raise IOError("DAP_Transfer 响应过短")
        transfer_count = resp[1]
        ack = resp[2]
        return transfer_count, ack, bytes(resp[3:])

    def _require_ack_ok(self, ack, op_name):
        ack_bits = ack & 0x07
        protocol_error = bool(ack & 0x08)
        value_mismatch = bool(ack & 0x10)

        if ack_bits == ACK_OK and not protocol_error and not value_mismatch:
            return

        ack_name = {
            ACK_WAIT: "WAIT",
            ACK_FAULT: "FAULT",
            ACK_NO_ACK: "NO_ACK",
        }.get(ack_bits, f"UNKNOWN(0x{ack_bits:02X})")

        details = [f"ACK={ack_name}"]
        if protocol_error:
            details.append("ProtocolError")
        if value_mismatch:
            details.append("ValueMismatch")

        raise IOError(f"{op_name} 失败: raw=0x{ack:02X}, {'/'.join(details)}")

    def _dap_transfer(self, request_bytes, write_data=None):
        payload = [DAP_TRANSFER, 0, len(request_bytes)] + list(request_bytes)
        for value in write_data or []:
            payload.extend(struct.pack('<I', value & 0xFFFFFFFF))
        resp = self._send_cmd(payload)
        transfer_count, ack, data = self._parse_transfer_response(resp)
        self._require_ack_ok(ack, "DAP_Transfer")
        if transfer_count != len(request_bytes):
            raise IOError(
                f"DAP_Transfer 实际完成 {transfer_count} 次, 期望 {len(request_bytes)} 次"
            )
        return data

    def _dap_transfer_block_read(self, count, request_byte):
        payload = [DAP_TRANSFER_BLOCK, 0] + list(struct.pack('<H', count)) + [request_byte]
        resp = self._send_cmd(payload)
        if len(resp) < 4:
            raise IOError("DAP_TransferBlock 响应过短")
        actual_count = struct.unpack_from('<H', resp, 1)[0]
        ack = resp[3]
        self._require_ack_ok(ack, "DAP_TransferBlock")
        if actual_count != count:
            raise IOError(
                f"DAP_TransferBlock 实际完成 {actual_count} 次, 期望 {count} 次"
            )
        data = bytes(resp[4:4 + count * 4])
        if len(data) < count * 4:
            raise IOError("DAP_TransferBlock 读数据长度不足")
        return _bytes_to_words(data)

    def _dap_transfer_block_write(self, request_byte, values):
        payload = [DAP_TRANSFER_BLOCK, 0] + list(struct.pack('<H', len(values))) + [request_byte]
        for value in values:
            payload.extend(struct.pack('<I', value & 0xFFFFFFFF))
        resp = self._send_cmd(payload)
        if len(resp) < 4:
            raise IOError("DAP_TransferBlock 写响应过短")
        actual_count = struct.unpack_from('<H', resp, 1)[0]
        ack = resp[3]
        self._require_ack_ok(ack, "DAP_TransferBlock")
        if actual_count != len(values):
            raise IOError(
                f"DAP_TransferBlock 实际写入 {actual_count} 次, 期望 {len(values)} 次"
            )

    def swd_read_dp(self, reg):
        req = encode_request_byte(0, 1, reg)
        data = self._dap_transfer([req])
        if len(data) < 4:
            raise IOError("DP 读取结果长度不足")
        return struct.unpack_from('<I', data, 0)[0]

    def swd_write_dp(self, reg, value):
        req = encode_request_byte(0, 0, reg)
        self._dap_transfer([req], [value])

    def swd_read_ap(self, reg):
        req_ap = encode_request_byte(1, 1, reg)
        req_rd = encode_request_byte(0, 1, DP_RDBUFF)
        data = self._dap_transfer([req_ap, req_rd])
        if len(data) < 8:
            raise IOError("AP 读取结果长度不足")
        return struct.unpack_from('<I', data, 4)[0]

    def swd_write_ap(self, reg, value):
        req = encode_request_byte(1, 0, reg)
        self._dap_transfer([req], [value])

    def ap_select(self, ap_num=0):
        self.swd_write_dp(DP_SELECT, ap_num << 24)

    def _ap_setup(self, csw=CSW_32BIT | CSW_ADDR_INC):
        self.ap_select(0)
        self.swd_write_ap(AP_CSW, csw)

    def clear_sticky_errors(self):
        self.swd_write_dp(DP_ABORT, ABORT_STKCMPCLR | ABORT_STKERRCLR | ABORT_WDERRCLR | ABORT_ORUNERRCLR)

    def power_up_debug(self, timeout_sec=0.5):
        request = CTRLSTAT_CDBGPWRUPREQ | CTRLSTAT_CSYSPWRUPREQ
        self.swd_write_dp(DP_CTRL_STAT, request)
        deadline = time.time() + timeout_sec
        while time.time() < deadline:
            ctrl = self.swd_read_dp(DP_CTRL_STAT)
            if (ctrl & CTRLSTAT_CDBGPWRUPACK) and (ctrl & CTRLSTAT_CSYSPWRUPACK):
                return True
            time.sleep(0.01)
        raise IOError("目标未响应 Debug/System PowerUp ACK")

    def write32(self, addr, value):
        self._ap_setup()
        self.swd_write_ap(AP_TAR, addr)
        self.swd_write_ap(AP_DRW, value)

    def read32(self, addr):
        self._ap_setup()
        self.swd_write_ap(AP_TAR, addr)
        return self.swd_read_ap(AP_DRW)

    def read_mem8(self, addr, count):
        result = []
        for offset in range(count):
            aligned = self.read32((addr + offset) & ~0x3)
            shift = ((addr + offset) & 0x3) * 8
            result.append((aligned >> shift) & 0xFF)
        return result

    def read_mem_block32(self, addr, count):
        if count <= 0:
            return []
        self._ap_setup()
        self.swd_write_ap(AP_TAR, addr)
        values = self._dap_transfer_block_read(count + 1, encode_request_byte(1, 1, AP_DRW))
        if len(values) < count + 1:
            raise IOError("块读返回数据不足")
        return values[1:count + 1]

    def write_mem_block32(self, addr, values):
        if not values:
            return
        self._ap_setup()
        self.swd_write_ap(AP_TAR, addr)
        self._dap_transfer_block_write(encode_request_byte(1, 0, AP_DRW), values)

    def _swd_init(self, reset=False):
        self.dap_swj_sequence(b'\xFF' * 7)
        self.dap_swj_sequence(SWD_SWITCH_SEQUENCE)
        self.dap_swj_sequence(b'\xFF' * 7)
        self.dap_swj_sequence(b'\x00')
        if reset:
            self.dap_reset_target()
            self.dap_swj_sequence(b'\xFF' * 7)
        if not self.dap_swd_configure(0):
            raise IOError("DAP_SWD_Configure 失败")
        if not self.dap_transfer_configure(0, self._retry_count, 0):
            raise IOError("DAP_TransferConfigure 失败")
        self.clear_sticky_errors()
        self.swd_read_dp(DP_IDCODE)
        self.power_up_debug()
        self._ap_setup()

    def connect_swd(self, reset=False, speed_hz=None):
        with self._lock:
            self._timeout = 500
            if not self.dap_connect(1):
                raise IOError("CMSIS-DAP 连接失败")
            if speed_hz:
                self.dap_swj_clock(speed_hz)
            self._swd_init(reset=reset)
            self._timeout = 2000
            return True

    def disconnect_swd(self):
        with self._lock:
            try:
                self.dap_disconnect()
            except Exception:
                pass
