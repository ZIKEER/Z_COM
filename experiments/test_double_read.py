"""Test: double AP read to bypass RDBUFF confusion."""
import sys, os, struct, time
sys.path.insert(0, os.path.abspath('.'))

import hid

def run():
    dev = hid.device()
    for info in hid.enumerate(0xC251, 0xF00A):
        prod = info.get('product_string', '')
        if 'cmsis-dap' in prod.lower():
            dev.open_path(info['path'])
            break
    dev.set_nonblocking(True)

    def sr(cmd):
        dev.write(b'\x00' + bytes(cmd).ljust(64, b'\x00'))
        time.sleep(0.25)
        data = dev.read(65, timeout_ms=3000)
        return bytes(data) if data else None
    def rd4(b, pos=3):
        return struct.unpack_from('<I', b, pos)[0]
    E = lambda ap, rnw, a: (ap&1)|((rnw&1)<<1)|((a&3)<<2)

    sr([0x02, 1])
    for _ in range(3): sr([0x12, 96] + [0xFF]*12)
    sr([0x12, 16] + [0x9E, 0xE7])
    sr([0x12, 8] + [0x00])
    sr([0x13, 0])
    r = sr([0x05, 0, 1, E(0,1,0)])
    print(f"IDCODE = 0x{rd4(r):08X}")
    sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
    r = sr([0x05, 0, 1, E(0,1,1)])
    print(f"CTRL   = 0x{rd4(r):08X}")
    
    # Clear sticky
    if rd4(r) & (1<<6):
        sr([0x05, 0, 1, E(0,0,1), 0x00, 0x00, 0x00, 0x50])
        r = sr([0x05, 0, 1, E(0,1,1)])
        print(f"CTRL(clr) = 0x{rd4(r):08X}")

    sr([0x05, 0, 1, E(0,0,2), 0,0,0,0])

    # === Phase 1: Write CSW + double AP read ===
    CSW_CORRECT = 0x23000000 | (2 << 4) | 1  # = 0x23000021
    
    print(f"\n--- Write AP_CSW = 0x{CSW_CORRECT:08X} ---")
    sr([0x05, 0, 1, E(1,0,0)] + list(struct.pack('<I', CSW_CORRECT)))
    
    print("\n--- Phase: Double AP read ---")
    sr([0x05, 0, 1, E(1,1,0)])  # AP read 0 (CSW) → RDBUFF = actual CSW
    r = sr([0x05, 0, 1, E(1,1,0)])  # AP read 0 again → stale = RDBUFF = CSW
    if r:
        val = rd4(r)
        print(f"  2nd AP read (stale = CSW) = 0x{val:08X}")
        print(f"  Expected 0x{CSW_CORRECT:08X}, match={val == CSW_CORRECT}")

    # === Phase 2: Memory read ===
    print(f"\n--- Memory read: write TAR=0x08000000, then double-read DRW ---")
    sr([0x05, 0, 1, E(1,0,1)] + list(struct.pack('<I', 0x08000000)))
    sr([0x05, 0, 1, E(1,1,3)])  # AP read DRW (pipeline: RDBUFF gets DRW value)
    r = sr([0x05, 0, 1, E(1,1,3)])  # AP read DRW (stale = previous DRW)
    if r:
        val = rd4(r)
        print(f"  2nd AP read DRW = 0x{val:08X}")

    # === Phase 3: Combined transfer (2 requests) - DP reads only ===
    print(f"\n--- Combined DAP_Transfer: 2 DP reads ---")
    r = sr([0x05, 0, 2, E(0,1,0), E(0,1,1)])
    if r:
        print(f"  Raw: {r[:16].hex()}")
        # Try parsing as [echo, count, status, data0(4B), data1(4B)]
        if len(r) >= 11:
            d0 = struct.unpack_from('<I', r, 3)[0]
            d1 = struct.unpack_from('<I', r, 7)[0]
            print(f"  [fmt: echo,count,status,d0,d1]")
            print(f"  DP_IDCODE={d0:08X} DP_CTRL={d1:08X}")

    # === Phase 4: Combined transfer - AP read + RDBUFF ===
    print(f"\n--- Combined DAP_Transfer: AP read + RDBUFF ---")
    r = sr([0x05, 0, 2, E(1,1,0), E(0,1,3)])
    if r:
        print(f"  Raw: {r[:16].hex()}")
        for i in range(min(14, len(r))):
            print(f"  [{i}] = 0x{r[i]:02X}")
        # Try interleaved: [echo, count, status0, data0(4B), status1, data1(4B)]
        if len(r) >= 12:
            s0 = r[2]
            d0 = struct.unpack_from('<I', r, 3)[0]
            s1 = r[7]
            d1 = struct.unpack_from('<I', r, 8)[0]
            print(f"  [interleaved] s0={s1:02X} stale={d0:08X} s1={s1:02X} rdbuff={d1:08X}")

    dev.close()

import time
time.sleep(1)
run()
