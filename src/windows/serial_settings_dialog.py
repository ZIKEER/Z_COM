import sys
import os
from PySide6.QtWidgets import QDialog
from PySide6.QtCore import Signal

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from ui.Ui_serial_settings_dialog import Ui_SerialSettingsDialog


class SerialSettingsDialog(QDialog):
    """更多设置对话框（串口/RTT/通用设置）"""
    
    settings_changed = Signal(dict)
    rtt_settings_changed = Signal(dict)
    common_settings_changed = Signal(dict)
    
    def __init__(self, current_settings=None, rtt_settings=None, display_ansi=False, parent=None):
        super().__init__(parent)
        self.ui = Ui_SerialSettingsDialog()
        self.ui.setupUi(self)
        
        self._init_ui()
        
        if current_settings:
            self._load_settings(current_settings)
        if rtt_settings:
            self._load_rtt_settings(rtt_settings)
        
        self.ui.ansiCheckBox.setChecked(display_ansi)
    
    def _init_ui(self):
        """初始化界面"""
        # 串口 Grid 列比例：标签自然宽度，ComboBox 拉伸
        self.ui.gridLayout.setColumnStretch(0, 0)
        self.ui.gridLayout.setColumnStretch(1, 1)
        self.ui.gridLayout.setColumnStretch(2, 0)
        self.ui.gridLayout.setColumnStretch(3, 1)
        # RTT Grid 列比例
        self.ui.rttGridLayout.setColumnStretch(0, 0)
        self.ui.rttGridLayout.setColumnStretch(1, 1)
        
        # 数据位
        databits = ['5', '6', '7', '8']
        self.ui.databitsComboBox.addItems(databits)
        self.ui.databitsComboBox.setCurrentText('8')
        
        # 停止位
        stopbits = ['1', '1.5', '2']
        self.ui.stopbitsComboBox.addItems(stopbits)
        self.ui.stopbitsComboBox.setCurrentText('1')
        
        # 校验位
        parity = ['None', 'Even', 'Odd', 'Mark', 'Space']
        self.ui.parityComboBox.addItems(parity)
        self.ui.parityComboBox.setCurrentText('None')
        
        # 流控制
        flowcontrol = ['None', 'RTS/CTS', 'DTR/DSR', 'XON/XOFF']
        self.ui.flowcontrolComboBox.addItems(flowcontrol)
        self.ui.flowcontrolComboBox.setCurrentText('None')
        
        # 连接信号
        self.ui.buttonBox.accepted.connect(self._on_accept)
        self.ui.buttonBox.rejected.connect(self.reject)
    
    def _load_settings(self, settings):
        """加载串口设置"""
        if 'databits' in settings:
            self.ui.databitsComboBox.setCurrentText(str(settings['databits']))
        if 'stopbits' in settings:
            self.ui.stopbitsComboBox.setCurrentText(str(settings['stopbits']))
        if 'parity' in settings:
            self.ui.parityComboBox.setCurrentText(settings['parity'])
        if 'flowcontrol' in settings:
            self.ui.flowcontrolComboBox.setCurrentText(settings['flowcontrol'])
    
    def _load_rtt_settings(self, settings):
        """加载 RTT 设置"""
        # 加载芯片历史记录到下拉框
        chip_history = settings.get('chip_history', [])
        if chip_history:
            self.ui.rttChipComboBox.addItems(chip_history)
        
        # 设置当前芯片
        if 'chip' in settings and settings['chip']:
            chip = settings['chip']
            index = self.ui.rttChipComboBox.findText(chip)
            if index < 0:
                # 如果当前芯片不在历史记录中，添加到最前面
                self.ui.rttChipComboBox.insertItem(0, chip)
                index = 0
            self.ui.rttChipComboBox.setCurrentIndex(index)
        
        if 'speed' in settings:
            self.ui.rttSpeedSpinBox.setValue(int(settings['speed']))
        if 'reset' in settings:
            self.ui.rttResetCheckBox.setChecked(settings['reset'])
        if 'start_address' in settings:
            self.ui.rttStartAddressLineEdit.setText(settings['start_address'])
        if 'range_size' in settings:
            self.ui.rttRangeSizeLineEdit.setText(settings['range_size'])
        if 'frame_timeout' in settings:
            self.ui.frameTimeoutSpinBox.setValue(settings['frame_timeout'])
    
    def _on_accept(self):
        """确认按钮点击"""
        settings = self.get_settings()
        rtt_settings = self.get_rtt_settings()
        common_settings = self.get_common_settings()
        self.settings_changed.emit(settings)
        self.rtt_settings_changed.emit(rtt_settings)
        self.common_settings_changed.emit(common_settings)
        self.accept()
    
    def get_settings(self):
        """获取串口设置"""
        return {
            'databits': int(self.ui.databitsComboBox.currentText()),
            'stopbits': float(self.ui.stopbitsComboBox.currentText()),
            'parity': self.ui.parityComboBox.currentText(),
            'flowcontrol': self.ui.flowcontrolComboBox.currentText(),
        }
    
    def get_rtt_settings(self):
        """获取 RTT 设置"""
        return {
            'chip': self.ui.rttChipComboBox.currentText(),
            'speed': self.ui.rttSpeedSpinBox.value(),
            'reset': self.ui.rttResetCheckBox.isChecked(),
            'start_address': self.ui.rttStartAddressLineEdit.text().strip(),
            'range_size': self.ui.rttRangeSizeLineEdit.text().strip(),
        }
    
    def get_common_settings(self):
        """获取通用设置"""
        return {
            'frame_timeout': self.ui.frameTimeoutSpinBox.value(),
            'display_ansi': self.ui.ansiCheckBox.isChecked(),
        }
