import re
from src.io.io_transport import IOTransport
from src.io.rtt_reader import RttReaderThread


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

    def _import_pylink(self):
        try:
            import pylink
            return pylink
        except ImportError:
            self.error_occurred.emit("未找到 pylink 库，请安装: pip install pylink")
            return None

    def get_available_devices(self):
        pylink = self._import_pylink()
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
        """RTT 专用连接入口。"""
        pylink = self._import_pylink()
        if pylink is None:
            return False

        with self._lock:
            if self.is_connected:
                self._disconnect_internal()
            try:
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
                self.is_connected = True
                self.connection_changed.emit(True)
                return True
            except Exception as e:
                self._disconnect_internal()
                self.error_occurred.emit(f"J-Link 连接失败: {str(e)}")
                return False

    def _close_resource(self):
        if self.jlink and self.jlink.opened():
            try:
                self.jlink.rtt_stop()
            except Exception:
                pass
            self.jlink.close()
        self.jlink = None

    def _send_bytes(self, data: bytes) -> bool:
        write_data = list(data)
        self.jlink.rtt_write(0, write_data)
        return True

    def get_serial_number(self):
        if self.jlink and self.jlink.opened():
            return self.jlink.serial_number
        return 0
