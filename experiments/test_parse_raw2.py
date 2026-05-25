"""Dump raw DAP_Transfer for 1-req and 2-req with proper AP init."""
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

# --- SWD Init ---
print(f"Connect:     {sr([0x02, 1]).hex()}")
print(f"SWJ_SEQ(96): {sr([0x12, 96] + [0xFF]*12).hex()}")
print(f"SWJ_SEQ(sw): {sr([0x12, 16] + [0x9E, 0xE7]).hex()}")
print(f"SWJ_SEQ(96): {sr([0x12, 96] + [0xFF]*12).hex()}")
print(f"SWJ_SEQ(id): {sr([0x12, 8] + [0x00]).hex()}")
print(f"SWD_CFG:     {sr([0x13, 0]).hex()}")

# Read DP_IDCODE (1 read)
req_rd0 = (0&1)|((1&1)<<1)|((0&3)<<2)  # DP read IDCODE
r1 = sr([0x05, 0, 1, req_rd0])
print(f"\n=== DP_IDCODE (1 read) ===")
for i in range(min(12, len(r1))):
    print(f"  [{i}] = 0x{r1[i]:02X}")
idcode = struct.unpack_from('<I', r1, 3)[0]
print(f"  => DP_IDCODE = 0x{idcode:08X}")

# Write DP_CTRL_STAT to power up
req_wr1 = (0&1)|((0&1)<<1)|((1&3)<<2)  # DP write CTRL_STAT
r2 = sr([0x05, 0, 1, req_wr1, 0x00, 0x00, 0x00, 0x50])
print(f"\n=== DP_CTRL_STAT write ===")
for i in range(min(8, len(r2))):
    print(f"  [{i}] = 0x{r2[i]:02X}")

time.sleep(0.1)

# Write DP_SELECT (AP 0, bank 0)
req_wr_sel = (0&1)|((0&1)<<1)|((2&3)<<2)
r_sel = sr([0x05, 0, 1, req_wr_sel, 0, 0, 0, 0])
print(f"\n=== DP_SELECT write ===")
for i in range(min(8, len(r_sel))):
    print(f"  [{i}] = 0x{r_sel[i]:02X}")

# Write AP_CSW to init
req_wr_csw = (1&1)|((0&1)<<1)|((0&3)<<2)  # AP write CSW
csw_val = 0x23000010 | 2  # 32-bit, addr inc
csw_bytes = struct.pack('<I', csw_val)
r_csw = sr([0x05, 0, 1, req_wr_csw] + list(csw_bytes))
print(f"\n=== AP_CSW write ===")
for i in range(min(8, len(r_csw))):
    print(f"  [{i}] = 0x{r_csw[i]:02X}")

# Now read AP_CSW: 2 reads in one transfer (AP read + RDBUFF)
req_ap_rd = (1&1)|((1&1)<<1)|((0&3)<<2)  # AP read CSW
req_rdb = (0&1)|((1&1)<<1)|((3&3)<<2)    # DP read RDBUFF
r3 = sr([0x05, 0, 2, req_ap_rd, req_rdb])
print(f"\n=== AP_CSW via 2 reads in one DAP_Transfer ===")
for i in range(min(16, len(r3))):
    print(f"  [{i}] = 0x{r3[i]:02X}")

# Now try: 2 separate single transfers
print(f"\n=== AP_CSW via 2 separate DAP_Transfer ===")
r4 = sr([0x05, 0, 1, req_ap_rd])
print(f"AP_CSW read (single):", " ".join(f"0x{r4[i]:02X}" for i in range(min(10, len(r4)))))

r5 = sr([0x05, 0, 1, req_rdb])
print(f"RDBUFF read (single):", " ".join(f"0x{r5[i]:02X}" for i in range(min(10, len(r5)))))

if len(r4) >= 7:
    stale = struct.unpack_from('<I', r4, 3)[0]
    print(f"AP stale data = 0x{stale:08X}")
if len(r5) >= 7:
    actual = struct.unpack_from('<I', r5, 3)[0]
    print(f"RDBUFF result  = 0x{actual:08X}")

dev.close()
