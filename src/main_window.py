import sys
import os
import time
from datetime import datetime
from PySide6.QtWidgets import QMainWindow, QMessageBox, QVBoxLayout, QLabel, QComboBox, QPushButton, QHBoxLayout, QWidget
from PySide6.QtCore import QTimer, Signal, QObject, QThread, Qt
from PySide6.QtGui import QTextCursor, QIcon
import serial
import serial.tools.list_ports

# 添加项目根目录到路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from ui.Ui_main_window import Ui_MainWindow
from src.serial_settings_dialog import SerialSettingsDialog
from src.config_manager import ConfigManager
from src.extended_send_manager import ExtendedSendManager
from src.extended_send_widget import ExtendedSendWidget
from src.version import VERSION, APP_NAME, ICON_PATH, BUILD_TIME
from src.rtt_manager import RttManager


def get_resource_path(relative_path):
    """获取资源文件的绝对路径，支持打包后的路径"""
    if getattr(sys, 'frozen', False):
        # 打包后的路径
        base_path = sys._MEIPASS
    else:
        # 开发环境路径
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base_path, relative_path)


class SerialReaderThread(QThread):
    """串口读取线程"""
    data_received = Signal(bytes)
    error_occurred = Signal(str)
    
    def __init__(self, serial_port, frame_timeout=50):
        super().__init__()
        self.serial_port = serial_port
        self.running = False
        self.buffer = bytearray()
        self.frame_timeout = frame_timeout / 1000.0  # 转换为秒
        self.last_receive_time = 0
        self._lock = __import__('threading').Lock()
    
    def run(self):
        """线程运行函数"""
        self.running = True
        while self.running:
            try:
                if not self.serial_port.is_open:
                    break
                
                if self.serial_port.in_waiting:
                    with self._lock:
                        data = self.serial_port.read(self.serial_port.in_waiting)
                    current_time = time.time()
                    
                    # 如果距离上次接收时间超过拼包超时，先发送缓冲区数据
                    if self.buffer and (current_time - self.last_receive_time) > self.frame_timeout:
                        self.data_received.emit(bytes(self.buffer))
                        self.buffer.clear()
                    
                    self.buffer.extend(data)
                    self.last_receive_time = current_time
                else:
                    # 检查缓冲区是否有数据需要发送
                    if self.buffer and self.last_receive_time > 0:
                        if (time.time() - self.last_receive_time) > self.frame_timeout:
                            self.data_received.emit(bytes(self.buffer))
                            self.buffer.clear()
                    
                    self.msleep(1)  # 短暂休眠避免CPU占用过高
            except serial.SerialException as e:
                self.error_occurred.emit(f"串口错误: {str(e)}")
                break
            except Exception as e:
                if self.running:
                    self.error_occurred.emit(f"读取错误: {str(e)}")
                    self.msleep(10)
        
        self.running = False
    
    def stop(self):
        """停止线程"""
        self.running = False
        # 发送缓冲区中剩余的数据
        with self._lock:
            if self.buffer:
                self.data_received.emit(bytes(self.buffer))
                self.buffer.clear()
        self.wait(1000)  # 等待最多1秒
        if self.isRunning():
            self.terminate()  # 强制终止


class SerialManager(QObject):
    """串口管理类"""
    data_received = Signal(bytes)
    connection_changed = Signal(bool)
    error_occurred = Signal(str)
    
    def __init__(self):
        super().__init__()
        self.serial = serial.Serial()
        self.is_connected = False
        self.reader_thread = None
        self._lock = __import__('threading').Lock()
        
        # 默认串口设置
        self.settings = {
            'baudrate': 115200,
            'databits': 8,
            'stopbits': 1,
            'parity': 'None',
            'flowcontrol': 'None',
            'frame_timeout': 50  # 拼包超时时间（毫秒）
        }
    
    def get_available_ports(self):
        """获取可用串口列表"""
        ports = serial.tools.list_ports.comports()
        return [(p.device, p.description) for p in ports]
    
    def update_settings(self, settings):
        """更新串口设置"""
        self.settings.update(settings)
    
    def reconfigure(self):
        """热更新串口参数（无需关闭/重新打开串口）"""
        with self._lock:
            if not self.is_connected or not self.serial.is_open:
                return False
            
            try:
                # 更新波特率
                self.serial.baudrate = self.settings['baudrate']
                # 更新数据位
                self.serial.bytesize = self.settings['databits']
                # 更新停止位
                stopbits_map = {1: serial.STOPBITS_ONE, 1.5: serial.STOPBITS_ONE_POINT_FIVE, 2: serial.STOPBITS_TWO}
                self.serial.stopbits = stopbits_map.get(self.settings['stopbits'], serial.STOPBITS_ONE)
                # 更新校验位
                parity_map = {
                    'None': serial.PARITY_NONE,
                    'Even': serial.PARITY_EVEN,
                    'Odd': serial.PARITY_ODD,
                    'Mark': serial.PARITY_MARK,
                    'Space': serial.PARITY_SPACE
                }
                self.serial.parity = parity_map.get(self.settings['parity'], serial.PARITY_NONE)
                return True
            except Exception as e:
                self.error_occurred.emit(f"重新配置失败: {str(e)}")
                return False
    
    def connect(self, port):
        """连接串口"""
        with self._lock:
            # 确保之前的连接已断开
            if self.is_connected:
                self._disconnect_internal()
            
            try:
                self.serial.port = port
                self.serial.baudrate = self.settings['baudrate']
                self.serial.bytesize = self.settings['databits']
                
                stopbits_map = {1: serial.STOPBITS_ONE, 1.5: serial.STOPBITS_ONE_POINT_FIVE, 2: serial.STOPBITS_TWO}
                self.serial.stopbits = stopbits_map.get(self.settings['stopbits'], serial.STOPBITS_ONE)
                
                parity_map = {
                    'None': serial.PARITY_NONE,
                    'Even': serial.PARITY_EVEN,
                    'Odd': serial.PARITY_ODD,
                    'Mark': serial.PARITY_MARK,
                    'Space': serial.PARITY_SPACE
                }
                self.serial.parity = parity_map.get(self.settings['parity'], serial.PARITY_NONE)
                
                self.serial.open()
                self.is_connected = True
                
                # 启动读取线程
                frame_timeout = self.settings.get('frame_timeout', 50)
                self.reader_thread = SerialReaderThread(self.serial, frame_timeout)
                self.reader_thread.data_received.connect(self.data_received)
                self.reader_thread.error_occurred.connect(self._on_thread_error)
                self.reader_thread.finished.connect(self._on_thread_finished)
                self.reader_thread.start()
                
                self.connection_changed.emit(True)
                return True
            except Exception as e:
                self._disconnect_internal()
                self.error_occurred.emit(f"连接失败: {str(e)}")
                return False
    
    def disconnect(self):
        """断开串口"""
        with self._lock:
            return self._disconnect_internal()
    
    def _disconnect_internal(self):
        """内部断开连接方法（需要已获取锁）"""
        try:
            # 停止读取线程
            if self.reader_thread and self.reader_thread.isRunning():
                self.reader_thread.stop()
                self.reader_thread = None
            
            if self.serial.is_open:
                self.serial.close()
            
            self.is_connected = False
            self.connection_changed.emit(False)
            return True
        except Exception as e:
            self.error_occurred.emit(f"断开失败: {str(e)}")
            return False
    
    def _on_thread_error(self, error_msg):
        """线程错误处理"""
        self.error_occurred.emit(error_msg)
        # 自动断开连接
        self.disconnect()
    
    def _on_thread_finished(self):
        """线程结束处理"""
        if self.is_connected:
            # 线程意外结束，断开连接
            self.disconnect()
    
    def send_data(self, data, is_hex=False):
        """发送数据"""
        try:
            if is_hex:
                hex_str = data.replace(' ', '').replace('\n', '')
                bytes_data = bytes.fromhex(hex_str)
            else:
                bytes_data = data
            
            self.serial.write(bytes_data)
            return True
        except Exception as e:
            self.error_occurred.emit(str(e))
            return False


class DataHandler:
    """数据处理类"""
    
    # 不可见字符映射表
    CONTROL_CHAR_MAP = {
        0x00: '␀',  # NUL
        0x01: '␁',  # SOH
        0x02: '␂',  # STX
        0x03: '␃',  # ETX
        0x04: '␄',  # EOT
        0x05: '␅',  # ENQ
        0x06: '␆',  # ACK
        0x07: '␇',  # BEL
        0x08: '␈',  # BS
        0x09: '␉',  # HT (Tab)
        0x0A: '␊',  # LF (换行)
        0x0B: '␋',  # VT
        0x0C: '␌',  # FF
        0x0D: '␍',  # CR (回车)
        0x0E: '␎',  # SO
        0x0F: '␏',  # SI
        0x10: '␐',  # DLE
        0x11: '␑',  # DC1
        0x12: '␒',  # DC2
        0x13: '␓',  # DC3
        0x14: '␔',  # DC4
        0x15: '␕',  # NAK
        0x16: '␖',  # SYN
        0x17: '␗',  # ETB
        0x18: '␘',  # CAN
        0x19: '␙',  # EM
        0x1A: '␚',  # SUB
        0x1B: '␛',  # ESC
        0x1C: '␜',  # FS
        0x1D: '␝',  # GS
        0x1E: '␞',  # RS
        0x1F: '␟',  # US
        0x7F: '␡',  # DEL
    }
    
    @staticmethod
    def bytes_to_hex(data):
        """字节数据转 HEX 字符串"""
        return ' '.join(f'{b:02X}' for b in data)
    
    @staticmethod
    def bytes_to_ascii(data):
        """字节数据转 ASCII 字符串，不可见字符用特殊符号显示"""
        result = []
        for b in data:
            if b in DataHandler.CONTROL_CHAR_MAP:
                result.append(DataHandler.CONTROL_CHAR_MAP[b])
                # LF 和 CR 后面加实际换行，方便阅读
                if b == 0x0A:  # LF
                    result.append('\n')
            elif 0x20 <= b < 0x7F:
                # 可见 ASCII 字符
                result.append(chr(b))
            else:
                # 其他不可见字符（高位字节）
                result.append(f'\\x{b:02x}')
        return ''.join(result)
    
    @staticmethod
    def format_display(data, mode):
        """格式化显示数据"""
        if mode == 'HEX':
            return DataHandler.bytes_to_hex(data)
        elif mode == 'ASCII':
            return DataHandler.bytes_to_ascii(data)
        elif mode == 'MIXED':
            hex_str = DataHandler.bytes_to_hex(data)
            ascii_str = DataHandler.bytes_to_ascii(data)
            return f"\n ← HEX: {hex_str}\n ← ASCII: {ascii_str}"
        return ''
    
    @staticmethod
    def validate_hex_input(text):
        """验证 HEX 输入"""
        hex_str = text.replace(' ', '').replace('\n', '')
        try:
            bytes.fromhex(hex_str)
            return True
        except ValueError:
            return False


class Logger:
    """日志管理类"""
    
    def __init__(self, log_dir=None, instance_id=1):
        if log_dir:
            self.log_dir = log_dir
        elif instance_id > 1:
            self.log_dir = os.path.join(f"instance_{instance_id}", "logs")
        else:
            self.log_dir = "logs"
        
        os.makedirs(self.log_dir, exist_ok=True)
        self.current_log_file = None
        self._update_log_file()
    
    def _update_log_file(self):
        """更新日志文件（按日期创建）"""
        date_str = datetime.now().strftime("%Y-%m-%d")
        filename = f"log_{date_str}.txt"
        self.current_log_file = os.path.join(self.log_dir, filename)
    
    def log(self, data, direction, timestamp, display_mode='ASCII'):
        """记录数据
        
        Args:
            data: 数据内容
            direction: 方向 'RECEIVE' 或 'SEND'
            timestamp: 时间戳（必须传入）
            display_mode: 显示模式
        """
        self._update_log_file()
        
        arrow = "←" if direction == "RECEIVE" else "→"
        
        ascii_data = DataHandler.bytes_to_ascii(data)
        hex_data = DataHandler.bytes_to_hex(data)
        
        log_entry = f"[{timestamp}]\n"
        log_entry += f" {arrow} HEX: {hex_data}\n"
        log_entry += f" {arrow} ASCII: {ascii_data}\n"
        
        with open(self.current_log_file, 'a', encoding='utf-8') as f:
            f.write(log_entry)


class MainWindow(QMainWindow):
    """主窗口类"""
    
    def __init__(self, instance_id=1):
        super().__init__()
        self.instance_id = instance_id
        self.ui = Ui_MainWindow()
        self.ui.setupUi(self)
        
        # 设置窗口标题（包含版本号和实例号）
        title = f"{APP_NAME} V{VERSION}"
        if instance_id > 1:
            title += f" [实例{instance_id}]"
        self.setWindowTitle(title)
        
        # 设置窗口图标
        icon_path = get_resource_path(ICON_PATH)
        if os.path.exists(icon_path):
            self.setWindowIcon(QIcon(icon_path))
        
        # 初始化配置管理
        self.config_manager = ConfigManager(instance_id=instance_id)
        
        # 初始化管理器
        self.serial_manager = SerialManager()
        self.rtt_manager = RttManager()
        self.data_handler = DataHandler()
        self.logger = Logger(instance_id=instance_id)
        
        # 初始化扩展发送管理器，注入统一的发送函数
        self.extended_send_manager = ExtendedSendManager(self._send_data_func)
        
        # 创建扩展发送面板
        self.extended_send_widget = ExtendedSendWidget(self.extended_send_manager)
        
        # 将扩展发送面板添加到容器中
        container_layout = QVBoxLayout(self.ui.extendedSendContainer)
        container_layout.setContentsMargins(0, 0, 0, 0)
        container_layout.addWidget(self.extended_send_widget)
        
        # 状态变量
        self.receive_count = 0
        self.send_count = 0
        self.is_rtt_mode = False  # 是否为 RTT 模式
        self.auto_send_timer = QTimer()
        self.auto_send_timer.timeout.connect(self._auto_send)
        
        # 初始化界面
        self._init_ui()
        self._setup_connections()
        self._load_config()
    
    def _init_ui(self):
        """初始化界面"""
        # 创建状态栏
        self._create_status_bar()
        
        # 设置默认显示模式
        self.ui.asciiRadio.setChecked(True)
        
        # 设置默认发送格式
        self.ui.sendAsciiRadio.setChecked(True)
        
        # 设置打开串口按钮初始颜色（红色-未连接）
        self.ui.openButton.setStyleSheet("background-color: #F44336; color: white; font-weight: bold;")
        
        # 设置接收区域使用等宽字体
        from PySide6.QtGui import QFont
        mono_font = QFont("Consolas", 10)
        mono_font.setStyleHint(QFont.StyleHint.Monospace)
        self.ui.receiveTextEdit.setFont(mono_font)
        
        # 设置分割器初始比例
        # 上部分（接收+扩展）和下部分（发送）的比例为5:1
        self.ui.mainSplitter.setSizes([500, 100])
        
        # 接收区域和扩展发送区域的宽度比例为1.55:1
        self.ui.topSplitter.setSizes([550, 340])
        
        # 设置发送区域中间部分的两行比例为1:4
        self.ui.sendCenterLayout.setStretch(0, 1)  # 配置项行
        self.ui.sendCenterLayout.setStretch(1, 4)  # 发送文本框行
    
    def _create_status_bar(self):
        """创建状态栏"""
        # 创建自定义状态栏
        status_widget = QWidget()
        status_layout = QHBoxLayout(status_widget)
        status_layout.setContentsMargins(3, 0, 3, 0)
        status_layout.setSpacing(6)
        
        # 左侧：串口配置
        # 刷新按钮
        self.refresh_button = QPushButton("刷新串口")
        self.refresh_button.setMaximumWidth(70)
        status_layout.addWidget(self.refresh_button)
        
        # 串口选择
        port_label = QLabel("串口:")
        status_layout.addWidget(port_label)
        
        self.port_combo = QComboBox()
        self.port_combo.setMinimumWidth(200)
        self.port_combo.setSizeAdjustPolicy(QComboBox.SizeAdjustPolicy.AdjustToContents)
        self.port_combo.setToolTip("选择串口或J-Link设备")
        status_layout.addWidget(self.port_combo)
        
        # 波特率选择
        baudrate_label = QLabel("波特率:")
        status_layout.addWidget(baudrate_label)
        
        self.baudrate_combo = QComboBox()
        self.baudrate_combo.setMinimumWidth(80)
        baudrates = ['9600', '19200', '38400', '57600', '115200', '230400', '460800', '921600']
        self.baudrate_combo.addItems(baudrates)
        self.baudrate_combo.setCurrentText('115200')
        status_layout.addWidget(self.baudrate_combo)
        
        # 更多设置按钮
        settings_button = QPushButton("更多设置")
        settings_button.setMaximumWidth(70)
        status_layout.addWidget(settings_button)
        
        # 扩展发送按钮
        self.toggle_preset_button = QPushButton("扩展发送")
        self.toggle_preset_button.setCheckable(True)
        self.toggle_preset_button.setChecked(False)
        self.toggle_preset_button.setMaximumWidth(70)
        status_layout.addWidget(self.toggle_preset_button)
        
        # 中间弹簧
        status_layout.addStretch()
        
        # 右侧：状态信息
        self.status_label = QLabel("已断开")
        self.status_label.setStyleSheet("color: red;")
        self.status_label.setMinimumWidth(150)
        self.status_label.setMaximumWidth(150)
        status_layout.addWidget(self.status_label)
        
        separator1 = QLabel("|")
        status_layout.addWidget(separator1)
        
        self.send_count_label = QLabel("发送: 0 字节")
        status_layout.addWidget(self.send_count_label)
        
        separator2 = QLabel("|")
        status_layout.addWidget(separator2)
        
        self.receive_count_label = QLabel("接收: 0 字节")
        status_layout.addWidget(self.receive_count_label)
        
        # 添加到主窗口底部
        self.statusBar().setStyleSheet("QStatusBar::item{border:0}")
        self.statusBar().addPermanentWidget(status_widget, 1)
        self.statusBar().show()
        
        # 保存引用
        self.settings_button = settings_button
    
    def _setup_connections(self):
        """设置信号连接"""
        # 状态栏按钮连接
        self.refresh_button.clicked.connect(self._refresh_ports)
        self.settings_button.clicked.connect(self._show_serial_settings)
        self.baudrate_combo.currentTextChanged.connect(self._on_baudrate_changed)
        self.toggle_preset_button.clicked.connect(self._toggle_preset_panel)
        
        # 发送区域的打开串口按钮
        self.ui.openButton.clicked.connect(self._toggle_serial)
        
        # 数据发送
        self.ui.sendButton.clicked.connect(self._send_data)
        
        # 接收区域控制
        self.ui.clearReceiveButton.clicked.connect(self._clear_receive)
        
        # 自动发送
        self.ui.autoSendCheckBox.stateChanged.connect(self._toggle_auto_send)
        
        # 配置改变时立即保存
        self.port_combo.currentIndexChanged.connect(lambda: self._save_config_item('port'))
        self.port_combo.currentIndexChanged.connect(self._on_port_changed)
        self.baudrate_combo.currentTextChanged.connect(lambda: self._save_config_item('baudrate'))
        self.ui.hexRadio.toggled.connect(lambda: self._save_config_item('display_mode'))
        self.ui.asciiRadio.toggled.connect(lambda: self._save_config_item('display_mode'))
        self.ui.mixedRadio.toggled.connect(lambda: self._save_config_item('display_mode'))
        self.ui.sendAsciiRadio.toggled.connect(lambda: self._save_config_item('send_mode'))
        self.ui.sendHexRadio.toggled.connect(lambda: self._save_config_item('send_mode'))
        self.ui.autoScrollCheckBox.stateChanged.connect(lambda: self._save_config_item('auto_scroll'))
        self.ui.intervalSpinBox.valueChanged.connect(lambda: self._save_config_item('auto_send_interval'))
        
        # 串口管理器信号
        self.serial_manager.data_received.connect(self._on_data_received)
        self.serial_manager.connection_changed.connect(self._on_connection_changed)
        self.serial_manager.error_occurred.connect(self._on_error)
        
        # RTT 管理器信号
        self.rtt_manager.data_received.connect(self._on_data_received)
        self.rtt_manager.connection_changed.connect(self._on_connection_changed)
        self.rtt_manager.error_occurred.connect(self._on_error)
        
        # 扩展发送管理器信号
        self.extended_send_manager.data_sent.connect(self._on_extended_data_sent)
        
        # 菜单动作
        self.ui.actionExit.triggered.connect(self.close)
        self.ui.actionClearReceive.triggered.connect(self._clear_receive)
        self.ui.actionClearSend.triggered.connect(self._clear_send)
        self.ui.actionSettings.triggered.connect(self._show_serial_settings)
        self.ui.togglePresetAction.triggered.connect(self._toggle_preset_panel_menu)
        self.ui.actionAbout.triggered.connect(self._show_about)
    
    def _refresh_ports(self, block_signals=False):
        """刷新串口列表"""
        # 保存当前选中的端口
        current_port = self.port_combo.currentData() if not block_signals else None
        
        # 临时阻塞信号防止触发保存
        if block_signals:
            self.port_combo.blockSignals(True)
        
        self.port_combo.clear()
        ports = self.serial_manager.get_available_ports()
        for port, description in ports:
            # 移除描述中可能包含的括号中的端口号，避免重复显示
            # 例如: "USB-SERIAL CH340 (COM3)" -> "USB-SERIAL CH340"
            full_description = description
            if '(' in description and ')' in description:
                description = description.split('(')[0].strip()
            display_text = f"{port}-{description}"
            self.port_combo.addItem(display_text, port)
            # 设置 tooltip 显示完整信息
            index = self.port_combo.count() - 1
            self.port_combo.setItemData(index, f"{port}-{full_description}", Qt.ToolTipRole)
        
        # 恢复信号
        if block_signals:
            self.port_combo.blockSignals(False)
        
        # 在后台线程扫描 J-Link 设备，避免阻塞 UI
        from PySide6.QtCore import QThread, Signal as QSignal
        
        class JLinkScanThread(QThread):
            """J-Link 扫描线程"""
            scan_finished = QSignal(list)
            
            def __init__(self, rtt_manager):
                super().__init__()
                self.rtt_manager = rtt_manager
            
            def run(self):
                devices = self.rtt_manager.get_available_jlink_devices()
                self.scan_finished.emit(devices)
        
        self._jlink_scan_thread = JLinkScanThread(self.rtt_manager)
        self._jlink_scan_thread.scan_finished.connect(self._on_jlink_scan_finished)
        self._jlink_scan_thread.start()
    
    def _on_jlink_scan_finished(self, jlink_devices):
        """J-Link 扫描完成回调"""
        for sn, description in jlink_devices:
            jlink_key = f"JLINK:SN={sn}"
            # 检查是否已存在
            if self.port_combo.findData(jlink_key) < 0:
                display_text = f"{jlink_key} - {description}"
                self.port_combo.addItem(display_text, jlink_key)
                # 设置 tooltip 显示完整信息
                index = self.port_combo.count() - 1
                self.port_combo.setItemData(index, display_text, Qt.ToolTipRole)
    
    def _show_serial_settings(self):
        """显示串口设置对话框"""
        rtt_settings = self.config_manager.get_rtt_settings()
        dialog = SerialSettingsDialog(self.serial_manager.settings, rtt_settings, self)
        dialog.settings_changed.connect(self._on_serial_settings_changed)
        dialog.rtt_settings_changed.connect(self._on_rtt_settings_changed)
        dialog.exec()
    
    def _on_serial_settings_changed(self, settings):
        """串口设置改变"""
        self.serial_manager.update_settings(settings)
        # 如果串口已连接，立即生效
        if self.serial_manager.is_connected:
            self.serial_manager.reconfigure()
    
    def _on_rtt_settings_changed(self, settings):
        """RTT 设置改变"""
        self.rtt_manager.update_settings(settings)
        # 保存 RTT 配置
        self.config_manager.set('rtt_chip', settings.get('chip', 'nRF52840_xxAA'))
        self.config_manager.set('rtt_speed', settings.get('speed', 4000))
        self.config_manager.set('rtt_reset', settings.get('reset', True))
        self.config_manager.set('rtt_start_address', settings.get('start_address', ''))
        self.config_manager.set('rtt_range_size', settings.get('range_size', ''))
        self.config_manager.save()
    
    def _on_baudrate_changed(self, text):
        """波特率改变"""
        try:
            baudrate = int(text)
            self.serial_manager.settings['baudrate'] = baudrate
            # 如果串口已连接，立即生效
            if self.serial_manager.is_connected:
                self.serial_manager.reconfigure()
        except ValueError:
            pass
    
    def _on_port_changed(self, index):
        """端口选择改变"""
        if index < 0:
            return
        port_data = self.port_combo.currentData()
        # 判断是否为 J-Link 设备
        is_jlink = port_data and port_data.startswith('JLINK:')
        self.baudrate_combo.setEnabled(not is_jlink)
    
    def _is_jlink_port(self):
        """判断当前选中端口是否为 J-Link 设备"""
        if self.port_combo.currentIndex() < 0:
            return False
        port_data = self.port_combo.currentData()
        return port_data and port_data.startswith('JLINK:')
    
    def _toggle_serial(self):
        """切换串口连接状态"""
        if self.serial_manager.is_connected or self.rtt_manager.is_connected:
            # 断开当前连接
            if self.is_rtt_mode:
                self.rtt_manager.disconnect()
            else:
                self.serial_manager.disconnect()
        else:
            if self.port_combo.currentIndex() < 0:
                QMessageBox.warning(self, '警告', '请先选择串口')
                return
            
            port = self.port_combo.currentData()
            
            # 判断是否为 J-Link 设备
            if port and port.startswith('JLINK:'):
                # 解析 J-Link 序列号
                sn = port.replace('JLINK:SN=', '')
                rtt_settings = self.config_manager.get_rtt_settings()
                chip = rtt_settings.get('chip', '')
                success = self.rtt_manager.connect(
                    serial_no=sn,
                    chip=chip,
                    speed=rtt_settings.get('speed'),
                    reset_flag=rtt_settings.get('reset'),
                    start_address=rtt_settings.get('start_address') or None,
                    range_size=rtt_settings.get('range_size') or None
                )
                if success:
                    self.is_rtt_mode = True
                    # 保存芯片到历史记录
                    if chip:
                        self.config_manager.add_rtt_chip_history(chip)
            else:
                # 普通串口连接
                self.serial_manager.connect(port)
    
    def _on_connection_changed(self, connected):
        """串口连接状态改变"""
        if connected:
            if self.is_rtt_mode:
                self.ui.openButton.setText('关闭\nRTT')
            else:
                self.ui.openButton.setText('关闭\n串口')
            self.ui.openButton.setStyleSheet("background-color: #4CAF50; color: white; font-weight: bold;")
            self.status_label.setText(f'已连接 {self.port_combo.currentText()}')
            self.status_label.setStyleSheet("color: green;")
            # 保护锁：串口打开时禁用刷新按钮和串口选择
            self.refresh_button.setEnabled(False)
            self.port_combo.setEnabled(False)
        else:
            self.is_rtt_mode = False
            self.ui.openButton.setText('打开\n串口')
            self.ui.openButton.setStyleSheet("background-color: #F44336; color: white; font-weight: bold;")
            self.status_label.setText('已断开')
            self.status_label.setStyleSheet("color: red;")
            # 解除保护锁：串口关闭时启用刷新按钮和串口选择
            self.refresh_button.setEnabled(True)
            self.port_combo.setEnabled(True)
    
    def _on_error(self, error_msg):
        """错误处理"""
        if self.is_rtt_mode:
            QMessageBox.critical(self, '错误', f'RTT 错误：{error_msg}')
        else:
            QMessageBox.critical(self, '错误', f'串口错误：{error_msg}')
    
    def _on_data_received(self, data):
        """接收数据处理"""
        self.receive_count += len(data)
        
        # 获取显示模式
        if self.ui.hexRadio.isChecked():
            mode = 'HEX'
        elif self.ui.asciiRadio.isChecked():
            mode = 'ASCII'
        else:
            mode = 'MIXED'
        
        # 格式化显示（带颜色）
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
        html_text = self._format_colored_display(data, mode, timestamp, '←')
        
        # 添加到接收区
        self.ui.receiveTextEdit.append(html_text)
        
        # 自动滚动
        if self.ui.autoScrollCheckBox.isChecked():
            cursor = self.ui.receiveTextEdit.textCursor()
            cursor.movePosition(QTextCursor.MoveOperation.End)
            self.ui.receiveTextEdit.setTextCursor(cursor)
        
        # 记录日志（使用相同的时间戳）
        self.logger.log(data, 'RECEIVE', timestamp, mode)
        
        # 更新状态栏
        self._update_status_bar()
    
    def _format_colored_display(self, data, mode, timestamp, arrow):
        """格式化带颜色的显示文本"""
        # 颜色定义
        timestamp_color = "#00CED1"  # 淡青色
        arrow_color = "#000000"      # 黑色
        data_color = "#000000"       # 黑色
        
        hex_str = self.data_handler.bytes_to_hex(data)
        ascii_str = self.data_handler.bytes_to_ascii(data)
        
        # 将换行符转为 HTML 换行
        ascii_str_html = ascii_str.replace('\n', '<br>')
        
        if mode == 'HEX':
            return (f'<span style="color:{timestamp_color};">[{timestamp}]</span> '
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} HEX:</span> '
                    f'<span style="color:{data_color};">{hex_str}</span>')
        elif mode == 'ASCII':
            return (f'<span style="color:{timestamp_color};">[{timestamp}]</span> '
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} ASCII:</span> '
                    f'<span style="color:{data_color};">{ascii_str_html}</span>')
        else:  # MIXED
            return (f'<span style="color:{timestamp_color};">[{timestamp}]</span><br>'
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} HEX:</span> '
                    f'<span style="color:{data_color};">{hex_str}</span><br>'
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} ASCII:</span> '
                    f'<span style="color:{data_color};">{ascii_str_html}</span>')
    
    def _send_data_func(self, data):
        """统一发送接口，供扩展发送管理器调用
        
        Args:
            data: bytes类型的数据
            
        Returns:
            bool: 发送是否成功
        """
        if self.is_rtt_mode:
            return self.rtt_manager.send_data(data, is_hex=False)
        else:
            return self.serial_manager.send_data(data, is_hex=False)
    
    def _send_data(self):
        """发送数据"""
        if not self.serial_manager.is_connected and not self.rtt_manager.is_connected:
            QMessageBox.warning(self, '警告', '请先打开串口')
            return
        
        data = self.ui.sendTextEdit.toPlainText()
        if not data:
            return
        
        is_hex = self.ui.sendHexRadio.isChecked()
        
        # 验证 HEX 输入
        if is_hex and not self.data_handler.validate_hex_input(data):
            QMessageBox.warning(self, '警告', 'HEX 格式输入错误')
            return
        
        if is_hex:
            hex_str = data.replace(' ', '').replace('\n', '')
            bytes_data = bytes.fromhex(hex_str)
        else:
            bytes_data = data.encode('utf-8')
        
        # 检查是否需要添加回车换行
        if self.ui.appendNewLineCheckBox.isChecked():
            bytes_data += b'\r\n'
        
        # 根据连接类型选择发送方式
        success = False
        if self.is_rtt_mode:
            success = self.rtt_manager.send_data(bytes_data, is_hex=False)
        else:
            success = self.serial_manager.send_data(bytes_data, is_hex=False)
        
        if success:
            self.send_count += len(bytes_data)
            
            # 获取显示模式
            if self.ui.hexRadio.isChecked():
                mode = 'HEX'
            elif self.ui.asciiRadio.isChecked():
                mode = 'ASCII'
            else:
                mode = 'MIXED'
            
            # 在显示区显示发送的数据（带颜色）
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
            html_text = self._format_colored_display(bytes_data, mode, timestamp, '→')
            self.ui.receiveTextEdit.append(html_text)
            
            # 记录日志（使用相同的时间戳）
            self.logger.log(bytes_data, 'SEND', timestamp, mode)
            
            # 更新状态栏
            self._update_status_bar()
    
    def _on_extended_data_sent(self, data):
        """扩展发送数据后显示"""
        self.send_count += len(data)
        
        # 获取显示模式
        if self.ui.hexRadio.isChecked():
            mode = 'HEX'
        elif self.ui.asciiRadio.isChecked():
            mode = 'ASCII'
        else:
            mode = 'MIXED'
        
        # 在显示区显示发送的数据（带颜色）
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
        html_text = self._format_colored_display(data, mode, timestamp, '→')
        self.ui.receiveTextEdit.append(html_text)
        
        # 记录日志（使用相同的时间戳）
        self.logger.log(data, 'SEND', timestamp, mode)
        
        # 更新状态栏
        self._update_status_bar()
    
    def _auto_send(self):
        """自动发送"""
        self._send_data()
    
    def _toggle_auto_send(self, state):
        """切换自动发送"""
        if state:
            interval = self.ui.intervalSpinBox.value()
            self.auto_send_timer.start(interval)
        else:
            self.auto_send_timer.stop()
    
    def _toggle_preset_panel(self, checked):
        """切换扩展发送面板显示（按钮）"""
        self.ui.extendedSendContainer.setVisible(checked)
        self.toggle_preset_button.setChecked(checked)
        self.ui.togglePresetAction.setChecked(checked)
    
    def _toggle_preset_panel_menu(self, checked):
        """切换扩展发送面板显示（菜单）"""
        self.ui.extendedSendContainer.setVisible(checked)
        self.toggle_preset_button.setChecked(checked)
    
    def _clear_receive(self):
        """清空接收区"""
        self.ui.receiveTextEdit.clear()
        self.receive_count = 0
        self._update_status_bar()
    
    def _clear_send(self):
        """清空发送区"""
        self.ui.sendTextEdit.clear()
    
    def _show_about(self):
        """显示关于对话框"""
        about_text = f"""{APP_NAME} V{VERSION}

基于 PySide6 开发的串口调试工具

功能特性：
• 支持 HEX/ASCII/HEX+ASCII 多种显示模式
• 支持数据帧自动拼接
• 支持扩展发送（多条数据批量发送）
• 支持自动发送和回车换行
• 支持 J-Link RTT 数据收发
• 支持程序多开
• 自动记录日志

编译时间：{BUILD_TIME}"""
        QMessageBox.about(self, '关于', about_text)
    
    def _update_status_bar(self):
        """更新状态栏"""
        self.send_count_label.setText(f'发送: {self.send_count} 字节')
        self.receive_count_label.setText(f'接收: {self.receive_count} 字节')
    
    def _load_config(self):
        """加载配置"""
        # 加载串口设置
        serial_settings = self.config_manager.get_serial_settings()
        self.serial_manager.update_settings(serial_settings)
        
        # 加载 RTT 设置
        rtt_settings = self.config_manager.get_rtt_settings()
        self.rtt_manager.update_settings(rtt_settings)
        
        # 加载波特率
        baudrate = self.config_manager.get('baudrate', '115200')
        self.baudrate_combo.setCurrentText(baudrate)
        
        # 加载显示模式
        display_mode = self.config_manager.get('display_mode', 'ASCII')
        if display_mode == 'HEX':
            self.ui.hexRadio.setChecked(True)
        elif display_mode == 'MIXED':
            self.ui.mixedRadio.setChecked(True)
        else:
            self.ui.asciiRadio.setChecked(True)
        
        # 加载发送模式
        send_mode = self.config_manager.get('send_mode', 'ASCII')
        if send_mode == 'HEX':
            self.ui.sendHexRadio.setChecked(True)
        else:
            self.ui.sendAsciiRadio.setChecked(True)
        
        # 加载自动滚动
        auto_scroll = self.config_manager.get('auto_scroll', True)
        self.ui.autoScrollCheckBox.setChecked(auto_scroll)
        
        # 加载自动发送间隔
        interval = self.config_manager.get('auto_send_interval', 1000)
        self.ui.intervalSpinBox.setValue(interval)
        
        # 刷新端口列表并加载保存的端口（阻塞信号防止覆盖配置）
        saved_port = self.config_manager.get('port', '')
        self._refresh_ports(block_signals=True)
        
        # 加载串口
        if saved_port:
            index = self.port_combo.findData(saved_port)
            if index >= 0:
                self.port_combo.setCurrentIndex(index)
    
    def _save_config(self):
        """保存配置"""
        if self.port_combo.currentIndex() >= 0:
            self.config_manager.set('port', self.port_combo.currentData())
        
        self.config_manager.set('baudrate', self.baudrate_combo.currentText())
        
        if self.ui.hexRadio.isChecked():
            self.config_manager.set('display_mode', 'HEX')
        elif self.ui.mixedRadio.isChecked():
            self.config_manager.set('display_mode', 'MIXED')
        else:
            self.config_manager.set('display_mode', 'ASCII')
        
        if self.ui.sendHexRadio.isChecked():
            self.config_manager.set('send_mode', 'HEX')
        else:
            self.config_manager.set('send_mode', 'ASCII')
        
        self.config_manager.set('auto_scroll', self.ui.autoScrollCheckBox.isChecked())
        self.config_manager.set('auto_send_interval', self.ui.intervalSpinBox.value())
        
        self.config_manager.save()
    
    def _save_config_item(self, item_key):
        """保存单个配置项"""
        if item_key == 'port':
            if self.port_combo.currentIndex() >= 0:
                self.config_manager.set('port', self.port_combo.currentData())
        elif item_key == 'baudrate':
            self.config_manager.set('baudrate', self.baudrate_combo.currentText())
        elif item_key == 'display_mode':
            if self.ui.hexRadio.isChecked():
                self.config_manager.set('display_mode', 'HEX')
            elif self.ui.mixedRadio.isChecked():
                self.config_manager.set('display_mode', 'MIXED')
            else:
                self.config_manager.set('display_mode', 'ASCII')
        elif item_key == 'send_mode':
            if self.ui.sendHexRadio.isChecked():
                self.config_manager.set('send_mode', 'HEX')
            else:
                self.config_manager.set('send_mode', 'ASCII')
        elif item_key == 'auto_scroll':
            self.config_manager.set('auto_scroll', self.ui.autoScrollCheckBox.isChecked())
        elif item_key == 'auto_send_interval':
            self.config_manager.set('auto_send_interval', self.ui.intervalSpinBox.value())
        
        self.config_manager.save()
    
    def closeEvent(self, event):
        """关闭事件"""
        self._save_config()
        if self.serial_manager.is_connected:
            self.serial_manager.disconnect()
        if self.rtt_manager.is_connected:
            self.rtt_manager.disconnect()
        self.extended_send_manager.stop_sending()
        event.accept()
