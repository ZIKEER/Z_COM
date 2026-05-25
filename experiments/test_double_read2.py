"""Final test: double AP read technique on fresh device."""
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
    dev.write(b'\x00' + bytes(cmd).ljust(64, b'\x00'))
    time.sleep(0.2)
    data = dev.read(65, timeout_ms=3000)
    if not data: print(f"  [{label}] TIMEOUT"); return None
    return bytes(data)
def rd4(b, pos=3):
    return struct.unpack_from('<I', b, pos)[0]
E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

sr([0x02, 1], "Connect")
sr([0x12, 96] + [0xFF]*12, "SWJ96")
sr([0x12, 16] + [0x9E, 0xE7], "SWJsw")
sr([0x12, 96] + [0xFF]*12, "SWJ96")
sr([0x12, 8] + [0x00], "Idle")
sr([0x13, 0], "Cfg")

r = sr([0x05, 0, 1, E(0,1,0)])
if r: print(f"IDCODE = 0x{rd4(r):08X}")

sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50], "Power")
r = sr([0x05, 0, 1, E(0,1,1)])
if r: print(f"CTRL   = 0x{rd4(r):08X}")

sr([0x05, 0, 1, E(0,0,2), 0,0,0,0], "SELECT")

# === Double AP read: first triggers, second reads ===
print("\n=== Double AP read (CSW, no prior write) ===")
r1 = sr([0x05, 0, 1, E(1,1,0)], "AP1")
r2 = sr([0x05, 0, 1, E(1,1,0)], "AP2")
if r1: print(f"  AP[1] stale={rd4(r1):08X}")
if r2: print(f"  AP[2] stale={rd4(r2):08X} (this IS the actual CSW)")

# === Write CSW ===
print(f"\n=== Write CSW = 0x23000021 ===")
sr([0x05, 0, 1, E(1,0,0)] + list(struct.pack('<I', 0x23000021)))

# === Double AP read again ===
r1 = sr([0x05, 0, 1, E(1,1,0)], "AP1")
r2 = sr([0x05, 0, 1, E(1,1,0)], "AP2")
if r1: print(f"  AP[1] stale={rd4(r1):08X} (old RDBUFF)")
if r2: print(f"  AP[2] stale={rd4(r2):08X} (should = written CSW if write worked)")

# === Read IDR via double AP read with bank switch ===
# IDR is at APBANKSEL=3, A[3:2]=0
print("\n=== AP_IDR via double read ===")
sr([0x05, 0, 1, E(0,0,2), 3, 0, 0, 0])
r1 = sr([0x05, 0, 1, E(1,1,0)], "IDR1")
r2 = sr([0x05, 0, 1, E(1,1,0)], "IDR2")
if r2: print(f"  AP_IDR = 0x{rd4(r2):08X}")
sr([0x05, 0, 1, E(0,0,2), 0, 0, 0, 0])

# === Memory read ===
print("\n=== Memory read at 0x08000000 ===")
sr([0x05, 0, 1, E(1,0,1)] + list(struct.pack('<I', 0x08000000)))
r1 = sr([0x05, 0, 1, E(1,1,3)], "DRW1")  # AP read DRW
r2 = sr([0x05, 0, 1, E(1,1,3)], "DRW2")  # AP read DRW again (stale = previous DRW read)
if r1: print(f"  DRW[1] stale={rd4(r1):08X}")
if r2: print(f"  DRW[2] stale={rd4(r2):08X}  <-- actual memory content")

dev.close()
