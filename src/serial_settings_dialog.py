import sys
import os
from PySide6.QtWidgets import QDialog
from PySide6.QtCore import Signal

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from ui.Ui_serial_settings_dialog import Ui_SerialSettingsDialog


class SerialSettingsDialog(QDialog):
    """串口设置对话框"""
    
    settings_changed = Signal(dict)
    
    def __init__(self, current_settings=None, parent=None):
        super().__init__(parent)
        self.ui = Ui_SerialSettingsDialog()
        self.ui.setupUi(self)
        
        self._init_ui()
        
        if current_settings:
            self._load_settings(current_settings)
    
    def _init_ui(self):
        """初始化界面"""
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
        """加载当前设置"""
        if 'databits' in settings:
            self.ui.databitsComboBox.setCurrentText(str(settings['databits']))
        if 'stopbits' in settings:
            self.ui.stopbitsComboBox.setCurrentText(str(settings['stopbits']))
        if 'parity' in settings:
            self.ui.parityComboBox.setCurrentText(settings['parity'])
        if 'flowcontrol' in settings:
            self.ui.flowcontrolComboBox.setCurrentText(settings['flowcontrol'])
    
    def _on_accept(self):
        """确认按钮点击"""
        settings = self.get_settings()
        self.settings_changed.emit(settings)
        self.accept()
    
    def get_settings(self):
        """获取设置"""
        return {
            'databits': int(self.ui.databitsComboBox.currentText()),
            'stopbits': float(self.ui.stopbitsComboBox.currentText()),
            'parity': self.ui.parityComboBox.currentText(),
            'flowcontrol': self.ui.flowcontrolComboBox.currentText()
        }
