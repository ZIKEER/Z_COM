"""Deep debug: AP power, selection, IDR."""
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
def dump(label, r, n=16):
    print(f"  {label}: {' '.join(f'{b:02X}' for b in r[:n])}")

E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)
rd4 = lambda b, pos: struct.unpack_from('<I', b, pos)[0]

sr([0x02, 1]); dump("Connect", rr())
sr([0x12, 96] + [0xFF]*12); rr()
sr([0x12, 16] + [0x9E, 0xE7]); rr()
sr([0x12, 96] + [0xFF]*12); rr()
sr([0x12, 8] + [0x00]); rr()
sr([0x13, 0]); rr()

# === Phase 1: DP basics ===
r = sr([0x05, 0, 1, E(0,1,0)])
idcode = rd4(r, 3)
print(f"\nDP_IDCODE = 0x{idcode:08X}")

# Power up
r = sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
print(f"CTRL_STAT write: status=0x{r[2]:02X}")
time.sleep(0.05)

r = sr([0x05, 0, 1, E(0,1,1)])
ctrl = rd4(r, 3)
print(f"CTRL_STAT read = 0x{ctrl:08X}")
print(f"  CDBGPWRUPACK={(ctrl>>30)&1} CSYSPWRUPACK={(ctrl>>28)&1}")
print(f"  STICKYORUN={(ctrl>>6)&1} ORUNDETECT={(ctrl>>4)&1}")

# Clear sticky bits if needed
if ctrl & (1 << 6):
    print("  -> clearing STICKYORUN")
    sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])

# === Phase 2: AP identification ===
# Try reading AP IDR for AP 0
for bank in range(8):
    sr([0x05, 0, 1, E(0,0,2), bank, 0, 0, 0])  # DP_SELECT = bank
    r = sr([0x05, 0, 1, E(1,1,0)])  # AP read reg 0 (IDR with bank 3)
    time.sleep(0.05)
    # First AP read: stale, ignore
    # Second: RDBUFF
    r2 = sr([0x05, 0, 1, E(0,1,3)])  # DP RDBUFF
    val = rd4(r2, 3)
    if val and val != 0xFFFFFFFF:
        print(f"\nAP 0, bank={bank}: reg0=0x{val:08X}")

# Try AP 1
for bank in [0, 3]:
    sr([0x05, 0, 1, E(0,0,2), 0x01 | (bank << 4), 0, 0, 0])  # APSEL=1, APBANKSEL=bank
    r2 = sr([0x05, 0, 1, E(0,1,3)])
    val = rd4(r2, 3)
    if val and val != 0xFFFFFFFF:
        print(f"\nAP 1, bank={bank}: RDBUFF=0x{val:08X}")

# === Phase 3: Try single AP read without separate RDBUFF ===
# Some targets return data directly in the AP read response
print("\n=== Single AP read without RDBUFF (not recommended but diagnostic) ===")
sr([0x05, 0, 1, E(0,0,2), 0, 0, 0, 0])  # DP_SELECT=0
rr()
r = sr([0x05, 0, 1, E(1,1,0)])  # AP read reg 0 (CSW in bank 0)
data = rd4(r, 3)
print(f"  AP read (no RDBUFF): 0x{data:08X}")

# === Phase 4: Try write + readback of DP register via RDBUFF ===
# Write a DP register (RESEND/IDCODE), then read RDBUFF
print("\n=== DP RDBUFF test ===")
r = sr([0x05, 0, 1, E(0,1,3)])  # DP RDBUFF read, no prior AP read
data = rd4(r, 3)
print(f"  RDBUFF after nothing: 0x{data:08X}")

r = sr([0x05, 0, 1, E(0,1,0)])  # DP IDCODE read
data = rd4(r, 3)
print(f"  DP IDCODE: 0x{data:08X} (same as before)")

r = sr([0x05, 0, 1, E(0,1,3)])  # DP RDBUFF read
data = rd4(r, 3)
print(f"  RDBUFF after DP read: 0x{data:08X}")
print(f"  (should be same as IDCODE if RDBUFF caches DP reads)")

dev.close()
