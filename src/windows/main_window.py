import sys
import os
import gc
from PySide6.QtWidgets import QMainWindow, QMessageBox, QVBoxLayout, QFileDialog, QInputDialog
from PySide6.QtCore import QTimer, QThread, Qt
from PySide6.QtGui import QIcon

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from ui.Ui_main_window import Ui_MainWindow
from src.windows.serial_settings_dialog import SerialSettingsDialog
from src.core.config_manager import ConfigManager
from src.core.extended_send_manager import ExtendedSendManager
from src.core.path_utils import get_resource_path
from src.windows.extended_send_widget import ExtendedSendWidget
from src.windows.status_bar_controller import StatusBarController
from src.windows.receive_display_handler import ReceiveDisplayHandler
from src.io.rtt_manager import RttManager
from src.core.ansi_parser import AnsiParser
from src.core.data_handler import DataHandler
from src.core.logger import Logger
from src.version import APP_NAME, VERSION, ICON_PATH
from src.io.serial_manager import SerialManager
from src.io.socket_manager import SocketManager
from src.build_info import BUILD_TIME

MEMORY_RECOVER_INTERVAL_MS = 10000


class MainWindow(QMainWindow):

    def __init__(self, instance_id=1):
        super().__init__()
        self.instance_id = instance_id
        self.ui = Ui_MainWindow()
        self.ui.setupUi(self)

        title = f"{APP_NAME} V{VERSION}"
        if instance_id > 1:
            title += f" [实例{instance_id}]"
        self.setWindowTitle(title)

        icon_path = get_resource_path(ICON_PATH)
        if os.path.exists(icon_path):
            self.setWindowIcon(QIcon(icon_path))

        # 配置
        self.config_manager = ConfigManager(instance_id=instance_id)

        # IO 管理器
        self.serial_manager = SerialManager()
        self.rtt_manager = RttManager()
        self.socket_manager = SocketManager()
        self.data_handler = DataHandler()
        self.ansi_parser = AnsiParser()
        self.logger = Logger(instance_id=instance_id)

        # 扩展发送（注入配置目录以支持多实例隔离）
        self.extended_send_manager = ExtendedSendManager(
            self._send_data_func, config_dir=self.config_manager.config_dir,
        )
        self.extended_send_widget = ExtendedSendWidget(self.extended_send_manager)
        container_layout = QVBoxLayout(self.ui.extendedSendContainer)
        container_layout.setContentsMargins(0, 0, 0, 0)
        container_layout.addWidget(self.extended_send_widget)

        # 接收区显示处理器
        self.send_count = 0
        self.io_mode = 'serial'
        self.display_ansi = False
        self._display_handler = ReceiveDisplayHandler(
            self.ui.receiveTextEdit, self.data_handler, self.ansi_parser,
            self.logger,
            get_display_mode=lambda: self._display_mode,
            get_display_ansi=lambda: self.display_ansi,
            get_auto_scroll=lambda: self.ui.autoScrollCheckBox.isChecked(),
        )

        # 状态栏
        self._status_bar = StatusBarController(self.statusBar())

        # 定时器
        self.auto_send_timer = QTimer()
        self.auto_send_timer.timeout.connect(self._auto_send)
        self._memory_recover_timer = QTimer()
        self._memory_recover_timer.timeout.connect(self._recover_memory_if_needed)
        self._memory_recover_timer.start(MEMORY_RECOVER_INTERVAL_MS)
        self._log_flush_timer = QTimer()
        self._log_flush_timer.timeout.connect(self.logger.flush)
        self._log_flush_timer.start(1000)
        self._save_debounce_timer = QTimer()
        self._save_debounce_timer.setSingleShot(True)
        self._save_debounce_timer.timeout.connect(self.config_manager.save)
        self._preset_panel_last_width = 320
        self._jlink_scan_thread = None

        self._init_ui()
        self._setup_connections()
        self._load_config()
        self._display_handler.append_event(f"软件启动 {APP_NAME} V{VERSION}", 'green')

    # ── 属性 ──

    @property
    def _display_mode(self):
        if self.ui.hexRadio.isChecked():
            return 'HEX'
        elif self.ui.asciiRadio.isChecked():
            return 'ASCII'
        return 'MIXED'

    @property
    def _io(self):
        return {
            'serial': self.serial_manager,
            'rtt': self.rtt_manager,
            'socket': self.socket_manager,
        }[self.io_mode]

    # ── UI 初始化 ──

    def _init_ui(self):
        self.ui.asciiRadio.setChecked(True)
        self.ui.sendAsciiRadio.setChecked(True)
        self.ui.openButton.setStyleSheet("background-color: #F44336; color: white; font-weight: bold;")
        self.ui.sendTextEdit.setFont(self.ui.receiveTextEdit.font())
        self.ui.sendTextEdit.setPlaceholderText("输入要发送的数据...")

        self.ui.mainSplitter.setChildrenCollapsible(False)
        self.ui.topSplitter.setChildrenCollapsible(False)
        self.ui.mainSplitter.setStretchFactor(0, 7)
        self.ui.mainSplitter.setStretchFactor(1, 1)
        self.ui.topSplitter.setStretchFactor(0, 7)
        self.ui.topSplitter.setStretchFactor(1, 0)
        self.ui.mainSplitter.setSizes([590, 92])
        self.ui.topSplitter.setSizes([720, 0])
        self.ui.sendCenterLayout.setStretch(0, 0)
        self.ui.sendCenterLayout.setStretch(1, 1)

        self._display_handler.setup_context_menu(self._toggle_ansi_display)

        for b in ['9600', '19200', '38400', '57600', '115200', '230400', '460800', '921600']:
            self.ui.baudrateCombo.addItem(b)
        self.ui.baudrateCombo.setCurrentText('115200')
        self.ui.intervalSpinBox.setMinimum(10)

    def _setup_connections(self):
        self.ui.refreshButton.clicked.connect(self._refresh_ports)
        self.ui.settingsButton.clicked.connect(self._show_serial_settings)
        self.ui.baudrateCombo.currentTextChanged.connect(self._on_baudrate_changed)
        self.ui.togglePresetButton.clicked.connect(self._toggle_preset_panel)
        self.ui.topSplitter.splitterMoved.connect(self._on_top_splitter_moved)
        self.ui.mainSplitter.splitterMoved.connect(self._on_main_splitter_moved)
        self.ui.openButton.clicked.connect(self._toggle_serial)
        self.ui.sendButton.clicked.connect(self._send_data)
        self.ui.clearReceiveButton.clicked.connect(self._clear_receive)
        self.ui.autoSendCheckBox.stateChanged.connect(self._toggle_auto_send)

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

        for mgr in (self.serial_manager, self.rtt_manager, self.socket_manager):
            mgr.data_received.connect(self._display_handler.on_data_received)
            mgr.connection_changed.connect(self._on_connection_changed)
            mgr.error_occurred.connect(self._on_error)
        self.socket_manager.client_event.connect(self._on_socket_client_event)

        self.extended_send_manager.data_sent.connect(self._on_extended_data_sent)
        self.extended_send_manager.send_started.connect(
            lambda: self._display_handler.append_event(">>> 扩展发送启动", 'green'))
        self.extended_send_manager.send_finished.connect(
            lambda: self._display_handler.append_event("<<< 扩展发送停止", 'orange'))
        self.extended_send_manager.error_occurred.connect(
            lambda msg: self._display_handler.append_event(f"!!! 扩展发送错误：{msg}", 'red'))
        self.ui.actionExit.triggered.connect(self.close)
        self.ui.actionClearReceive.triggered.connect(self._clear_receive)
        self.ui.actionClearSend.triggered.connect(self._clear_send)
        self.ui.actionSettings.triggered.connect(self._show_serial_settings)
        self.ui.togglePresetAction.triggered.connect(self._toggle_preset_panel_menu)
        self.ui.actionAbout.triggered.connect(self._show_about)
        self.ui.actionLogConverter.triggered.connect(self._show_log_converter)

    # ── 端口刷新 ──

    def _refresh_ports(self, block_signals=False, scan_rtt=None):
        """刷新端口列表，按配置决定是否探测 J-Link。"""
        if scan_rtt is None:
            scan_rtt = self.config_manager.get_bool('support_jlink', False)
        current_port = self.ui.portCombo.currentData() if not block_signals else None
        if block_signals:
            self.ui.portCombo.blockSignals(True)

        self.ui.portCombo.clear()
        for port, description in self.serial_manager.get_available_ports():
            full_description = description
            if '(' in description and ')' in description:
                description = description.split('(')[0].strip()
            display_text = f"{port}-{description}"
            self.ui.portCombo.addItem(display_text, port)
            index = self.ui.portCombo.count() - 1
            self.ui.portCombo.setItemData(index, f"{port}-{full_description}", Qt.ToolTipRole)

        if block_signals:
            self.ui.portCombo.blockSignals(False)

        if not scan_rtt:
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
            if current_port is not None:
                index = self.ui.portCombo.findData(current_port)
                if index >= 0:
                    self.ui.portCombo.setCurrentIndex(index)
            return

        from PySide6.QtCore import Signal as QSignal

        class JLinkScanThread(QThread):
            scan_finished = QSignal(list)
            def __init__(self, rtt_mgr):
                super().__init__()
                self.rtt_mgr = rtt_mgr
            def run(self):
                devs = self.rtt_mgr.get_available_devices()
                self.scan_finished.emit(devs)

        self._cleanup_jlink_scan_thread()
        self._jlink_scan_thread = JLinkScanThread(self.rtt_manager)
        self._jlink_scan_thread.scan_finished.connect(self._on_jlink_scan_finished)
        self._jlink_scan_thread.finished.connect(self._on_jlink_scan_thread_finished)
        self._jlink_scan_thread.start()

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

        if current_port is not None:
            index = self.ui.portCombo.findData(current_port)
            if index >= 0:
                self.ui.portCombo.setCurrentIndex(index)

    def _on_jlink_scan_finished(self, jlink_devices):
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

    def _on_jlink_scan_thread_finished(self):
        if self._jlink_scan_thread:
            self._jlink_scan_thread.deleteLater()
            self._jlink_scan_thread = None

    def _cleanup_jlink_scan_thread(self):
        if self._jlink_scan_thread and self._jlink_scan_thread.isRunning():
            self._jlink_scan_thread.wait(1000)
        if self._jlink_scan_thread:
            self._jlink_scan_thread.deleteLater()
            self._jlink_scan_thread = None

    # ── 设置对话框 ──

    def _show_serial_settings(self):
        rtt_settings = self.config_manager.get_rtt_settings()
        dialog = SerialSettingsDialog(
            self.serial_manager.settings, rtt_settings, self.display_ansi,
            self, support_jlink=self.config_manager.get_bool('support_jlink', False),
        )
        dialog.settings_changed.connect(self._on_serial_settings_changed)
        dialog.rtt_settings_changed.connect(self._on_rtt_settings_changed)
        dialog.common_settings_changed.connect(self._on_common_settings_changed)
        dialog.exec()

    def _on_serial_settings_changed(self, settings):
        self.serial_manager.update_settings(settings)
        self.config_manager.set('databits', settings.get('databits', 8))
        self.config_manager.set('stopbits', settings.get('stopbits', 1))
        self.config_manager.set('parity', settings.get('parity', 'None'))
        self.config_manager.set('flowcontrol', settings.get('flowcontrol', 'None'))
        self.config_manager.save()
        if self.serial_manager.is_connected:
            self.serial_manager.reconfigure()

    def _on_rtt_settings_changed(self, settings):
        self.rtt_manager.update_settings(settings)
        self.config_manager.set('rtt_chip', settings.get('chip', 'nRF52840_xxAA'))
        self.config_manager.set('rtt_speed', settings.get('speed', 4000))
        self.config_manager.set('rtt_reset', settings.get('reset', True))
        self.config_manager.set('rtt_start_address', settings.get('start_address', ''))
        self.config_manager.set('rtt_range_size', settings.get('range_size', ''))
        self.config_manager.save()

    def _on_common_settings_changed(self, settings):
        if 'frame_timeout' in settings:
            timeout = settings['frame_timeout']
            self._display_handler.set_batch_window(timeout)
            for mgr in (self.serial_manager, self.rtt_manager, self.socket_manager):
                mgr.update_settings({'frame_timeout': timeout})
            self.config_manager.set('frame_timeout', timeout)
            self.config_manager.set('rtt_frame_timeout', timeout)
        if 'display_ansi' in settings:
            self.display_ansi = settings['display_ansi']
            self.config_manager.set('display_ansi', self.display_ansi)
        if 'support_jlink' in settings:
            support_jlink = bool(settings['support_jlink'])
            self.config_manager.set('support_jlink', support_jlink)
            self._refresh_ports()
        self.config_manager.save()

    # ── 连接控制 ──

    def _on_baudrate_changed(self, text):
        try:
            baudrate = int(text)
            self.serial_manager.settings['baudrate'] = baudrate
            if self.serial_manager.is_connected:
                self.serial_manager.reconfigure()
        except ValueError:
            pass

    def _on_port_changed(self, index):
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
        if self._io.is_connected:
            self._io.close_connection()
        else:
            if self.ui.portCombo.currentIndex() < 0:
                QMessageBox.warning(self, '警告', '请先选择端口')
                return
            port = self.ui.portCombo.currentData()
            prev_mode = self.io_mode

            if port and port.startswith('SOCKET:'):
                host = self.ui.ipCombo.currentText().strip()
                port_val = self.ui.portSpin.value()
                protocol = 'TCP' if 'TCP' in port else 'UDP'
                role = 'Server' if 'Server' in port else 'Client'
                self.io_mode = 'socket'
                if not self.socket_manager.open_connection(host, port_val, protocol, role):
                    self.io_mode = prev_mode
            elif port and port.startswith('JLINK:'):
                sn = port.replace('JLINK:SN=', '')
                rtt_settings = self.config_manager.get_rtt_settings()
                chip = rtt_settings.get('chip', '')
                self.io_mode = 'rtt'
                success = self.rtt_manager.open_connection(
                    serial_no=sn, chip=chip,
                    speed=rtt_settings.get('speed'),
                    reset_flag=rtt_settings.get('reset'),
                    start_address=rtt_settings.get('start_address') or None,
                    range_size=rtt_settings.get('range_size') or None,
                )
                if success:
                    if chip:
                        self.config_manager.add_rtt_chip_history(chip)
                else:
                    self.io_mode = prev_mode
            else:
                self.io_mode = 'serial'
                self.serial_manager.open_connection(port)

    _MODE_BUTTON_TEXT = {
        'serial': '关闭端口', 'rtt': '关闭RTT', 'socket': '关闭Socket',
    }
    _MODE_OPEN_TEXT = '打开端口'
    _ERROR_TITLES = {
        'serial': '串口错误', 'rtt': 'RTT 错误', 'socket': 'Socket 错误',
    }

    def _on_connection_changed(self, connected):
        if connected:
            text = self._MODE_BUTTON_TEXT.get(self.io_mode, self._MODE_OPEN_TEXT)
            self.ui.openButton.setText(text)
            self.ui.openButton.setStyleSheet("background-color: #4CAF50; color: white; font-weight: bold;")
            port_text = self.ui.portCombo.currentText()
            if self.io_mode == 'socket' and self.socket_manager.current_client:
                client = self.socket_manager.current_client
                self._status_bar.set_connected(f'已连接 {client[0]}:{client[1]} ({port_text})')
            else:
                self._status_bar.set_connected(f'已连接 {port_text}')
            self.ui.refreshButton.setEnabled(False)
            self.ui.portCombo.setEnabled(False)
            self._display_handler.append_event(f">>> 已连接 {port_text}", 'green')
        else:
            port_text = self.ui.portCombo.currentText()
            self.ui.openButton.setText(self._MODE_OPEN_TEXT)
            self.ui.openButton.setStyleSheet("background-color: #F44336; color: white; font-weight: bold;")
            self._status_bar.set_disconnected()
            self.ui.refreshButton.setEnabled(True)
            self.ui.portCombo.setEnabled(True)
            self._display_handler.append_event(f"<<< 已断开 {port_text}", 'orange')

    def _on_error(self, error_msg):
        title = self._ERROR_TITLES.get(self.io_mode, '错误')
        self._display_handler.append_event(f"!!! {title}：{error_msg}", 'red')
        QMessageBox.critical(self, '错误', f'{title}：{error_msg}')

    def _on_socket_client_event(self, event_type, addr):
        host, port = addr
        color = 'green' if event_type == 'connected' else 'orange'
        arrow = '>>>' if event_type == 'connected' else '<<<'
        self._display_handler.append_event(f"{arrow} Client {event_type}: {host}:{port}", color)
        if self._io.is_connected and self.io_mode == 'socket' and event_type == 'connected':
            self._status_bar.set_connected(f'已连接 {host}:{port} ({self.ui.portCombo.currentText()})')

    # ── 发送 ──

    def _send_data_func(self, data):
        return self._io.send_data(data, is_hex=False)

    def _send_data(self):
        auto = self.auto_send_timer.isActive()
        if not self._io.is_connected:
            if not auto:
                QMessageBox.warning(self, '警告', '请先打开端口')
            return
        data = self.ui.sendTextEdit.toPlainText()
        if not data:
            return

        is_hex = self.ui.sendHexRadio.isChecked()
        if is_hex and not self.data_handler.validate_hex_input(data):
            if not auto:
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
            self.send_count += len(bytes_data)
            self._display_handler.append_data(bytes_data, '→', 'SEND')
            self._update_status_counts()
        else:
            self._display_handler.append_event("!!! 发送失败", 'red')

    def _on_extended_data_sent(self, data):
        self.send_count += len(data)
        self._display_handler.append_data(data, '→', 'SEND')
        self._update_status_counts()

    def _auto_send(self):
        self._send_data()

    def _toggle_auto_send(self, state):
        if state:
            self.auto_send_timer.start(self.ui.intervalSpinBox.value())
        else:
            self.auto_send_timer.stop()

    # ── 预设面板 ──

    def _toggle_preset_panel(self, checked):
        self._set_preset_panel_visible(checked)
        self.ui.togglePresetButton.setChecked(checked)
        self.ui.togglePresetAction.setChecked(checked)
        self._save_config_item('preset_panel_visible')

    def _toggle_preset_panel_menu(self, checked):
        self._set_preset_panel_visible(checked)
        self.ui.togglePresetButton.setChecked(checked)
        self._save_config_item('preset_panel_visible')

    def _set_preset_panel_visible(self, visible):
        self.ui.extendedSendContainer.setVisible(visible)
        total_width = max(self.ui.topSplitter.width(), sum(self.ui.topSplitter.sizes()), 720)
        if visible:
            panel_width = min(max(self._preset_panel_last_width, 280), max(total_width // 2, 280))
            receive_width = max(total_width - panel_width, 420)
            self.ui.topSplitter.setSizes([receive_width, panel_width])
        else:
            sizes = self.ui.topSplitter.sizes()
            if len(sizes) >= 2 and sizes[1] > 0:
                self._preset_panel_last_width = sizes[1]
            self.ui.topSplitter.setSizes([max(total_width, 1), 0])

    def _on_top_splitter_moved(self, _pos, _index):
        sizes = self.ui.topSplitter.sizes()
        if self.ui.extendedSendContainer.isVisible() and len(sizes) >= 2 and sizes[1] > 0:
            self._preset_panel_last_width = sizes[1]
        self.config_manager.set('top_splitter_sizes', sizes)
        self._save_debounce_timer.start(500)

    def _on_main_splitter_moved(self, _pos, _index):
        self.config_manager.set('main_splitter_sizes', self.ui.mainSplitter.sizes())
        self._save_debounce_timer.start(500)

    # ── 清空 ──

    def _clear_receive(self):
        self._display_handler.clear()
        self.send_count = 0
        self._update_status_counts()

    def _clear_send(self):
        self.ui.sendTextEdit.clear()

    # ── 关于 ──

    def _show_log_converter(self):
        from scripts.log_converter import convert_file

        file_path, _ = QFileDialog.getOpenFileName(
            self, "选择日志文件", self.logger.log_dir, "日志文件 (*.txt);;所有文件 (*)"
        )
        if not file_path:
            return

        fmt, ok = QInputDialog.getItem(
            self, "输出格式", "选择转换格式:", ["HEX", "ASCII", "Both"], 2, False
        )
        if not ok:
            return

        fmt_map = {"HEX": "hex", "ASCII": "ascii", "Both": "both"}
        try:
            results = convert_file(file_path, fmt_map[fmt])
            parts = [f"  {k.upper()}: {v}" for k, v in results.items()]
            QMessageBox.information(
                self, "转换成功", f"文件已生成：\n" + "\n".join(parts)
            )
        except Exception as e:
            QMessageBox.warning(self, "转换失败", f"转换出错：{e}")

    def _show_about(self):
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

    # ── ANSI 切换 ──

    def _toggle_ansi_display(self, checked):
        self.display_ansi = checked
        self._save_config_item('display_ansi')

    # ── 状态栏 ──

    def _update_status_counts(self):
        self._status_bar.update_counts(self.send_count, self._display_handler.receive_count)

    # ── 内存回收 ──

    def _recover_memory_if_needed(self):
        self.logger.flush()
        self.extended_send_manager.flush()
        gc.collect()

    # ── 配置 ──

    def _load_config(self):
        serial_settings = self.config_manager.get_serial_settings()
        rtt_settings = self.config_manager.get_rtt_settings()
        frame_timeout = self.config_manager.get_int(
            'frame_timeout',
            self.config_manager.get_int('rtt_frame_timeout', 50, minimum=1),
            minimum=1,
        )

        self.serial_manager.update_settings(serial_settings)
        self.rtt_manager.update_settings(rtt_settings)
        self.socket_manager.update_settings({'frame_timeout': frame_timeout})
        self._display_handler.set_batch_window(frame_timeout)

        self.ui.baudrateCombo.setCurrentText(str(self.config_manager.get('baudrate', '115200')))

        display_mode = self.config_manager.get('display_mode', 'ASCII')
        if display_mode == 'HEX':
            self.ui.hexRadio.setChecked(True)
        elif display_mode == 'MIXED':
            self.ui.mixedRadio.setChecked(True)
        else:
            self.ui.asciiRadio.setChecked(True)

        send_mode = self.config_manager.get('send_mode', 'ASCII')
        if send_mode == 'HEX':
            self.ui.sendHexRadio.setChecked(True)
        else:
            self.ui.sendAsciiRadio.setChecked(True)

        self.ui.autoScrollCheckBox.setChecked(self.config_manager.get_bool('auto_scroll', True))
        self.ui.intervalSpinBox.setValue(
            self.config_manager.get_int('auto_send_interval', 1000, minimum=10)
        )

        main_sizes = self.config_manager.get_int_list('main_splitter_sizes', [590, 92], expected_len=2, minimum=1)
        self.ui.mainSplitter.setSizes(main_sizes)

        top_sizes = self.config_manager.get_int_list('top_splitter_sizes', [700, 320], expected_len=2, minimum=0)
        self._preset_panel_last_width = top_sizes[1]

        preset_panel_visible = self.config_manager.get_bool('preset_panel_visible', False)
        self._set_preset_panel_visible(preset_panel_visible)
        self.ui.togglePresetButton.setChecked(preset_panel_visible)
        self.ui.togglePresetAction.setChecked(preset_panel_visible)

        self.display_ansi = self.config_manager.get_bool('display_ansi', False)

        saved_port = self.config_manager.get('port', '')
        self._refresh_ports(block_signals=True)
        if saved_port:
            index = self.ui.portCombo.findData(saved_port)
            if index >= 0:
                self.ui.portCombo.setCurrentIndex(index)

    def _save_display_mode(self):
        self.config_manager.set('display_mode', self._display_mode)

    def _save_send_mode(self):
        if self.ui.sendHexRadio.isChecked():
            self.config_manager.set('send_mode', 'HEX')
        else:
            self.config_manager.set('send_mode', 'ASCII')

    def _save_config(self):
        if self.ui.portCombo.currentIndex() >= 0:
            self.config_manager.set('port', self.ui.portCombo.currentData())
        self.config_manager.set('baudrate', self.ui.baudrateCombo.currentText())
        self._save_display_mode()
        self._save_send_mode()
        self.config_manager.set('auto_scroll', self.ui.autoScrollCheckBox.isChecked())
        self.config_manager.set('auto_send_interval', self.ui.intervalSpinBox.value())
        self.config_manager.set('display_ansi', self.display_ansi)
        self.config_manager.set('main_splitter_sizes', self.ui.mainSplitter.sizes())
        self.config_manager.set('top_splitter_sizes', self.ui.topSplitter.sizes())
        self.config_manager.set('preset_panel_visible', self.ui.extendedSendContainer.isVisible())
        self.config_manager.save()

    def _save_config_item(self, item_key):
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
        elif item_key == 'preset_panel_visible':
            self.config_manager.set('preset_panel_visible', self.ui.extendedSendContainer.isVisible())
        self._save_debounce_timer.start(500)

    def closeEvent(self, event):
        self.auto_send_timer.stop()
        self._memory_recover_timer.stop()
        self._log_flush_timer.stop()
        self._save_debounce_timer.stop()
        self._display_handler.stop_timers()
        self.extended_send_manager.stop_sending()
        self.extended_send_manager.stop_timers()
        self._cleanup_jlink_scan_thread()
        self._save_config()
        if self._io.is_connected:
            self._io.close_connection()
        self._display_handler.append_event("=== 软件退出 ===", 'orange')
        self._display_handler.flush()
        self.logger.flush()
        self.extended_send_manager.flush()
        event.accept()
