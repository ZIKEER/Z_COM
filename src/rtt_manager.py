import re
import time
import threading
from PySide6.QtCore import QObject, Signal, QThread


class RttReaderThread(QThread):
    """RTT 数据读取线程"""
    data_received = Signal(bytes)
    error_occurred = Signal(str)

    def __init__(self, jlink, buffer_idx=0, read_size=8192, read_interval=0.002):
        super().__init__()
        self.jlink = jlink
        self.buffer_idx = buffer_idx
        self.read_size = read_size
        self.read_interval = read_interval
        self.running = False

    def run(self):
        """线程运行函数"""
        self.running = True
        while self.running:
            try:
                if self.jlink.opened():
                    rtt_data = self.jlink.rtt_read(self.buffer_idx, self.read_size)
                    if rtt_data:
                        self.data_received.emit(bytes(rtt_data))
                self.msleep(int(self.read_interval * 1000))
            except Exception as e:
                if self.running:
                    self.error_occurred.emit(f"RTT读取错误: {str(e)}")
                    self.msleep(10)

        self.running = False

    def stop(self):
        """停止线程"""
        self.running = False
        self.wait(1000)
        if self.isRunning():
            self.terminate()


class RttManager(QObject):
    """RTT 管理类 - 封装 J-Link RTT 读写操作"""

    data_received = Signal(bytes)
    connection_changed = Signal(bool)
    error_occurred = Signal(str)

    def __init__(self):
        super().__init__()
        self.jlink = None
        self.is_connected = False
        self.reader_thread = None
        self._lock = threading.Lock()

        # 默认 RTT 设置
        self.settings = {
            'chip': '',
            'speed': 4000,
            'reset': False,
            'start_address': '',
            'range_size': '',
        }

    def _import_pylink(self):
        """延迟导入 pylink 模块"""
        try:
            import pylink
            return pylink
        except ImportError:
            self.error_occurred.emit("未找到 pylink 库，请安装: pip install pylink")
            return None

    def get_available_jlink_devices(self):
        """扫描已连接的 J-Link 设备列表

        Returns:
            list: [(serial_number, description), ...] 设备列表
        """
        pylink = self._import_pylink()
        if pylink is None:
            print("[RTT] pylink 导入失败，无法扫描 J-Link 设备")
            return []

        devices = []
        try:
            jlink_temp = pylink.JLink()
            # 获取已连接的 J-Link 序列号列表
            try:
                connected_emulators = jlink_temp.connected_emulators()
                print(f"[RTT] 扫描到 {len(connected_emulators)} 个 J-Link 设备")
            except Exception as e:
                print(f"[RTT] connected_emulators 调用失败: {e}")
                connected_emulators = []

            for emu in connected_emulators:
                try:
                    # 解析设备信息，格式如: "J-Link CE <Serial No. 69614015, Conn. USB>"
                    # 提取序列号
                    desc = str(emu)
                    sn = None
                    
                    # 尝试从 "Serial No. XXXXX" 中提取序列号
                    import re
                    match = re.search(r'Serial No\.\s*(\d+)', desc)
                    if match:
                        sn = int(match.group(1))
                    
                    if sn is not None:
                        # 尝试打开设备获取更多信息
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
                            # 即使打开失败，也添加到列表中
                            devices.append((sn, desc))
                    else:
                        print(f"[RTT] 无法解析设备序列号: {desc}")
                except Exception as e:
                    print(f"[RTT] 处理设备信息失败: {e}")
                    continue

            # 如果没有通过 connected_emulators 找到设备，尝试直接打开默认 J-Link
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

    def update_settings(self, settings):
        """更新 RTT 设置"""
        self.settings.update(settings)

    def connect(self, serial_no=None, chip=None, speed=None, reset_flag=None,
                start_address=None, range_size=None):
        """连接 J-Link 并启动 RTT

        Args:
            serial_no: J-Link 序列号，None 表示使用默认
            chip: 芯片型号
            speed: J-Link 速度 (kHz)
            reset_flag: 连接时是否复位
            start_address: RTT 搜索起始地址
            range_size: RTT 搜索范围大小

        Returns:
            bool: 连接是否成功
        """
        pylink = self._import_pylink()
        if pylink is None:
            return False

        with self._lock:
            # 确保之前的连接已断开
            if self.is_connected:
                self._disconnect_internal()

            try:
                # 使用传入参数或默认设置
                chip = chip or self.settings.get('chip', 'nRF52840_xxAA')
                speed = speed or self.settings.get('speed', 4000)
                reset_flag = reset_flag if reset_flag is not None else self.settings.get('reset', True)

                # 处理 RTT 地址范围
                if start_address is None:
                    addr_str = self.settings.get('start_address', '')
                    if addr_str and addr_str.strip():
                        start_address = int(addr_str, 16)
                if range_size is None:
                    range_str = self.settings.get('range_size', '')
                    if range_str and range_str.strip():
                        range_size = int(range_str, 16)

                # 创建 J-Link 连接
                self.jlink = pylink.JLink()

                # 打开 J-Link
                if serial_no:
                    self.jlink.open(serial_no=int(serial_no))
                else:
                    self.jlink.open()

                # 配置 J-Link
                self.jlink.set_tif(pylink.enums.JLinkInterfaces.SWD)
                self.jlink.set_speed(speed)
                self.jlink.connect(chip)

                # 复位 MCU
                if reset_flag:
                    self.jlink.reset(ms=10, halt=False)

                # 启动 RTT
                self.jlink.rtt_start(start_address)
                self.is_connected = True

                # 启动读取线程
                self.reader_thread = RttReaderThread(
                    self.jlink,
                    buffer_idx=0,  # 使用通道 0
                    read_size=8192,
                    read_interval=0.002
                )
                self.reader_thread.data_received.connect(self.data_received)
                self.reader_thread.error_occurred.connect(self._on_thread_error)
                self.reader_thread.finished.connect(self._on_thread_finished)
                self.reader_thread.start()

                self.connection_changed.emit(True)
                return True

            except Exception as e:
                self._disconnect_internal()
                self.error_occurred.emit(f"J-Link 连接失败: {str(e)}")
                return False

    def disconnect(self):
        """断开 J-Link 连接"""
        with self._lock:
            return self._disconnect_internal()

    def _disconnect_internal(self):
        """内部断开连接方法（需要已获取锁）"""
        try:
            # 停止读取线程
            if self.reader_thread and self.reader_thread.isRunning():
                self.reader_thread.stop()
                self.reader_thread = None

            if self.jlink and self.jlink.opened():
                try:
                    self.jlink.rtt_stop()
                except Exception:
                    pass
                self.jlink.close()

            self.jlink = None
            self.is_connected = False
            self.connection_changed.emit(False)
            return True
        except Exception as e:
            self.error_occurred.emit(f"断开失败: {str(e)}")
            return False

    def _on_thread_error(self, error_msg):
        """线程错误处理"""
        self.error_occurred.emit(error_msg)

    def _on_thread_finished(self):
        """线程结束处理"""
        if self.is_connected:
            # 线程意外结束，断开连接
            self.disconnect()

    def send_data(self, data, is_hex=False):
        """发送数据到 RTT 通道 0

        Args:
            data: 数据（bytes 或 str）
            is_hex: 是否为 HEX 格式

        Returns:
            bool: 发送是否成功
        """
        if not self.is_connected or not self.jlink:
            self.error_occurred.emit("J-Link 未连接")
            return False

        try:
            if is_hex:
                if isinstance(data, str):
                    hex_str = data.replace(' ', '').replace('\n', '')
                    bytes_data = bytes.fromhex(hex_str)
                else:
                    bytes_data = data
            else:
                if isinstance(data, str):
                    bytes_data = data.encode('utf-8')
                else:
                    bytes_data = data

            # 将 bytes 转换为 pylink 需要的格式（list of int）
            write_data = list(bytes_data)
            self.jlink.rtt_write(0, write_data)
            return True
        except Exception as e:
            self.error_occurred.emit(f"RTT 发送失败: {str(e)}")
            return False

    def get_serial_number(self):
        """获取当前连接的 J-Link 序列号"""
        if self.jlink and self.jlink.opened():
            return self.jlink.serial_number
        return 0
