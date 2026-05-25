"""CMSIS-DAP debug power-up and AP access."""
import hid, time, struct

info = None
for d in hid.enumerate():
    if d.get('vendor_id') == 0xC251 and d.get('product_id') == 0xF00A:
        if d.get('usage_page') == 0xFF00 and d.get('usage') == 0x01 and 'CMSIS-DAP' in d.get('product_string',''):
            info = d
            break

dev = hid.device()
dev.open_path(info['path'])
dev.set_nonblocking(True)

def xfer(cmd_id, payload=b''):
    data = bytes([cmd_id]) + payload
    tx = b'\x00' + data + b'\x00' * (64 - len(data))
    dev.write(tx)
    rx = dev.read(65, timeout_ms=500)
    return bytes(rx) if rx else None

def send(cmd_id, payload=b''):
    resp = xfer(cmd_id, payload)
    return None if not resp or resp[0] != cmd_id else resp[1:]

DP_RnW = 1 << 1
APnDP = 1
def dp_read(a):  return (a >> 2) << 2 | DP_RnW
def dp_write(a): return (a >> 2) << 2
def ap_read(a):  return ((a >> 2) << 2) | APnDP | DP_RnW
def ap_write(a): return ((a >> 2) << 2) | APnDP

# Init
for cmd_id, payload in [
    (0x02, bytes([1])),           # Connect SWD
    (0x13, bytes([0])),           # SWD Configure
    (0x04, struct.pack('<BHB', 0, 200, 0)),  # TransferConfigure
    (0x11, struct.pack('<I', 1000000)),      # Clock 1MHz
]:
    send(cmd_id, payload)

# SWJ init
for payload in [
    bytes([51]) + b'\xff' * 7,    # Line reset (51 clocks)
    bytes([16]) + b'\x9e\xe7',    # JTAG-to-SWD
    bytes([51]) + b'\xff' * 7,    # Line reset #2
    bytes([8]) + b'\x00',         # Idle cycle
]:
    send(0x12, payload)

print("=== SWD Initialized ===")

# Read DP_IDCODE
resp = xfer(0x05, bytes([0, 1, dp_read(0x00)]))
if resp:
    idcode = struct.unpack('<I', resp[3:7])[0]
    print(f"DP_IDCODE   = 0x{idcode:08X}")
else:
    print("Failed to read DP_IDCODE")

# Read DP_CTRL_STAT
resp = xfer(0x05, bytes([0, 1, dp_read(0x04)]))
if resp:
    ctrl = struct.unpack('<I', resp[3:7])[0]
    print(f"DP_CTRL_STAT = 0x{ctrl:08X}")
    print(f"  CDBGPWRUPACK={(ctrl>>30)&1} CSYSPWRUPACK={(ctrl>>28)&1}")

# Write DP_CTRL_STAT to request power-up
CTRL_REQ = (1 << 28) | (1 << 30)  # CSYSPWRUPREQ + CDBGPWRUPREQ
print(f"\n=== Write DP_CTRL_STAT = 0x{CTRL_REQ:08X} ===")
resp = xfer(0x05, bytes([0, 1, dp_write(0x04)]) + struct.pack('<I', CTRL_REQ))
if resp:
    print(f"  Write: cnt={resp[1]} ack={resp[2]:#04x}")

# Read back DP_CTRL_STAT
time.sleep(0.01)
resp = xfer(0x05, bytes([0, 1, dp_read(0x04)]))
if resp:
    ctrl = struct.unpack('<I', resp[3:7])[0]
    print(f"DP_CTRL_STAT = 0x{ctrl:08X}")
    print(f"  CDBGPWRUPACK={(ctrl>>30)&1} CSYSPWRUPACK={(ctrl>>28)&1}")

# If power-up restored, read AP registers
if resp:
    ctrl = struct.unpack('<I', resp[3:7])[0]
    cdbgp = (ctrl >> 30) & 1
    csysp = (ctrl >> 28) & 1
    if cdbgp and csysp:
        print("\n=== Power-up OK, reading AP ===")
        
        # First set APBANKSEL for AP_CSW (register 0x00, APBANKSEL=0)
        print("\n  Write DP_SELECT (APSEL=0, APBANKSEL=0)")
        send(0x05, bytes([0, 1, dp_write(0x08)]) + struct.pack('<I', 0))
        
        # Read AP_CSW
        resp = xfer(0x05, bytes([0, 1, ap_read(0x00)]))
        if resp:
            cnt, ack = resp[1], resp[2]
            ap_data = struct.unpack('<I', resp[3:7])[0] if len(resp) >= 7 else 0
            print(f"  AP_CSW: cnt={cnt} ack={ack:#04x} value=0x{ap_data:08X}")
        
        # Read AP_IDR
        resp = xfer(0x05, bytes([0, 1, ap_read(0x0C)]))
        if resp:
            cnt, ack = resp[1], resp[2]
            ap_data = struct.unpack('<I', resp[3:7])[0] if len(resp) >= 7 else 0
            print(f"  AP_IDR: cnt={cnt} ack={ack:#04x} value=0x{ap_data:08X}")
    else:
        print(f"\n  Power-up NOT confirmed (CDBGPWRUPACK={cdbgp}, CSYSPWRUPACK={csysp})")

dev.close()
print("\nDone")
