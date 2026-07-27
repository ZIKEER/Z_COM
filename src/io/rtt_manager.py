import re
import time
from src.io.io_transport import IOTransport
from src.io.rtt_reader import RttReaderThread


RTT_WRITE_TIMEOUT_SECONDS = 0.1
RTT_WRITE_RETRY_INTERVAL_SECONDS = 0.001


class RttManager(IOTransport):
    """RTT 管理类 - 封装 J-Link RTT 读写操作"""

    def __init__(self):
        super().__init__()
        self.jlink = None
        self.settings = {
            'chip': '',
            'speed': 4000,
            'reset': False,
            'start_address': '',
            'range_size': '',
            'frame_timeout': 50,
        }

    def _import_pylink(self, emit_error=True):
        try:
            import pylink
            return pylink
        except ImportError:
            if emit_error:
                self.error_occurred.emit("未找到 pylink 库，请安装: pip install pylink")
            return None

    def get_available_devices(self):
        pylink = self._import_pylink(emit_error=False)
        if pylink is None:
            print("[RTT] pylink 导入失败，无法扫描 J-Link 设备")
            return []

        devices = []
        try:
            jlink_temp = pylink.JLink()
            try:
                connected_emulators = jlink_temp.connected_emulators()
                print(f"[RTT] 扫描到 {len(connected_emulators)} 个 J-Link 设备")
            except Exception as e:
                print(f"[RTT] connected_emulators 调用失败: {e}")
                connected_emulators = []

            for emu in connected_emulators:
                try:
                    desc = str(emu)
                    sn = None
                    match = re.search(r'Serial No\.\s*(\d+)', desc)
                    if match:
                        sn = int(match.group(1))

                    if sn is not None:
                        try:
                            jlink_temp.open(serial_no=sn)
                            jlink_name = jlink_temp.product_name if hasattr(jlink_temp, 'product_name') else 'J-Link'
                            jlink_temp.close()
                            devices.append((sn, f"{jlink_name} (SN={sn})"))
                            print(f"[RTT] 发现 J-Link: SN={sn}, 名称={jlink_name}")
                        except Exception as e:
                            print(f"[RTT] 打开 J-Link SN={sn} 失败: {e}")
                            try:
                                jlink_temp.close()
                            except Exception:
                                pass
                            devices.append((sn, desc))
                    else:
                        print(f"[RTT] 无法解析设备序列号: {desc}")
                except Exception as e:
                    print(f"[RTT] 处理设备信息失败: {e}")
                    continue

            if not devices:
                try:
                    jlink_temp.open()
                    sn = jlink_temp.serial_number
                    jlink_name = jlink_temp.product_name if hasattr(jlink_temp, 'product_name') else 'J-Link'
                    jlink_temp.close()
                    devices.append((sn, f"{jlink_name} (SN={sn})"))
                    print(f"[RTT] 通过默认方式发现 J-Link: SN={sn}")
                except Exception as e:
                    print(f"[RTT] 默认方式打开 J-Link 失败: {e}")
                    try:
                        jlink_temp.close()
                    except Exception:
                        pass

        except Exception as e:
            print(f"[RTT] J-Link 扫描异常: {e}")

        print(f"[RTT] 共发现 {len(devices)} 个 J-Link 设备")
        return devices

    def open_connection(self, serial_no=None, chip=None, speed=None, reset_flag=None,
                        start_address=None, range_size=None):
        return super().open_connection(
            serial_no=serial_no,
            chip=chip,
            speed=speed,
            reset_flag=reset_flag,
            start_address=start_address,
            range_size=range_size,
        )

    def _connect_impl(self, serial_no=None, chip=None, speed=None, reset_flag=None,
                      start_address=None, range_size=None):
        pylink = self._import_pylink(emit_error=False)
        if pylink is None:
            raise RuntimeError("未找到 pylink 库，请安装: pip install pylink")

        chip = chip or self.settings.get('chip', 'nRF52840_xxAA')
        speed = speed or self.settings.get('speed', 4000)
        reset_flag = reset_flag if reset_flag is not None else self.settings.get('reset', True)

        if start_address is None:
            addr_str = self.settings.get('start_address', '')
            if addr_str and addr_str.strip():
                start_address = int(addr_str, 16)
        if range_size is None:
            range_str = self.settings.get('range_size', '')
            if range_str and range_str.strip():
                range_size = int(range_str, 16)

        self.jlink = pylink.JLink()
        if serial_no:
            self.jlink.open(serial_no=int(serial_no))
        else:
            self.jlink.open()

        self.jlink.set_tif(pylink.enums.JLinkInterfaces.SWD)
        self.jlink.set_speed(speed)
        self.jlink.connect(chip)

        if reset_flag:
            self.jlink.reset(ms=10, halt=False)

        self.jlink.rtt_start(start_address)

        frame_timeout = self.settings.get('frame_timeout', 50) / 1000.0
        thread = RttReaderThread(
            self.jlink, buffer_idx=0, read_size=8192,
            read_interval=0.002, frame_timeout=frame_timeout,
        )
        self._connect_reader_thread(thread)

    def _close_resource(self):
        if self.jlink and self.jlink.opened():
            try:
                self.jlink.rtt_stop()
            except Exception:
                pass
            self.jlink.close()
        self.jlink = None

    def _send_bytes(self, data: bytes) -> bool:
        offset = 0
        deadline = time.monotonic() + RTT_WRITE_TIMEOUT_SECONDS
        while offset < len(data):
            written = self.jlink.rtt_write(0, list(data[offset:]))
            if not isinstance(written, int) or written < 0 or written > len(data) - offset:
                raise RuntimeError(f"RTT 返回了无效的写入长度: {written}")
            if written == 0:
                if time.monotonic() >= deadline:
                    raise TimeoutError("RTT 写入超时，目标缓冲区可能已满")
                time.sleep(RTT_WRITE_RETRY_INTERVAL_SECONDS)
                continue
            offset += written
        return True

    def get_serial_number(self):
        if self.jlink and self.jlink.opened():
            return self.jlink.serial_number
        return 0
