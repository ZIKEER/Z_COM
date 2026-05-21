import sys
import os
from datetime import datetime
from PySide6.QtWidgets import QMainWindow, QMessageBox, QVBoxLayout, QLabel, QComboBox, QPushButton, QHBoxLayout, QWidget
from PySide6.QtCore import QTimer, Signal, QObject
from PySide6.QtGui import QTextCursor
import serial
import serial.tools.list_ports

# 添加项目根目录到路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from ui.Ui_main_window import Ui_MainWindow
from src.serial_settings_dialog import SerialSettingsDialog
from src.config_manager import ConfigManager
from src.extended_send_manager import ExtendedSendManager
from src.extended_send_widget import ExtendedSendWidget


class SerialManager(QObject):
    """串口管理类"""
    data_received = Signal(bytes)
    connection_changed = Signal(bool)
    error_occurred = Signal(str)
    
    def __init__(self):
        super().__init__()
        self.serial = serial.Serial()
        self.is_connected = False
        self.read_timer = QTimer()
        self.read_timer.timeout.connect(self._read_data)
        self.read_timer.setInterval(10)
        
        # 数据帧拼接
        self.buffer = bytearray()
        self.frame_timeout = QTimer()
        self.frame_timeout.setSingleShot(True)
        self.frame_timeout.setInterval(50)
        self.frame_timeout.timeout.connect(self._flush_buffer)
        
        # 默认串口设置
        self.settings = {
            'baudrate': 115200,
            'databits': 8,
            'stopbits': 1,
            'parity': 'None',
            'flowcontrol': 'None'
        }
    
    def get_available_ports(self):
        """获取可用串口列表"""
        ports = serial.tools.list_ports.comports()
        return [(p.device, p.description) for p in ports]
    
    def update_settings(self, settings):
        """更新串口设置"""
        self.settings.update(settings)
    
    def connect(self, port):
        """连接串口"""
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
            self.read_timer.start()
            self.connection_changed.emit(True)
            return True
        except Exception as e:
            self.error_occurred.emit(str(e))
            return False
    
    def disconnect(self):
        """断开串口"""
        try:
            self.read_timer.stop()
            self.frame_timeout.stop()
            self._flush_buffer()
            self.serial.close()
            self.is_connected = False
            self.connection_changed.emit(False)
            return True
        except Exception as e:
            self.error_occurred.emit(str(e))
            return False
    
    def _read_data(self):
        """读取串口数据"""
        if self.serial.in_waiting:
            data = self.serial.read(self.serial.in_waiting)
            self.buffer.extend(data)
            self.frame_timeout.start()
    
    def _flush_buffer(self):
        """刷新缓冲区，发送完整帧"""
        if self.buffer:
            data = bytes(self.buffer)
            self.buffer.clear()
            self.data_received.emit(data)
    
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
    
    @staticmethod
    def bytes_to_hex(data):
        """字节数据转 HEX 字符串"""
        return ' '.join(f'{b:02X}' for b in data)
    
    @staticmethod
    def bytes_to_ascii(data):
        """字节数据转 ASCII 字符串"""
        return data.decode('utf-8', errors='replace')
    
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
    
    def __init__(self, log_dir="logs"):
        self.log_dir = log_dir
        os.makedirs(log_dir, exist_ok=True)
        self.current_log_file = None
        self._update_log_file()
    
    def _update_log_file(self):
        """更新日志文件（按日期创建）"""
        date_str = datetime.now().strftime("%Y-%m-%d")
        filename = f"{date_str}.log"
        self.current_log_file = os.path.join(self.log_dir, filename)
    
    def log(self, data, direction, display_mode='ASCII'):
        """记录数据"""
        self._update_log_file()
        
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
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
    
    def __init__(self):
        super().__init__()
        self.ui = Ui_MainWindow()
        self.ui.setupUi(self)
        
        # 初始化配置管理
        self.config_manager = ConfigManager()
        
        # 初始化管理器
        self.serial_manager = SerialManager()
        self.data_handler = DataHandler()
        self.logger = Logger()
        
        # 初始化扩展发送管理器
        self.extended_send_manager = ExtendedSendManager(self.serial_manager)
        
        # 创建扩展发送面板
        self.extended_send_widget = ExtendedSendWidget(self.extended_send_manager)
        
        # 将扩展发送面板添加到容器中
        container_layout = QVBoxLayout(self.ui.extendedSendContainer)
        container_layout.setContentsMargins(0, 0, 0, 0)
        container_layout.addWidget(self.extended_send_widget)
        
        # 状态变量
        self.receive_count = 0
        self.send_count = 0
        self.auto_send_timer = QTimer()
        self.auto_send_timer.timeout.connect(self._auto_send)
        
        # 初始化界面
        self._init_ui()
        self._setup_connections()
        self._refresh_ports()
        self._load_config()
    
    def _init_ui(self):
        """初始化界面"""
        # 创建状态栏
        self._create_status_bar()
        
        # 设置默认显示模式
        self.ui.asciiRadio.setChecked(True)
        
        # 设置默认发送格式
        self.ui.sendAsciiRadio.setChecked(True)
        
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
        self.port_combo.setMinimumWidth(150)
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
        
        # 扩展发送管理器信号
        self.extended_send_manager.data_sent.connect(self._on_extended_data_sent)
        
        # 菜单动作
        self.ui.actionExit.triggered.connect(self.close)
        self.ui.actionClearReceive.triggered.connect(self._clear_receive)
        self.ui.actionClearSend.triggered.connect(self._clear_send)
        self.ui.actionSettings.triggered.connect(self._show_serial_settings)
        self.ui.togglePresetAction.triggered.connect(self._toggle_preset_panel_menu)
        self.ui.actionAbout.triggered.connect(self._show_about)
    
    def _refresh_ports(self):
        """刷新串口列表"""
        self.port_combo.clear()
        ports = self.serial_manager.get_available_ports()
        for port, description in ports:
            self.port_combo.addItem(f"{port} - {description}", port)
    
    def _show_serial_settings(self):
        """显示串口设置对话框"""
        dialog = SerialSettingsDialog(self.serial_manager.settings, self)
        dialog.settings_changed.connect(self._on_serial_settings_changed)
        dialog.exec()
    
    def _on_serial_settings_changed(self, settings):
        """串口设置改变"""
        self.serial_manager.update_settings(settings)
    
    def _on_baudrate_changed(self, text):
        """波特率改变"""
        try:
            baudrate = int(text)
            self.serial_manager.settings['baudrate'] = baudrate
        except ValueError:
            pass
    
    def _toggle_serial(self):
        """切换串口连接状态"""
        if self.serial_manager.is_connected:
            self.serial_manager.disconnect()
        else:
            if self.port_combo.currentIndex() < 0:
                QMessageBox.warning(self, '警告', '请先选择串口')
                return
            
            port = self.port_combo.currentData()
            self.serial_manager.connect(port)
    
    def _on_connection_changed(self, connected):
        """串口连接状态改变"""
        if connected:
            self.ui.openButton.setText('关闭串口')
            self.status_label.setText(f'已连接 {self.port_combo.currentText()}')
            self.status_label.setStyleSheet("color: green;")
        else:
            self.ui.openButton.setText('打开串口')
            self.status_label.setText('已断开')
            self.status_label.setStyleSheet("color: red;")
    
    def _on_error(self, error_msg):
        """错误处理"""
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
        
        # 记录日志
        self.logger.log(data, 'RECEIVE', mode)
        
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
        
        if mode == 'HEX':
            return (f'<span style="color:{timestamp_color};">[{timestamp}]</span> '
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} HEX:</span> '
                    f'<span style="color:{data_color};">{hex_str}</span>')
        elif mode == 'ASCII':
            return (f'<span style="color:{timestamp_color};">[{timestamp}]</span> '
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} ASCII:</span> '
                    f'<span style="color:{data_color};">{ascii_str}</span>')
        else:  # MIXED
            return (f'<span style="color:{timestamp_color};">[{timestamp}]</span><br>'
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} HEX:</span> '
                    f'<span style="color:{data_color};">{hex_str}</span><br>'
                    f'<span style="color:{arrow_color}; font-weight:bold;">{arrow} ASCII:</span> '
                    f'<span style="color:{data_color};">{ascii_str}</span>')
    
    def _send_data(self):
        """发送数据"""
        if not self.serial_manager.is_connected:
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
        
        if self.serial_manager.send_data(bytes_data, is_hex=False):
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
            
            # 记录日志
            self.logger.log(bytes_data, 'SEND')
            
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
        
        # 记录日志
        self.logger.log(data, 'SEND')
        
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
        QMessageBox.about(self, '关于', '串口助手 v1.0\n\n基于 PySide6 开发的串口调试工具')
    
    def _update_status_bar(self):
        """更新状态栏"""
        self.send_count_label.setText(f'发送: {self.send_count} 字节')
        self.receive_count_label.setText(f'接收: {self.receive_count} 字节')
    
    def _load_config(self):
        """加载配置"""
        # 加载串口设置
        serial_settings = self.config_manager.get_serial_settings()
        self.serial_manager.update_settings(serial_settings)
        
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
        
        # 加载串口
        port = self.config_manager.get('port', '')
        if port:
            index = self.port_combo.findData(port)
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
        self.extended_send_manager.stop_sending()
        event.accept()
