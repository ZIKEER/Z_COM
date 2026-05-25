"""Minimal AP test with error handling."""
import sys, os, struct, time
sys.path.insert(0, os.path.abspath('.'))

import hid
dev = hid.device()
for info in hid.enumerate(0xC251, 0xF00A):
    prod = info.get('product_string', '')
    if 'cmsis-dap' in prod.lower():
        dev.open_path(info['path'])
        break
dev.set_nonblocking(True)

def sr(cmd, label=""):
    import time
    dev.write(b'\x00' + bytes(cmd).ljust(64, b'\x00'))
    time.sleep(0.5)
    data = dev.read(65, timeout_ms=3000)
    if not data:
        print(f"  [{label}] TIMEOUT")
        return None
    return bytes(data)

def rd4(b, pos=3):
    return struct.unpack_from('<I', b, pos)[0]

print("=== SWD Init ===")
sr([0x02, 1], "Connect")
sr([0x12, 96] + [0xFF]*12, "SWJ96")
sr([0x12, 16] + [0x9E, 0xE7], "SWJsw")
sr([0x12, 96] + [0xFF]*12, "SWJ96")
sr([0x12, 8] + [0x00], "Idle")
sr([0x13, 0], "Cfg")

r = sr([0x05, 0, 1, 0x02], "IDCODE")
if r: print(f"DP_IDCODE = 0x{rd4(r):08X}")

r = sr([0x05, 0, 1, 0x06, 0x00, 0x00, 0x00, 0x50], "PowUp")
if r: print(f"PowUp status=0x{r[2]:02X}")

r = sr([0x05, 0, 1, 0x06], "CTRLrd")
if r: print(f"CTRL_STAT = 0x{rd4(r):08X}")

# DP_SELECT = 0
r = sr([0x05, 0, 1, 0x08, 0,0,0,0], "SEL")
if r: print(f"SELECT status=0x{r[2]:02X}")

# Write AP_CSW
csw = 0x23000012
r = sr([0x05, 0, 1, 0x01] + list(struct.pack('<I', csw)), "CSWwr")
if r: print(f"CSW write status=0x{r[2]:02X}")

# Read AP_CSW (AP read first, then RDBUFF)
r = sr([0x05, 0, 1, 0x03], "APrd")  # AP read reg 0
if r: print(f"AP rd (stale): 0x{rd4(r):08X} status=0x{r[2]:02X}")

r = sr([0x05, 0, 1, 0x0E], "RD")  # DP RDBUFF
if r: print(f"RDBUFF: 0x{rd4(r):08X} status=0x{r[2]:02X}")

print("=== Done ===")
dev.close()
