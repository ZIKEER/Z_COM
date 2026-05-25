"""Diagnostic: write AP TAR directly and read back via RDBUFF."""
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

def w(data):
    dev.write(b'\x00' + bytes(data).ljust(64, b'\x00'))
def rr():
    return bytes(dev.read(65, timeout_ms=5000) or [])
def sr(cmd):
    w(cmd)
    return rr()
def rd4(b, pos=3):
    return struct.unpack_from('<I', b, pos)[0]

E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

# Init SWD
sr([0x02, 1]); rr()
for _ in range(3): sr([0x12, 96] + [0xFF]*12); rr()
sr([0x12, 16] + [0x9E, 0xE7]); rr()
sr([0x12, 8] + [0x00]); rr()
sr([0x13, 0]); rr()

# Read IDCODE
r = sr([0x05, 0, 1, E(0,1,0)])
print(f"DP_IDCODE = 0x{rd4(r):08X}")

# Power up
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
r = sr([0x05, 0, 1, E(0,1,1)])
print(f"CTRL_STAT = 0x{rd4(r):08X}")

# Set DP_SELECT = 0 (AP 0, bank 0)
sr([0x05, 0, 1, E(0,0,2), 0,0,0,0]); rr()

# ===== TEST: Write AP_TAR with known value ===== 
# AP TAR is at register A[3:2]=1, bank 0
req_tar_wr = E(1, 0, 1)  # AP write TAR
for test_val in [0xA5A55A5A, 0x12345678, 0xDEADBEEF]:
    w_bytes = struct.pack('<I', test_val)
    r = sr([0x05, 0, 1, req_tar_wr] + list(w_bytes))
    print(f"\nWrite AP_TAR = 0x{test_val:08X} -> status=0x{r[2]:02X}")
    
    # Read AP TAR via RDBUFF (2 separate transfers)
    # First: AP read TAR (stale data, ignore)
    req_tar_rd = E(1, 1, 1)  # AP read TAR
    sr([0x05, 0, 1, req_tar_rd]); rr()
    # Second: DP RDBUFF read (actual value)
    r2 = sr([0x05, 0, 1, E(0, 1, 3)])
    val = rd4(r2, 3)
    print(f"  Readback via RDBUFF = 0x{val:08X}")

# ===== TEST: DP RDBUFF behavior after AP write =====
print("\n=== Verify RDBUFF isolation ===")
# Write AP_CSW
req_csw_wr = E(1, 0, 0)
csw_val = 0x23000012
sr([0x05, 0, 1, req_csw_wr] + list(struct.pack('<I', csw_val))); rr()

# Read AP CSW 
req_csw_rd = E(1, 1, 0)
sr([0x05, 0, 1, req_csw_rd]); rr()
r = sr([0x05, 0, 1, E(0,1,3)])
val = rd4(r, 3)
print(f"AP_CSW readback = 0x{val:08X} (should be ~0x23000012)")

# Read AP_IDR (bank 3)
print("\n=== Read AP_IDR ===")
sr([0x05, 0, 1, E(0,0,2), 3, 0, 0, 0]); rr()
req_idr_rd = E(1, 1, 0)
sr([0x05, 0, 1, req_idr_rd]); rr()
r = sr([0x05, 0, 1, E(0,1,3)])
val = rd4(r, 3)
print(f"AP_IDR = 0x{val:08X}")

# ===== TEST: Read AP ROM table =====
# AHB-AP has a 4kB register space. The first word at offset 0xF0 is the ROM table entry.
print("\n=== Read AP ROM table ===")
sr([0x05, 0, 1, E(0,0,2), 0, 0, 0, 0]); rr()

# Write AP TAR = 0xF0
sr([0x05, 0, 1, E(1,0,1)] + list(struct.pack('<I', 0xF0))); rr()
# Read AP DRW
sr([0x05, 0, 1, E(1,1,3)]); rr()
r = sr([0x05, 0, 1, E(0,1,3)])
val = rd4(r, 3)
print(f"AP mem[0xF0] = 0x{val:08X} (ROM table)")

dev.close()
