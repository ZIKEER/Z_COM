"""Test: write AP CSW → AP read (stale) → RDBUFF = actual CSW?"""
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

# Init SWD
sr([0x02, 1])
for _ in range(3): sr([0x12, 96] + [0xFF]*12)
sr([0x12, 16] + [0x9E, 0xE7])
sr([0x12, 8] + [0x00])
sr([0x13, 0])

sr([0x05, 0, 1, E(0,1,0)])
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
r = sr([0x05, 0, 1, E(0,1,1)])
ctrl = rd4(r)
print(f"CTRL_STAT = 0x{ctrl:08X}")

# Clear STICKYORUN
if ctrl & (1 << 6):
    sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
    r = sr([0x05, 0, 1, E(0,1,1)])
    ctrl = rd4(r)
    print(f"CTRL_STAT (cleared) = 0x{ctrl:08X}")

# DP_SELECT = 0
sr([0x05, 0, 1, E(0,0,2), 0,0,0,0])

# === METHOD A: Read AP CSW directly (stale data, no RDBUFF) ===
r = sr([0x05, 0, 1, E(1,1,0)])
stale = rd4(r)
print(f"\nAP CSW (direct stale) = 0x{stale:08X}")

# === METHOD A2: Read AP CSW via RDBUFF after the direct read ===
r = sr([0x05, 0, 1, E(0,1,3)])
rdb_before_write = rd4(r)
print(f"AP CSW (via RDBUFF)   = 0x{rdb_before_write:08X}")

# === Write AP CSW ===
print(f"\n--- Write AP_CSW = 0x23000012 ---")
sr([0x05, 0, 1, E(1,0,0)] + list(struct.pack('<I', 0x23000012)))

# Read AP CSW stale data (should be stale from PREVIOUS read)
r = sr([0x05, 0, 1, E(1,1,0)])
stale2 = rd4(r)
print(f"AP CSW (stale after write) = 0x{stale2:08X}")

# Read RDBUFF
r = sr([0x05, 0, 1, E(0,1,3)])
rdb_after_write = rd4(r)
print(f"AP CSW (RDBUFF after write)= 0x{rdb_after_write:08X}")

# Now the RDBUFF should have the CSW value we just read (the stale2 was the INITIAL read)
# Let's read once more to get the TRUE CSW
r = sr([0x05, 0, 1, E(1,1,0)])  # AP read CSW again
stale3 = rd4(r)
print(f"\nAP CSW (stale #3) = 0x{stale3:08X}")
r = sr([0x05, 0, 1, E(0,1,3)])  # RDBUFF
rdb3 = rd4(r)
print(f"AP CSW (RDBUFF #3) = 0x{rdb3:08X}")

# One more iteration to be sure
r = sr([0x05, 0, 1, E(1,1,0)])
stale4 = rd4(r)
r = sr([0x05, 0, 1, E(0,1,3)])
rdb4 = rd4(r)
print(f"\nLoop: stale=0x{stale4:08X} RDBUFF=0x{rdb4:08X}")

r = sr([0x05, 0, 1, E(1,1,0)])
stale5 = rd4(r)
r = sr([0x05, 0, 1, E(0,1,3)])
rdb5 = rd4(r)
print(f"Loop: stale=0x{stale5:08X} RDBUFF=0x{rdb5:08X}")

dev.close()
