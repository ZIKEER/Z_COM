"""
CMSIS-DAP v1 over HID - H7TOOL 调试脚本 v2
基于固件源码分析:
  - DAP_PACKET_SIZE=64, 无 Report ID
  - 响应长度由 DAP_ExecuteCommand 返回值低16位决定
  - DAP_SWD_Transfer 的 request 字节编码: bit0=APnDP, bit1=RnW, bits[3:2]=A[3:2]
  - SWJ_Sequence 发送位时 LSB优先
  - HID 用 hidapi (pip install hidapi)
"""

import sys, os, time, struct

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

# CMSIS-DAP v1 command IDs (DAP.h)
ID_DAP_Info                = 0x00
ID_DAP_Connect             = 0x02
ID_DAP_Disconnect          = 0x03
ID_DAP_TransferConfigure   = 0x04
ID_DAP_Transfer            = 0x05
ID_DAP_TransferBlock       = 0x06
ID_DAP_SWJ_Pins            = 0x10
ID_DAP_SWJ_Clock           = 0x11
ID_DAP_SWJ_Sequence        = 0x12
ID_DAP_SWD_Configure       = 0x13

# SWD request bits
DPnAP  = 0  # bit 0: 0=DP
APnDP  = 1  # bit 0: 1=AP
RnW    = 2  # bit 1: 0=write, 1=read
A_BITS = (1<<2)|(1<<3)  # bits 3:2 = A[3:2]

def swd_req(dp_ap, rnw, addr):
    """addr: A[3:2] value, e.g. 0x00, 0x04, 0x08, 0x0C"""
    return dp_ap | rnw | ((addr >> 2) << 2)

def dp_read(addr):   return swd_req(DPnAP, RnW, addr)
def dp_write(addr):  return swd_req(DPnAP, 0, addr)
def ap_read(addr):   return swd_req(APnDP, RnW, addr)
def ap_write(addr):  return swd_req(APnDP, 0, addr)


def find_h7tool_dev():
    """Find H7TOOL HID device by VID/PID."""
    import hid
    for dev in hid.enumerate():
        if dev.get('vendor_id') == 0xC251 and dev.get('product_id') == 0xF00A:
            if dev.get('usage_page') == 0xFF00 and dev.get('usage') == 0x01:
                if 'CMSIS-DAP' in dev.get('product_string', ''):
                    return dev['path']
    return None


class CMSISDAPv1:
    """CMSIS-DAP v1 protocol driver over HID."""

    PACKET_SIZE = 64  # DAP_PACKET_SIZE from H7TOOL firmware

    def __init__(self):
        self.dev = None

    def open(self):
        import hid
        path = find_h7tool_dev()
        if not path:
            raise RuntimeError("H7TOOL HID device not found")
        self.dev = hid.device()
        self.dev.open_path(path)
        self.dev.set_nonblocking(True)
        print(f"[DAP] Opened H7TOOL HID device")

    def close(self):
        if self.dev:
            self.dev.close()
            self.dev = None

    def _xfer(self, data):
        """Send OUT report, read IN report.
        
        Windows HID: Write 65 bytes [0x00 cmd data... zeros to 64].
        Read 65 bytes, first byte is the CMSIS-DAP command echo.
        """
        pad = self.PACKET_SIZE - len(data)
        buf = b'\x00' + bytes(data) + b'\x00' * pad  # 65 bytes
        self.dev.write(buf)
        
        for _ in range(300):
            resp = self.dev.read(self.PACKET_SIZE + 1, timeout_ms=100)
            if resp:
                return bytes(resp)
            time.sleep(0.002)
        
        raise TimeoutError("No response from device")

    def _send_cmd(self, cmd_id, payload=b''):
        """Send CMSIS-DAP v1 command, return raw response."""
        req = bytes([cmd_id]) + payload
        resp = self._xfer(req)
        if len(resp) < 1 or resp[0] == 0xFF:
            print(f"  [ERR] Cmd {cmd_id:#04x} invalid")
        elif resp[0] != cmd_id:
            print(f"  [WARN] Echo mismatch: sent {cmd_id:#04x}, got {resp[0]:#04x}")
        return resp

    # ---- Commands ----
    def dap_info(self, info_id):
        resp = self._send_cmd(ID_DAP_Info, bytes([info_id]))
        if len(resp) >= 2:
            length = resp[1]
            return resp[2:2+length]
        return b''

    def dap_connect(self, port=1):
        resp = self._send_cmd(ID_DAP_Connect, bytes([port]))
        return resp[1] if len(resp) >= 2 else None

    def dap_disconnect(self):
        resp = self._send_cmd(ID_DAP_Disconnect)
        return resp[1] if len(resp) >= 2 else None

    def dap_swd_configure(self, config=0):
        resp = self._send_cmd(ID_DAP_SWD_Configure, bytes([config]))
        return resp[1] if len(resp) >= 2 else None

    def dap_transfer_configure(self, idle=0, retry=100, match_retry=0):
        payload = struct.pack('<BHB', idle, retry, match_retry)
        resp = self._send_cmd(ID_DAP_TransferConfigure, payload)
        return resp[1] if len(resp) >= 2 else None

    def dap_swj_clock(self, freq=5000000):
        resp = self._send_cmd(ID_DAP_SWJ_Clock, struct.pack('<I', freq))
        return resp[1] if len(resp) >= 2 else None

    def dap_swj_pins(self, value, select, wait_us=0):
        """value: pin levels (bit7=nRESET, bit1=SWDIO, bit0=SWCLK)
           select: which pins to change (same bit mask)"""
        payload = struct.pack('<BBI', value, select, wait_us)
        resp = self._send_cmd(ID_DAP_SWJ_Pins, payload)
        return resp[1] if len(resp) >= 2 else None

    def dap_swj_sequence(self, count, data):
        """Send bit sequence LSB-first per byte."""
        payload = bytes([count]) + bytes(data)
        resp = self._send_cmd(ID_DAP_SWJ_Sequence, payload)
        return resp[1] if len(resp) >= 2 else None

    def dap_transfer(self, dap_index, requests):
        """requests: list of (request_byte, write_data_or_None)"""
        payload = bytearray([dap_index, len(requests)])
        for req, wdata in requests:
            payload.append(req)
            if wdata is not None:
                payload += struct.pack('<I', wdata)
        resp = self._send_cmd(ID_DAP_Transfer, bytes(payload))
        
        # Parse response: [0]=echo, [1]=response_count, [2]=response_value, [3..]=data
        if len(resp) < 3:
            return (0, 0xFF, b'')
        cnt = resp[1]
        ack = resp[2]
        data = resp[3:]
        return (cnt, ack, data)

    def dap_transfer_block(self, dap_index, request_byte, write_data_list):
        """DAP_TransferBlock for bulk reads/writes.
        write_data_list: list of 32-bit values for writes, None for reads."""
        payload = bytearray([dap_index])
        if write_data_list is None:
            # Block read
            payload += struct.pack('<H', 1)  # count
            payload += bytes([request_byte])
        else:
            payload += struct.pack('<H', len(write_data_list))
            payload += bytes([request_byte])
            for val in write_data_list:
                payload += struct.pack('<I', val)
        resp = self._send_cmd(ID_DAP_TransferBlock, bytes(payload))
        
        if len(resp) < 3:
            return (0, 0xFF, b'')
        cnt = resp[1] | (resp[2] << 8) if len(resp) >= 3 else 0
        ack = resp[3] if len(resp) >= 4 else 0xFF
        data = resp[4:]
        return (cnt, ack, data)

    # ---- SWD Init Sequences ----
    def swd_line_reset(self):
        # 51 clocks SWDIO=1
        return self.dap_swj_sequence(51, [0xFF] * 7)

    def swd_jtag_to_swd(self):
        # 0xE79E MSB-first -> bytes [0x9E, 0xE7] sent LSB-first per byte
        return self.dap_swj_sequence(16, [0x9E, 0xE7])

    def swd_idle_cycle(self):
        # 8 clocks SWDIO=0
        return self.dap_swj_sequence(8, [0x00])

    def swd_init_full(self):
        """Full SWD init per ARM Debug Interface v5."""
        print("\n[SWD] Line Reset (51 clocks high)")
        self.swd_line_reset()
        print("[SWD] JTAG-to-SWD switch (0xE79E)")
        self.swd_jtag_to_swd()
        print("[SWD] Line Reset #2")
        self.swd_line_reset()
        print("[SWD] Idle Cycle (8 clocks low)")
        self.swd_idle_cycle()


def try_read_dp(dap, label, addr):
    """Try DAP_Transfer to read a DP register."""
    cnt, ack, data = dap.dap_transfer(0, [(dp_read(addr), None)])
    if cnt > 0 and len(data) >= 4:
        val = struct.unpack('<I', data[:4])[0]
        print(f"  {label} = 0x{val:08X}  (cnt={cnt}, ack={ack:#04x})")
        return val
    else:
        print(f"  {label}: FAILED (cnt={cnt}, ack={ack:#04x}, data={data.hex()})")
        return None


def try_read_ap(dap, label, addr, ap_sel=0):
    """Try DAP_Transfer to read an AP register (needs DP_SELECT first)."""
    # Write DP_SELECT for AP selection
    dap.dap_transfer(0, [(dp_write(DP_SELECT), ap_sel)])
    cnt, ack, data = dap.dap_transfer(0, [(ap_read(addr), None)])
    if cnt > 0 and len(data) >= 4:
        val = struct.unpack('<I', data[:4])[0]
        print(f"  {label} = 0x{val:08X}  (cnt={cnt}, ack={ack:#04x})")
        return val
    else:
        # Post-read: read DP_RDBUFF
        cnt2, ack2, data2 = dap.dap_transfer(0, [(dp_read(DP_RDBUFF), None)])
        if cnt2 > 0 and len(data2) >= 4:
            val = struct.unpack('<I', data2[:4])[0]
            print(f"  {label} = 0x{val:08X}  (via RDBUFF: cnt={cnt2}, ack={ack2:#04x})")
            return val
        print(f"  {label}: FAILED (cnt={cnt}, ack={ack:#04x})")
        return None


def main():
    dap = CMSISDAPv1()
    
    try:
        dap.open()
        print()
        
        # 1. DAP_Info
        print("=== 1. DAP_Info ===")
        for iid, name in [(0xF0, "Capabilities"), (0xFE, "PacketCount"),
                          (0xFF, "PacketSize"), (4, "FW_Ver")]:
            d = dap.dap_info(iid)
            print(f"  {name}: {d.hex()}")
        
        # 2. DAP_Connect
        print("\n=== 2. DAP_Connect (SWD) ===")
        port = dap.dap_connect(1)
        print(f"  Connected: port={port}")
        
        # 3. SWD Configure
        print("\n=== 3. DAP_SWD_Configure ===")
        r = dap.dap_swd_configure(0)
        print(f"  => {r}")
        
        # 4. Transfer Configure
        print("\n=== 4. DAP_TransferConfigure ===")
        r = dap.dap_transfer_configure(0, 200, 0)
        print(f"  => {r}")
        
        # 5. Set clock: 1MHz
        print("\n=== 5. DAP_SWJ_Clock (1MHz) ===")
        r = dap.dap_swj_clock(1_000_000)
        print(f"  => {r}")
        
        # 6. nRESET assert + release
        nRST = 1 << 7
        print("\n=== 6. nRESET: assert ===")
        r = dap.dap_swj_pins(0, nRST, 50_000)
        print(f"  Low => {r}")
        print("  ...wait 50ms")
        time.sleep(0.05)
        r = dap.dap_swj_pins(nRST, nRST, 100_000)
        print(f"  High => {r}")
        print("  ...wait 100ms")
        time.sleep(0.1)
        
        # 7. SWD init sequence
        print("\n=== 7. SWD Init Sequence ===")
        dap.swd_init_full()
        
        # 8. Read DP_IDCODE
        print("\n=== 8. DP_IDCODE ===")
        idcode = try_read_dp(dap, "DP_IDCODE", 0x00)
        
        # 9. Read DP_CTRL_STAT
        print("\n=== 9. DP_CTRL_STAT ===")
        ctrl = try_read_dp(dap, "DP_CTRL_STAT", 0x04)
        
        # 10. If DP_IDCODE worked, try to initialize debug
        if idcode is not None:
            print(f"\n=== 10. Target detected: IDCODE=0x{idcode:08X} ===")
            
            # Write DP_CTRL_STAT to enable debug
            # TRNNORN=0, STICKYERR=0, STICKYCMP=0, STICKYORUN=0
            # CSYSPWRUPREQ=1, CDBGPWRUPREQ=1
            CTRL_STAT_REQ = (1 << 28) | (1 << 30)
            print(f"  Write DP_CTRL_STAT = 0x{CTRL_STAT_REQ:08X} (CSYSPWRUPREQ+CDBGPWRUPREQ)")
            cnt, ack, _ = dap.dap_transfer(0, [(dp_write(0x04), CTRL_STAT_REQ)])
            print(f"  Write result: cnt={cnt}, ack={ack:#04x}")
            
            # Read back
            time.sleep(0.01)
            ctrl2 = try_read_dp(dap, "DP_CTRL_STAT (re-read)", 0x04)
            
            # Check power-up status
            if ctrl2 is not None:
                cdbgup = (ctrl2 >> 30) & 1
                csysup = (ctrl2 >> 28) & 1
                print(f"  CDBGPWRUPACK={cdbgup}, CSYSPWRUPACK={csysup}")
                
                if cdbgup and csysup:
                    # Try reading AP reg
                    print("\n=== 11. AP register reads ===")
                    try_read_ap(dap, "AP_CSW (AHB-AP)", 0x00, 0x00000000)
                    try_read_ap(dap, "AP_IDR", 0x0C, 0x00000000)
        else:
            print("\n=== FAILED: No SWD response from target ===")
            print("Possible causes:")
            print("  1. Target not powered or not connected")
            print("  2. H7TOOL needs 'Exit'(退出) from DAP-Link mode first, then select SWD")
            print("  3. H7TOOL firmware not in CMSIS-DAP mode (check OLED display)")
            print("  4. nRESET not properly released")
            print("  5. Target SWD lines have cross-chip interference")
        
    except Exception as e:
        import traceback
        traceback.print_exc()
    finally:
        dap.close()
        print("\n[DAP] Closed")


def parse_resp_hex(data):
    """Parse and display hex dump of raw response."""
    print("  Raw:", ' '.join(f'{b:02x}' for b in data))


if __name__ == '__main__':
    main()
