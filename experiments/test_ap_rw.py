"""Test: write AP_TAR then read it back."""
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
    time.sleep(0.3)
    data = dev.read(65, timeout_ms=3000)
    if not data: print(f"  [{label}] TIMEOUT"); return None
    return bytes(data)
def rd4(b, pos=3):
    return struct.unpack_from('<I', b, pos)[0]

E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

sr([0x02, 1])
sr([0x12, 96] + [0xFF]*12)
sr([0x12, 16] + [0x9E, 0xE7])
sr([0x12, 96] + [0xFF]*12)
sr([0x12, 8] + [0x00])
sr([0x13, 0])

sr([0x05, 0, 1, E(0,1,0)], "")
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
r = sr([0x05, 0, 1, E(0,1,1)])
print(f"CTRL_STAT = 0x{rd4(r):08X}")

# DP_SELECT = 0 (AP0, bank 0)
sr([0x05, 0, 1, E(0,0,2), 0,0,0,0])

# TEST 1: write AP_TAR (reg A[3:2]=1), read back
test_val = 0xA5A55A5A
req_wr = E(1, 0, 1)  # AP write TAR
sr([0x05, 0, 1, req_wr] + list(struct.pack('<I', test_val)))
print(f"\nTAR write 0x{test_val:08X}")

# Readback TAR
req_rd = E(1, 1, 1)  # AP read TAR
sr([0x05, 0, 1, req_rd])
r = sr([0x05, 0, 1, E(0,1,3)])
if r: print(f"TAR readback = 0x{rd4(r):08X}")

# Without re-writing, read again
sr([0x05, 0, 1, req_rd])
r = sr([0x05, 0, 1, E(0,1,3)])
if r: print(f"TAR 2nd read = 0x{rd4(r):08X}")

# TEST 2: try read CSW (current state, we haven't written it)
print(f"\n--- Read CSW directly (no prior write) ---")
req_csw_rd = E(1, 1, 0)
sr([0x05, 0, 1, req_csw_rd])
r = sr([0x05, 0, 1, E(0,1,3)])
if r: print(f"CSW readback = 0x{rd4(r):08X}")

# TEST 3: write CSW, then read via RDBUFF
print(f"\n--- Write CSW = 0x23000012 ---")
req_csw_wr = E(1, 0, 0)
sr([0x05, 0, 1, req_csw_wr] + list(struct.pack('<I', 0x23000012)))

# Read TAR (should still be 0xA5A55A5A)
print("--- TAR should still be 0xA5A55A5A ---")
sr([0x05, 0, 1, req_rd])
r = sr([0x05, 0, 1, E(0,1,3)])
if r: print(f"TAR = 0x{rd4(r):08X}")

# Read CSW
sr([0x05, 0, 1, req_csw_rd])
r = sr([0x05, 0, 1, E(0,1,3)])
if r: print(f"CSW = 0x{rd4(r):08X}")

# TEST 4: Write address to TAR then read from DRW at that address
print(f"\n--- Memory read via AP (32-bit at 0x08000000) ---")
sr([0x05, 0, 1, req_wr] + list(struct.pack('<I', 0x08000000)))

req_drw_rd = E(1, 1, 3)  # AP read DRW (A[3:2]=3)
v = sr([0x05, 0, 1, req_drw_rd])
r = sr([0x05, 0, 1, E(0,1,3)])
if r: print(f"Mem[0x08000000] = 0x{rd4(r):08X}")

dev.close()
print("\n=== ALL DONE ===")
