"""Step-by-step AP read pipeline trace.
Procedure per step:
  1. Single DAP_Transfer with AP_READ → returns STALE data 
  2. Single DAP_Transfer with DP_RDBUFF → returns ACTUAL value
Stale[0] → actual[0] → stale[1] = actual[0] → actual[1] ...
"""
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
    time.sleep(0.2)
    data = dev.read(65, timeout_ms=3000)
    return bytes(data) if data else None
def rd4(b, pos=3):
    return struct.unpack_from('<I', b, pos)[0]

E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

# Init
sr([0x02, 1])
for _ in range(3): sr([0x12, 96] + [0xFF]*12)
sr([0x12, 16] + [0x9E, 0xE7])
sr([0x12, 8] + [0x00])
sr([0x13, 0])

# Power up
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
r = sr([0x05, 0, 1, E(0,1,1)])
print(f"CTRL = 0x{rd4(r):08X}")
if rd4(r) & (1<<6):  # STICKYORUN
    sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
    r = sr([0x05, 0, 1, E(0,1,1)])
    print(f"CTRL(clr) = 0x{rd4(r):08X}")

# DP_SELECT = 0
sr([0x05, 0, 1, E(0,0,2), 0,0,0,0])

# === Read AP_CSW WITHOUT any prior write ===
print("\n--- AP_CSW read, no prior write ---")
r = sr([0x05, 0, 1, E(1,1,0)])  # AP read reg0
s1 = rd4(r)
print(f"  STALE[0] = 0x{s1:08X}")

r = sr([0x05, 0, 1, E(0,1,3)])  # RDBUFF
a1 = rd4(r)
print(f"  ACTUAL[0]= 0x{a1:08X}")

# Read again: stale should = ACTUAL[0]
r = sr([0x05, 0, 1, E(1,1,0)])
s2 = rd4(r)
print(f"  STALE[1] = 0x{s2:08X} (should = ACTUAL[0] = 0x{a1:08X})")

r = sr([0x05, 0, 1, E(0,1,3)])
a2 = rd4(r)
print(f"  ACTUAL[1]= 0x{a2:08X}")

# === Now write CSW ===
CSW_CORRECT = 0x23000000 | (2 << 4) | 1
print(f"\n--- Write AP_CSW = 0x{CSW_CORRECT:08X} ---")
sr([0x05, 0, 1, E(1,0,0)] + list(struct.pack('<I', CSW_CORRECT)))

# Read back
r = sr([0x05, 0, 1, E(1,1,0)])
s3 = rd4(r)
r = sr([0x05, 0, 1, E(0,1,3)])
a3 = rd4(r)
print(f"  STALE[2] = 0x{s3:08X} (should = ACTUAL[1])")
print(f"  ACTUAL[2]= 0x{a3:08X} (should = written CSW)")

# One more to confirm
r = sr([0x05, 0, 1, E(1,1,0)])
s4 = rd4(r)
r = sr([0x05, 0, 1, E(0,1,3)])
a4 = rd4(r)
print(f"  STALE[3] = 0x{s4:08X} (should = ACTUAL[2])")
print(f"  ACTUAL[3]= 0x{a4:08X} (should = CSW again)")

dev.close()
