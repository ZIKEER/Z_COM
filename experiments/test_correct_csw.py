"""Test AP write with CORRECT CSW bits."""
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

# === Init SWD ===
sr([0x02, 1])
for _ in range(3): sr([0x12, 96] + [0xFF]*12)
sr([0x12, 16] + [0x9E, 0xE7])
sr([0x12, 8] + [0x00])
sr([0x13, 0])

sr([0x05, 0, 1, E(0,1,0)])
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
r = sr([0x05, 0, 1, E(0,1,1)])
print(f"CTRL_STAT = 0x{rd4(r):08X}")

# === CORRECT CSW: SIZE=32bit (bits[5:4]=10), AddrInc=1 (bits[1:0]=01) ===
CSW_CORRECT = 0x23000000 | (2 << 4) | 1  # = 0x23000021

# Write DP_SELECT = 0
sr([0x05, 0, 1, E(0,0,2), 0,0,0,0])

# Write correct CSW
print(f"\nWriting AP_CSW = 0x{CSW_CORRECT:08X}")
sr([0x05, 0, 1, E(1,0,0)] + list(struct.pack('<I', CSW_CORRECT)))

# Now read back via 2 separate transfers
# 1. AP read CSW
sr([0x05, 0, 1, E(1,1,0)])
# 2. RDBUFF
r = sr([0x05, 0, 1, E(0,1,3)])
if r:
    val = rd4(r)
    print(f"AP_CSW (via RDBUFF) = 0x{val:08X}")

# === Try reading AP_IDR properly ===
# AP_IDR = register 0x0C = bank 3, A[3:2]=0
print(f"\nReading AP_IDR (APBANKSEL=3):")
sr([0x05, 0, 1, E(0,0,2), 3, 0, 0, 0])
sr([0x05, 0, 1, E(1,1,0)])  # AP read reg 0 (IDR in bank 3)
r = sr([0x05, 0, 1, E(0,1,3)])
if r:
    val = rd4(r)
    print(f"AP_IDR = 0x{val:08X}")

# === Restore APBANKSEL=0 for CSW ===
sr([0x05, 0, 1, E(0,0,2), 0, 0, 0, 0])

# === Now read CSW again to confirm write persisted ===
print(f"\nVerifying AP_CSW still set:")
sr([0x05, 0, 1, E(1,1,0)])
r = sr([0x05, 0, 1, E(0,1,3)])
if r:
    val = rd4(r)
    print(f"AP_CSW = 0x{val:08X}")

# === Now write TAR and read DRW ===
print(f"\nMemory read test:")
# Set TAR
sr([0x05, 0, 1, E(1,0,1)] + list(struct.pack('<I', 0x08000000)))
# Write CSW_AR = 0x22000000 to allow unaligned access
# Actually CSW is already set. Let's just read.
sr([0x05, 0, 1, E(1,1,3)])  # AP read DRW
r = sr([0x05, 0, 1, E(0,1,3)])
if r:
    val = rd4(r)
    print(f"Mem[0x08000000] = 0x{val:08X}")

dev.close()
