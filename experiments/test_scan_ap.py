"""Scan all APs via double-read technique."""
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

sr([0x02, 1]); [sr([0x12, 96] + [0xFF]*12) for _ in range(3)]
sr([0x12, 16] + [0x9E, 0xE7]); sr([0x12, 8] + [0x00])
sr([0x13, 0])
sr([0x05, 0, 1, E(0,1,0)])
sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])

# Try reading DP IDR/CTRL_STAT
r = sr([0x05, 0, 1, E(0,1,1)])
print(f"CTRL = 0x{rd4(r):08X}")

# Scan first 8 APs (APSEL=0..7) for IDR (bank 3, reg 0)
print("\n=== Scanning APs for valid IDR ===")
found = []
for ap in range(8):
    # Set APSEL=ap, APBANKSEL=3
    sr([0x05, 0, 1, E(0,0,2), (ap << 8) | 3, 0, 0, 0])
    # Double AP read reg 0 (IDR in bank 3)
    sr([0x05, 0, 1, E(1,1,0)])  # Primer
    r = sr([0x05, 0, 1, E(1,1,0)])  # Actual read
    if r:
        idr = rd4(r)
        print(f"  AP[{ap}] IDR = 0x{idr:08X}")
        if idr and idr != 0xFFFFFFFF:
            found.append((ap, idr))

if not found:
    print("\nNo valid AP found! Trying raw stale data from all APs...")
    for ap in range(8):
        sr([0x05, 0, 1, E(0,0,2), (ap << 8) | 0, 0, 0, 0])
        r1 = sr([0x05, 0, 1, E(1,1,0)])
        r2 = sr([0x05, 0, 1, E(1,1,0)])
        if r1 and r2:
            s1, s2 = rd4(r1), rd4(r2)
            if s1 != s2:
                print(f"  AP[{ap}] bank0: stale1={s1:08X} stale2={s2:08X} (CHANGED!)")

dev.close()
