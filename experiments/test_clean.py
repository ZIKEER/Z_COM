"""Clean AP test - separate DP vs AP reads properly."""
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

def sr(cmd):
    dev.write(b'\x00' + bytes(cmd).ljust(64, b'\x00'))
    time.sleep(0.3)
    data = dev.read(65, timeout_ms=3000)
    return bytes(data) if data else None
def rd4(b, pos=3):
    return struct.unpack_from('<I', b, pos)[0]

E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

# Fresh init
sr([0x02, 1])
sr([0x12, 96] + [0xFF]*12)
sr([0x12, 16] + [0x9E, 0xE7])
sr([0x12, 96] + [0xFF]*12)
sr([0x12, 8] + [0x00])
sr([0x13, 0])

# === DP reads return data directly in DAP_Transfer response ===
r = sr([0x05, 0, 1, E(0,1,0)])  # DP read IDCODE
print(f"DP_IDCODE  = 0x{rd4(r):08X}  (direct)")

# Power up
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
r = sr([0x05, 0, 1, E(0,1,1)])   # DP read CTRL_STAT
print(f"CTRL_STAT = 0x{rd4(r):08X} (direct)")

# DP_SELECT = 0
sr([0x05, 0, 1, E(0,0,2), 0,0,0,0])

# === Write correct CSW ===
CSW_CORRECT = 0x23000000 | (2 << 4) | 1
sr([0x05, 0, 1, E(1,0,0)] + list(struct.pack('<I', CSW_CORRECT)))

# === AP read via pipeline: AP read (ignored) + RDBUFF (actual data) ===
sr([0x05, 0, 1, E(1,1,0)])  # AP read CSW (initiates AP read)
r = sr([0x05, 0, 1, E(0,1,3)])  # DP read RDBUFF (gets AP result)
if r:
    csw = rd4(r)
    print(f"AP_CSW     = 0x{csw:08X} (via RDBUFF)")
    print(f"  Expected 0x{CSW_CORRECT:08X}, match={csw == CSW_CORRECT}")

# === Try reading Flash memory ===
sr([0x05, 0, 1, E(1,0,1)] + list(struct.pack('<I', 0x08000000)))  # AP write TAR
sr([0x05, 0, 1, E(1,1,3)])  # AP read DRW (initiates)
r = sr([0x05, 0, 1, E(0,1,3)])  # DP read RDBUFF
if r:
    print(f"Mem[0x08000000] = 0x{rd4(r):08X} (via RDBUFF)")

# === Read AP_IDR (bank 3) ===
sr([0x05, 0, 1, E(0,0,2), 3, 0, 0, 0])  # DP_SELECT (APBANKSEL=3)
sr([0x05, 0, 1, E(1,1,0)])  # AP read reg 0 (IDR in bank 3)
r = sr([0x05, 0, 1, E(0,1,3)])  # RDBUFF
if r:
    print(f"AP_IDR     = 0x{rd4(r):08X} (via RDBUFF)")

dev.close()

# === Now test via production code ===
print("\n--- Production code test ---")
sys.path.insert(0, os.path.abspath('.'))
from src.io.cmsis_dap import CmsisDapProtocol
dap = CmsisDapProtocol()
dap.transport.open_vid_pid(0xC251, 0xF00A)
dap.dap_connect()
dap._swd_init(reset=False)
dap._ap_setup()
# Read via production code
idcode = dap.swd_read_dp(0)
print(f"DP_IDCODE  = 0x{idcode:08X}")
ctrl = dap.swd_read_dp(1)
print(f"CTRL_STAT = 0x{ctrl:08X}")
csw = dap.swd_read_ap(0)
print(f"AP_CSW     = 0x{csw:08X} (via swd_read_ap)")
val = dap.read32(0x08000000)
print(f"Mem[0x08000000] = 0x{val:08X}")
dap.close()
