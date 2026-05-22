import sys
import os
from datetime import datetime
from PySide6.QtWidgets import QMainWindow, QMessageBox, QVBoxLayout
from PySide6.QtCore import QTimer, QThread, Qt
from PySide6.QtGui import QTextCursor, QIcon, QAction

# 添加项目根目录到路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from ui.Ui_main_window import Ui_MainWindow
from src.windows.serial_settings_dialog import SerialSettingsDialog
from src.core.config_manager import ConfigManager
from src.core.extended_send_manager import ExtendedSendManager
from src.windows.extended_send_widget import ExtendedSendWidget
from src.io.rtt_manager import RttManager
from src.core.ansi_parser import AnsiParser, escape_html
from src.core.data_handler import DataHandler
from src.core.logger import Logger
from src.version import APP_NAME, VERSION, ICON_PATH
from src.io.serial_manager import SerialManager
from src.io.socket_manager import SocketManager
from src.build_info import BUILD_TIME


def get_resource_path(relative_path):
    """获取资源文件的绝对路径，支持打包后的路径"""
    if getattr(sys, 'frozen', False):
        # 打包后的路径
        base_path = sys._MEIPASS
    else:
        # 开发环境路径
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base_path, relative_path)


# P5: 显示颜色常量
DISPLAY_TIMESTAMP_COLOR = "#00CED1"
DISPLAY_ARROW_COLOR = "#000000"
DISPLAY_DATA_COLOR = "#000000"
MAX_DISPLAY_LINES = 10000


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
        self.socket_manager = SocketManager()
        self.data_handler = DataHandler()
        self.ansi_parser = AnsiParser()
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
        self.io_mode = 'serial'  # 'serial' | 'rtt' | 'socket'
        self.display_ansi = False  # ANSI 颜色解析开关
        self.auto_send_timer = QTimer()
        self.auto_send_timer.timeout.connect(self._auto_send)
        self._log_flush_timer = QTimer()
        self._log_flush_timer.timeout.connect(self.logger.flush)
        self._log_flush_timer.start(1000)
        self._save_debounce_timer = QTimer()
        self._save_debounce_timer.setSingleShot(True)
        self._save_debounce_timer.timeout.connect(self.config_manager.save)
        
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
        
        # 设置接收区域右键菜单
        self.ui.receiveTextEdit.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.ui.receiveTextEdit.customContextMenuRequested.connect(self._show_receive_context_menu)
        
        # 设置分割器初始比例
        # 上部分（接收+扩展）和下部分（发送）的比例为5:1
        self.ui.mainSplitter.setSizes([500, 100])
        
        # 接收区域和扩展发送区域的宽度比例为1.55:1
        self.ui.topSplitter.setSizes([550, 340])
        
        # 设置发送区域中间部分的两行比例为1:4
        self.ui.sendCenterLayout.setStretch(0, 1)  # 配置项行
        self.ui.sendCenterLayout.setStretch(1, 4)  # 发送文本框行
    
    @property
    def _display_mode(self):
        """获取当前显示模式"""
        if self.ui.hexRadio.isChecked():
            return 'HEX'
        elif self.ui.asciiRadio.isChecked():
            return 'ASCII'
        return 'MIXED'

    @property
    def _io(self):
        """获取当前活动的 IO 管理器（串口/RTT/Socket）"""
        return {
            'serial': self.serial_manager,
            'rtt': self.rtt_manager,
            'socket': self.socket_manager,
        }[self.io_mode]
    
    def _create_status_bar(self):
        """初始化状态栏（控件在 .ui 中定义，此处做初始化和移入状态栏）"""
        # 从 centralwidget 移至状态栏
        self.statusBar().addPermanentWidget(self.ui.statusBarToolbar, 1)
        self.statusBar().setStyleSheet("QStatusBar::item{border:0}")
        self.statusBar().show()
        
        # 波特率下拉选项（编译时无 items，此处添加）
        for b in ['9600', '19200', '38400', '57600', '115200', '230400', '460800', '921600']:
            self.ui.baudrateCombo.addItem(b)
        self.ui.baudrateCombo.setCurrentText('115200')
        
        # 初始断开状态
        self.ui.statusLabel.setStyleSheet("color: red;")
        
        # 限制自动发送最小间隔
        self.ui.intervalSpinBox.setMinimum(10)
    
    def _setup_connections(self):
        """设置信号连接"""
        # 状态栏按钮连接
        self.ui.refreshButton.clicked.connect(self._refresh_ports)
        self.ui.settingsButton.clicked.connect(self._show_serial_settings)
        self.ui.baudrateCombo.currentTextChanged.connect(self._on_baudrate_changed)
        self.ui.togglePresetButton.clicked.connect(self._toggle_preset_panel)
        
        # 发送区域的打开串口按钮
        self.ui.openButton.clicked.connect(self._toggle_serial)
        
        # 数据发送
        self.ui.sendButton.clicked.connect(self._send_data)
        
        # 接收区域控制
        self.ui.clearReceiveButton.clicked.connect(self._clear_receive)
        
        # 自动发送
        self.ui.autoSendCheckBox.stateChanged.connect(self._toggle_auto_send)
        
        # 配置改变时立即保存
        self.ui.portCombo.currentIndexChanged.connect(lambda: self._save_config_item('port'))
        self.ui.portCombo.currentIndexChanged.connect(self._on_port_changed)
        self.ui.baudrateCombo.currentTextChanged.connect(lambda: self._save_config_item('baudrate'))
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
        
        # Socket 管理器信号
        self.socket_manager.data_received.connect(self._on_data_received)
        self.socket_manager.connection_changed.connect(self._on_connection_changed)
        self.socket_manager.error_occurred.connect(self._on_error)
        self.socket_manager.client_event.connect(self._on_socket_client_event)
        
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
        current_port = self.ui.portCombo.currentData() if not block_signals else None
        
        # 临时阻塞信号防止触发保存
        if block_signals:
            self.ui.portCombo.blockSignals(True)
        
        self.ui.portCombo.clear()
        ports = self.serial_manager.get_available_ports()
        for port, description in ports:
            # 移除描述中可能包含的括号中的端口号，避免重复显示
            # 例如: "USB-SERIAL CH340 (COM3)" -> "USB-SERIAL CH340"
            full_description = description
            if '(' in description and ')' in description:
                description = description.split('(')[0].strip()
            display_text = f"{port}-{description}"
            self.ui.portCombo.addItem(display_text, port)
            # 设置 tooltip 显示完整信息
            index = self.ui.portCombo.count() - 1
            self.ui.portCombo.setItemData(index, f"{port}-{full_description}", Qt.ToolTipRole)
        
        # 恢复信号
        if block_signals:
            self.ui.portCombo.blockSignals(False)
        
        # 在后台线程扫描 J-Link 设备，避免阻塞 UI
        from PySide6.QtCore import QThread, Signal as QSignal
        
        class JLinkScanThread(QThread):
            """J-Link 扫描线程"""
            scan_finished = QSignal(list)
            
            def __init__(self, rtt_manager):
                super().__init__()
                self.rtt_manager = rtt_manager
            
            def run(self):
                devices = self.rtt_manager.get_available_devices()
                self.scan_finished.emit(devices)
        
        self._jlink_scan_thread = JLinkScanThread(self.rtt_manager)
        self._jlink_scan_thread.scan_finished.connect(self._on_jlink_scan_finished)
        self._jlink_scan_thread.start()

        # 添加 Socket 模式固定条目（置于末尾）
        socket_modes = [
            ('SOCKET:TCP:Server', 'TCP Server'),
            ('SOCKET:TCP:Client', 'TCP Client'),
            ('SOCKET:UDP:Server', 'UDP Server'),
            ('SOCKET:UDP:Client', 'UDP Client'),
        ]
        for key, display_text in socket_modes:
            if self.ui.portCombo.findData(key) < 0:
                self.ui.portCombo.addItem(display_text, key)
                idx = self.ui.portCombo.count() - 1
                self.ui.portCombo.setItemData(idx, display_text, Qt.ToolTipRole)
    
    def _on_jlink_scan_finished(self, jlink_devices):
        """J-Link 扫描完成回调（插入到 Socket 条目之前）"""
        # 找到第一个 Socket 条目的位置
        insert_pos = self.ui.portCombo.count()
        for i in range(self.ui.portCombo.count()):
            d = self.ui.portCombo.itemData(i)
            if d and str(d).startswith('SOCKET:'):
                insert_pos = i
                break
        for sn, description in jlink_devices:
            jlink_key = f"JLINK:SN={sn}"
            if self.ui.portCombo.findData(jlink_key) < 0:
                display_text = f"{jlink_key} - {description}"
                self.ui.portCombo.insertItem(insert_pos, display_text, jlink_key)
                self.ui.portCombo.setItemData(insert_pos, display_text, Qt.ToolTipRole)
                insert_pos += 1
    
    def _show_serial_settings(self):
        """显示更多设置对话框"""
        rtt_settings = self.config_manager.get_rtt_settings()
        dialog = SerialSettingsDialog(
            self.serial_manager.settings,
            rtt_settings,
            self.display_ansi,
            self
        )
        dialog.settings_changed.connect(self._on_serial_settings_changed)
        dialog.rtt_settings_changed.connect(self._on_rtt_settings_changed)
        dialog.common_settings_changed.connect(self._on_common_settings_changed)
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
    
    def _on_common_settings_changed(self, settings):
        """通用设置改变"""
        if 'frame_timeout' in settings:
            timeout = settings['frame_timeout']
            self.serial_manager.update_settings({'frame_timeout': timeout})
            self.rtt_manager.update_settings({'frame_timeout': timeout})
            self.socket_manager.update_settings({'frame_timeout': timeout})
            self.config_manager.set('rtt_frame_timeout', timeout)
        if 'display_ansi' in settings:
            self.display_ansi = settings['display_ansi']
            self.config_manager.set('display_ansi', self.display_ansi)
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
        port_data = self.ui.portCombo.currentData()
        if not port_data:
            return

        if port_data.startswith('SOCKET:'):
            self.ui.baudrateStack.setCurrentIndex(1)

            is_server = 'Server' in port_data
            if is_server:
                self.ui.ipCombo.clear()
                from src.io.socket_manager import get_local_ips
                self.ui.ipCombo.addItems(get_local_ips())
                self.ui.ipCombo.setEditable(False)
            else:
                self.ui.ipCombo.clear()
                self.ui.ipCombo.setEditable(True)
                self.ui.ipCombo.setPlaceholderText("输入目标 IP")
        elif port_data.startswith('JLINK:'):
            self.ui.baudrateStack.setCurrentIndex(0)
            self.ui.baudrateCombo.setEnabled(False)
        else:
            self.ui.baudrateStack.setCurrentIndex(0)
            self.ui.baudrateCombo.setEnabled(True)
    
    def _toggle_serial(self):
        """切换端口连接状态"""
        if self._io.is_connected:
            self._io.disconnect()
        else:
            if self.ui.portCombo.currentIndex() < 0:
                QMessageBox.warning(self, '警告', '请先选择端口')
                return
            
            port = self.ui.portCombo.currentData()
            
            if port and port.startswith('SOCKET:'):
                host = self.ui.ipCombo.currentText().strip()
                port_val = self.ui.portSpin.value()
                protocol = 'TCP' if 'TCP' in port else 'UDP'
                role = 'Server' if 'Server' in port else 'Client'
                if self.socket_manager.connect(host, port_val, protocol, role):
                    self.io_mode = 'socket'
            elif port and port.startswith('JLINK:'):
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
                    self.io_mode = 'rtt'
                    if chip:
                        self.config_manager.add_rtt_chip_history(chip)
            else:
                self.serial_manager.connect(port)
                self.io_mode = 'serial'
    
    _MODE_BUTTON_TEXT = {
        'serial': '关闭\n端口', 'rtt': '关闭\nRTT', 'socket': '关闭\nSocket',
    }
    _MODE_OPEN_TEXT = '打开\n端口'

    def _on_connection_changed(self, connected):
        """连接状态改变"""
        if connected:
            text = self._MODE_BUTTON_TEXT.get(self.io_mode, self._MODE_OPEN_TEXT)
            self.ui.openButton.setText(text)
            self.ui.openButton.setStyleSheet("background-color: #4CAF50; color: white; font-weight: bold;")

            if self.io_mode == 'socket' and self.socket_manager.current_client:
                client = self.socket_manager.current_client
                self.ui.statusLabel.setText(
                    f'已连接 {client[0]}:{client[1]} ({self.ui.portCombo.currentText()})'
                )
            else:
                self.ui.statusLabel.setText(f'已连接 {self.ui.portCombo.currentText()}')
            self.ui.statusLabel.setStyleSheet("color: green;")
            self.ui.refreshButton.setEnabled(False)
            self.ui.portCombo.setEnabled(False)
        else:
            self.io_mode = 'serial'
            self.ui.openButton.setText(self._MODE_OPEN_TEXT)
            self.ui.openButton.setStyleSheet("background-color: #F44336; color: white; font-weight: bold;")
            self.ui.statusLabel.setText('已断开')
            self.ui.statusLabel.setStyleSheet("color: red;")
            self.ui.refreshButton.setEnabled(True)
            self.ui.portCombo.setEnabled(True)
    
    _ERROR_TITLES = {
        'serial': '串口错误', 'rtt': 'RTT 错误', 'socket': 'Socket 错误',
    }

    def _on_error(self, error_msg):
        """错误处理"""
        title = self._ERROR_TITLES.get(self.io_mode, '错误')
        QMessageBox.critical(self, '错误', f'{title}：{error_msg}')
    
    def _on_socket_client_event(self, event_type, addr):
        """Socket 客户端连接/断开事件"""
        host, port = addr
        ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
        msg = f'[{ts}] \u2190 Client {event_type}: {host}:{port}'
        self.ui.receiveTextEdit.append(
            f'<span style="color:#888;">{escape_html(msg)}</span>'
        )

        if self._io.is_connected and self.io_mode == 'socket' and event_type == 'connected':
            self.ui.statusLabel.setText(
                f'已连接 {host}:{port} ({self.ui.portCombo.currentText()})'
            )

    def _append_data_lines(self, data, arrow, log_type):
        """按行拆分并显示数据"""
        mode = self._display_mode
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
        
        if self.io_mode == 'socket' and self.socket_manager.current_client:
            client = self.socket_manager.current_client
            client_prefix = f'[{client[0]}:{client[1]}] '
            display_arrow = client_prefix + arrow
        else:
            display_arrow = arrow

        if log_type == 'RECEIVE':
            self.receive_count += len(data)
        else:
            self.send_count += len(data)
        
        hex_str = self.data_handler.bytes_to_hex(data)
        ascii_str = self.data_handler.bytes_to_ascii(data)
        self.logger.log(timestamp, log_type, hex_str, ascii_str)
        
        if mode == 'ASCII' and b'\n' in data:
            pieces = []
            for part in data.split(b'\n'):
                line_data = part.rstrip(b'\r')
                if not line_data:
                    continue
                html = self._format_colored_display(line_data, mode, timestamp, display_arrow, self.display_ansi)
                pieces.append(html)
            if pieces:
                self.ui.receiveTextEdit.append('<br>'.join(pieces))
        else:
            html = self._format_colored_display(data, mode, timestamp, display_arrow, self.display_ansi)
            self.ui.receiveTextEdit.append(html)
        
        if self.ui.autoScrollCheckBox.isChecked():
            cursor = self.ui.receiveTextEdit.textCursor()
            cursor.movePosition(QTextCursor.MoveOperation.End)
            self.ui.receiveTextEdit.setTextCursor(cursor)
        
        # 限制显示行数
        doc = self.ui.receiveTextEdit.document()
        if doc.blockCount() > MAX_DISPLAY_LINES:
            cursor = QTextCursor(doc.findBlockByNumber(doc.blockCount() - MAX_DISPLAY_LINES // 2))
            cursor.movePosition(QTextCursor.MoveOperation.Start, QTextCursor.MoveMode.KeepAnchor)
            cursor.removeSelectedText()
        
        self._update_status_bar()
    
    def _on_data_received(self, data):
        """接收数据处理"""
        self._append_data_lines(data, '←', 'RECEIVE')
    
    def _format_colored_display(self, data, mode, timestamp, arrow, display_ansi=False):
        """格式化带颜色的显示文本"""
        hex_str = self.data_handler.bytes_to_hex(data)
        
        if display_ansi and mode != 'HEX':
            ascii_colored = self.ansi_parser.bytes_to_html(data, self.data_handler.bytes_to_ascii)
        else:
            ascii_colored = escape_html(self.data_handler.bytes_to_ascii(data))
        
        ts_tag = f'<span style="color:{DISPLAY_TIMESTAMP_COLOR};">[{timestamp}]</span>'
        arrow_tag = f'<span style="color:{DISPLAY_ARROW_COLOR}; font-weight:bold;">{arrow}</span>'
        data_tag = lambda text: f'<span style="color:{DISPLAY_DATA_COLOR};">{text}</span>'
        
        lines = []
        if mode in ('HEX', 'MIXED'):
            lines.append(f'{arrow_tag} HEX: {data_tag(hex_str)}')
        if mode in ('ASCII', 'MIXED'):
            lines.append(f'{arrow_tag} ASCII: {data_tag(ascii_colored)}')
        
        if mode == 'MIXED':
            return f'{ts_tag}<br>' + '<br>'.join(lines)
        return f'{ts_tag} ' + lines[0]
    
    def _show_receive_context_menu(self, pos):
        """显示接收区域的右键菜单"""
        menu = self.ui.receiveTextEdit.createStandardContextMenu()
        menu.addSeparator()
        ansi_action = QAction("ANSI颜色显示", self)
        ansi_action.setCheckable(True)
        ansi_action.setChecked(self.display_ansi)
        ansi_action.toggled.connect(self._toggle_ansi_display)
        menu.addAction(ansi_action)
        menu.exec_(self.ui.receiveTextEdit.mapToGlobal(pos))
    
    def _toggle_ansi_display(self, checked):
        """切换 ANSI 颜色显示"""
        self.display_ansi = checked
        self._save_config_item('display_ansi')
    
    def _send_data_func(self, data):
        """统一发送接口，供扩展发送管理器调用"""
        return self._io.send_data(data, is_hex=False)
    
    def _send_data(self):
        """发送数据"""
        if not self._io.is_connected:
            QMessageBox.warning(self, '警告', '请先打开端口')
            return
        
        data = self.ui.sendTextEdit.toPlainText()
        if not data:
            return
        
        is_hex = self.ui.sendHexRadio.isChecked()
        
        if is_hex and not self.data_handler.validate_hex_input(data):
            QMessageBox.warning(self, '警告', 'HEX 格式输入错误')
            return
        
        if is_hex:
            hex_str = data.replace(' ', '').replace('\n', '')
            bytes_data = bytes.fromhex(hex_str)
        else:
            bytes_data = data.encode('utf-8')
        
        if self.ui.appendNewLineCheckBox.isChecked():
            bytes_data += b'\r\n'
        
        if self._io.send_data(bytes_data, is_hex=False):
            self._append_data_lines(bytes_data, '→', 'SEND')
    
    def _on_extended_data_sent(self, data):
        """扩展发送数据后显示"""
        self._append_data_lines(data, '→', 'SEND')
    
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
        self.ui.togglePresetButton.setChecked(checked)
        self.ui.togglePresetAction.setChecked(checked)
    
    def _toggle_preset_panel_menu(self, checked):
        """切换扩展发送面板显示（菜单）"""
        self.ui.extendedSendContainer.setVisible(checked)
        self.ui.togglePresetButton.setChecked(checked)
    
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

基于 PySide6 开发的多协议调试工具

功能特性：
• 支持串口（Serial）通信
• 支持 J-Link RTT 数据收发
• 支持 TCP/UDP Socket 通信
• 支持 HEX/ASCII/HEX+ASCII 多种显示模式
• 支持数据帧自动拼接（可调超时时间）
• 支持扩展发送（多条数据批量发送/循环发送）
• 支持自动发送和回车换行
• 支持 ANSI 颜色显示
• 支持程序多开（配置隔离）
• 支持配置持久化（自动保存设置）
• 自动记录日志

编译时间：{BUILD_TIME}"""
        QMessageBox.about(self, '关于', about_text)
    
    def _update_status_bar(self):
        """更新状态栏"""
        self.ui.sendCountLabel.setText(f'发送: {self.send_count} 字节')
        self.ui.receiveCountLabel.setText(f'接收: {self.receive_count} 字节')
    
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
        self.ui.baudrateCombo.setCurrentText(baudrate)
        
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
        
        # 加载 ANSI 颜色显示
        self.display_ansi = self.config_manager.get('display_ansi', False)
        
        # 刷新端口列表并加载保存的端口（阻塞信号防止覆盖配置）
        saved_port = self.config_manager.get('port', '')
        self._refresh_ports(block_signals=True)
        
        # 加载串口
        if saved_port:
            index = self.ui.portCombo.findData(saved_port)
            if index >= 0:
                self.ui.portCombo.setCurrentIndex(index)
    
    def _save_display_mode(self):
        """保存显示模式到配置"""
        self.config_manager.set('display_mode', self._display_mode)
    
    def _save_send_mode(self):
        """保存发送模式到配置"""
        if self.ui.sendHexRadio.isChecked():
            self.config_manager.set('send_mode', 'HEX')
        else:
            self.config_manager.set('send_mode', 'ASCII')
    
    def _save_config(self):
        """保存配置"""
        if self.ui.portCombo.currentIndex() >= 0:
            self.config_manager.set('port', self.ui.portCombo.currentData())
        
        self.config_manager.set('baudrate', self.ui.baudrateCombo.currentText())
        self._save_display_mode()
        self._save_send_mode()
        self.config_manager.set('auto_scroll', self.ui.autoScrollCheckBox.isChecked())
        self.config_manager.set('auto_send_interval', self.ui.intervalSpinBox.value())
        self.config_manager.set('display_ansi', self.display_ansi)
        self.config_manager.save()
    
    def _save_config_item(self, item_key):
        """保存单个配置项（防抖）"""
        if item_key == 'port':
            if self.ui.portCombo.currentIndex() >= 0:
                self.config_manager.set('port', self.ui.portCombo.currentData())
        elif item_key == 'baudrate':
            self.config_manager.set('baudrate', self.ui.baudrateCombo.currentText())
        elif item_key == 'display_mode':
            self._save_display_mode()
        elif item_key == 'send_mode':
            self._save_send_mode()
        elif item_key == 'auto_scroll':
            self.config_manager.set('auto_scroll', self.ui.autoScrollCheckBox.isChecked())
        elif item_key == 'auto_send_interval':
            self.config_manager.set('auto_send_interval', self.ui.intervalSpinBox.value())
        elif item_key == 'display_ansi':
            self.config_manager.set('display_ansi', self.display_ansi)
        
        self._save_debounce_timer.start(500)
    
    def closeEvent(self, event):
        """关闭事件"""
        self.logger.flush()
        self.extended_send_manager.flush()
        self._save_config()
        if self._io.is_connected:
            self._io.disconnect()
        self.extended_send_manager.stop_sending()
        event.accept()
