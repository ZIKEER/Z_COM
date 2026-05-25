"""Test: dump raw DAP_Transfer bytes for AP read."""
import sys, os, struct
sys.path.insert(0, os.path.abspath('.'))

from src.io.cmsis_dap import CmsisDapProtocol, encode_request_byte, DP_RDBUFF, DAP_TRANSFER

dap = CmsisDapProtocol()
dap.transport.open_vid_pid(0xC251, 0xF00A)
dap.dap_connect()

# Do SWD init manually using raw HID calls
import time
# Line reset
dap._send_cmd([0x12, 12*8] + [0xFF]*12)
print(f"SWJ_SEQ(96): {dap._recv_resp().hex()}")
dap._send_cmd([0x12, 16] + [0x9E, 0xE7])
print(f"SWJ_SEQ(switch): {dap._recv_resp().hex()}")
dap._send_cmd([0x12, 12*8] + [0xFF]*12)
print(f"SWJ_SEQ(96#2): {dap._recv_resp().hex()}")

# Idle cycle
dap._send_cmd([0x12, 8] + [0x00])
print(f"SWJ_SEQ(idle): {dap._recv_resp().hex()}")

dap.dap_swd_configure(0)

# Read DP_IDCODE - single request
req = encode_request_byte(0, 1, 0)
dap._send_cmd([DAP_TRANSFER, 0, 1, req])
resp = dap._recv_resp()
print(f"DP_IDCODE raw ({len(resp)}B): {resp.hex()}")
# Try to parse manually
print(f"  [0]=echo, [1]=count, [2]=status, [3:7]=data")
print(f"  echo=0x{resp[0]:02X} count={resp[1]} status=0x{resp[2]:02X} data={resp[3:7].hex()}")

# Read AP_CSW: [req_ap, req_rd]
req_ap = encode_request_byte(1, 1, 0)   # AP read CSW
req_rd = encode_request_byte(0, 1, DP_RDBUFF)  # DP read RDBUFF
dap._send_cmd([DAP_TRANSFER, 0, 2, req_ap, req_rd])
resp = dap._recv_resp()
print(f"\nAP_CSW raw ({len(resp)}B): {resp.hex()}")
print(f"  echo=0x{resp[0]:02X} count={resp[1]}")
if len(resp) > 2:
    print(f"  Byte[2]=0x{resp[2]:02X} Byte[3]=0x{resp[3]:02X} Byte[4]=0x{resp[4]:02X} Byte[5]=0x{resp[5]:02X}")

# Try parsing as interleaved: [status0, data0x4, status1, data1x4]
if len(resp) >= 12:
    status0 = resp[2]
    data0 = struct.unpack_from('<I', resp, 3)[0]
    status1 = resp[7]
    data1 = struct.unpack_from('<I', resp, 8)[0]
    print(f"  [interleaved] status0=0x{status0:02X} data0=0x{data0:08X} status1=0x{status1:02X} data1=0x{data1:08X}")
else:
    print("  (not enough bytes for interleaved)")

# Try parsing as sequential: [status0, status1, data0x4, data1x4]
if len(resp) >= 12:
    status0 = resp[2]
    status1 = resp[3]
    data0 = struct.unpack_from('<I', resp, 4)[0]
    data1 = struct.unpack_from('<I', resp, 8)[0]
    print(f"  [sequential]  status0=0x{status0:02X} status1=0x{status1:02X} data0=0x{data0:08X} data1=0x{data1:08X}")

dap.close()
