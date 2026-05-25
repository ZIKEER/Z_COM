"""Debug: test 2-request DAP_Transfer with DP reads vs AP reads."""
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
    data = dev.read(65, timeout_ms=5000)
    return bytes(data) if data else None
def sr(cmd):
    w(cmd)
    return rr()

sr([0x02, 1])
sr([0x12, 96] + [0xFF]*12)
sr([0x12, 16] + [0x9E, 0xE7])
sr([0x12, 96] + [0xFF]*12)
sr([0x12, 8] + [0x00])
sr([0x13, 0])

E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

# 1. Single read: DP_IDCODE
r1 = sr([0x05, 0, 1, E(0,1,0)])
print(f"DP_IDCODE (1 read):      {r1[:12].hex()}")

# 2. Two DP reads in one transfer
r2 = sr([0x05, 0, 2, E(0,1,0), E(0,1,1)])
print(f"DP_IDCODE+CTRL (2 rd):   {r2[:16].hex()}")
for i in range(min(14, len(r2))):
    print(f"  [{i}] = 0x{r2[i]:02X}")

# 3. Write DP_CTRL_STAT to power up
r3 = sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
print(f"CTRL_STAT write:         {r3[:8].hex()}")

# 4. Write DP_SELECT
r4 = sr([0x05, 0, 1, E(0,0,2), 0,0,0,0])
print(f"DP_SELECT write:         {r4[:8].hex()}")

# 5. Write AP_CSW
csw_val = 0x23000012
csw_b = struct.pack('<I', csw_val)
r5 = sr([0x05, 0, 1, E(1,0,0)] + list(csw_b))
print(f"AP_CSW write:            {r5[:8].hex()}")

# 6. Two DP reads again (now AP is set up)
r6 = sr([0x05, 0, 2, E(0,1,0), E(0,1,1)])
print(f"\nDP_IDCODE+CTRL (2 rd, after AP init):")
for i in range(min(14, len(r6))):
    print(f"  [{i}] = 0x{r6[i]:02X}")
idcode = struct.unpack_from('<I', r6, 3)[0]
ctrl = struct.unpack_from('<I', r6, 8)[0]
print(f"  => IDCODE=0x{idcode:08X} CTRL=0x{ctrl:08X}")

# 7. AP read + RDBUFF (the problematic transfer)
r7 = sr([0x05, 0, 2, E(1,1,0), E(0,1,3)])
print(f"\nAP_CSW + RDBUFF (2 rd):")
for i in range(min(14, len(r7))):
    print(f"  [{i}] = 0x{r7[i]:02X}")

# 8. Separate: AP read then RDBUFF
r8a = sr([0x05, 0, 1, E(1,1,0)])
r8b = sr([0x05, 0, 1, E(0,1,3)])
print(f"\nAP_CSW alone:  {r8a[:10].hex()}")
print(f"RDBUFF alone:  {r8b[:10].hex()}")
if len(r8b) >= 7:
    ap_val = struct.unpack_from('<I', r8b, 3)[0]
    print(f"AP_CSW (via RDBUFF alone) = 0x{ap_val:08X}")

dev.close()
