"""Scan APs with correct DP_SELECT format."""
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
    time.sleep(0.15)
    data = dev.read(65, timeout_ms=3000)
    return bytes(data) if data else None
def rd4(b):
    return struct.unpack_from('<I', b, 3)[0]
E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

# Fresh init
sr([0x03]); time.sleep(0.2)
sr([0x02, 1])
for _ in range(3): sr([0x12, 96] + [0xFF]*12)
sr([0x12, 16] + [0x9E, 0xE7]); sr([0x12, 8] + [0x00])
sr([0x13, 0])
sr([0x05, 0, 1, E(0,1,0)])
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
r = sr([0x05, 0, 1, E(0,1,1)])
print(f"CTRL = 0x{rd4(r):08X}")

# Scan APSEL=0..7, APBANKSEL=3 (IDR)
print("\n=== Scan APs for IDR ===")
for ap in range(8):
    sel_val = (ap << 24) | (3 << 4)  # APSEL=ap, APBANKSEL=3
    sr([0x05, 0, 1, E(0,0,2)] + list(struct.pack('<I', sel_val)))
    sr([0x05, 0, 1, E(1,1,0)])  # Primer
    r = sr([0x05, 0, 1, E(1,1,0)])  # Read
    if r:
        idr = rd4(r)
        print(f"  AP[{ap}] IDR = 0x{idr:08X}", end="")
        # Decode common AP types
        part = (idr >> 12) & 0xFFFF
        rev = (idr >> 8) & 0xF
        if part == 0x2477:
            print(" (AHB-AP, Cortex-M4)")
        elif part == 0x1477:
            print(" (AHB-AP, Cortex-M3)")
        else:
            print()

# Also scan bank 0
print("\n=== Scan APs bank 0 (CSW) ===")
for ap in range(8):
    sel_val = (ap << 24)  # APSEL=ap, APBANKSEL=0
    sr([0x05, 0, 1, E(0,0,2)] + list(struct.pack('<I', sel_val)))
    sr([0x05, 0, 1, E(1,1,0)])  # Primer
    r1 = sr([0x05, 0, 1, E(1,1,0)])
    sr([0x05, 0, 1, E(1,1,0)])  # Another primer
    r2 = sr([0x05, 0, 1, E(1,1,0)])
    if r1 and r2:
        s1, s2 = rd4(r1), rd4(r2)
        change = " <- CHANGED!" if s1 != s2 else ""
        print(f"  AP[{ap}] bank0: s1={s1:08X} s2={s2:08X}{change}")

dev.close()
