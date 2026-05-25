"""Full test: SWD init + power-up + AP read via production code."""
import sys, os
sys.path.insert(0, os.path.abspath('.'))

from src.io.cmsis_dap import CmsisDapProtocol

dap = CmsisDapProtocol()
dap.transport.open_vid_pid(0xC251, 0xF00A)
print(f"Open: {dap.is_open()}")

try:
    dap.connect_swd(reset=False)
    print("=== SWD Connected ===")

    idcode = dap.swd_read_dp(0)
    print(f"DP_IDCODE   = 0x{idcode:08X}")

    ctrl = dap.swd_read_dp(1)
    print(f"DP_CTRL_STAT = 0x{ctrl:08X}")
    print(f"  CDBGPWRUPACK={(ctrl>>30)&1} CSYSPWRUPACK={(ctrl>>28)&1}")

    from src.io.cmsis_dap import CSW_32BIT, CSW_ADDR_INC
    dap._ap_setup()
    csw = dap.swd_read_ap(0)
    print(f"AP_CSW      = 0x{csw:08X}")

    # Read AP_IDR with proper banksel
    from src.io.cmsis_dap import DP_SELECT
    dap.swd_write_dp(DP_SELECT, 3 << 0)  # APSEL=0, APBANKSEL=3
    idr = dap.swd_read_ap(0)
    print(f"AP_IDR      = 0x{idr:08X}")

    # Go back to bank 0
    dap.swd_write_dp(DP_SELECT, 0)
    dap._ap_setup()

    # Memory read test - try reading from 0x08000000 (Flash base)
    print("\n=== Memory read test ===")
    vals = dap.read_mem_block32(0x08000000, 8)
    if vals:
        print(f"Mem[0x08000000]: {' '.join(f'{v:08X}' for v in vals)}")

except Exception as e:
    import traceback
    traceback.print_exc()

dap.close()
print("Done")
