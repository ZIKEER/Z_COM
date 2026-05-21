import sys
import os
from PySide6.QtWidgets import QDialog
from PySide6.QtCore import Signal

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from ui.Ui_add_preset_dialog import Ui_AddPresetDialog


class AddPresetDialog(QDialog):
    """添加预设命令对话框"""
    
    preset_added = Signal(dict)  # 添加完成信号
    
    def __init__(self, parent=None, edit_data=None):
        super().__init__(parent)
        self.ui = Ui_AddPresetDialog()
        self.ui.setupUi(self)
        
        self._setup_connections()
        
        # 编辑模式
        if edit_data:
            self.setWindowTitle("编辑预设命令")
            self._load_data(edit_data)
    
    def _setup_connections(self):
        """设置信号连接"""
        self.ui.buttonBox.accepted.connect(self._on_accept)
        self.ui.buttonBox.rejected.connect(self.reject)
    
    def _load_data(self, data):
        """加载编辑数据"""
        if 'name' in data:
            self.ui.nameEdit.setText(data['name'])
        if 'command' in data:
            self.ui.commandEdit.setText(data['command'])
        if 'is_hex' in data and data['is_hex']:
            self.ui.hexRadio.setChecked(True)
    
    def _on_accept(self):
        """确认按钮点击"""
        command = self.ui.commandEdit.text().strip()
        if not command:
            return
        
        name = self.ui.nameEdit.text().strip()
        if not name:
            name = command[:20]  # 默认名称
        
        data = {
            'name': name,
            'command': command,
            'is_hex': self.ui.hexRadio.isChecked()
        }
        
        self.preset_added.emit(data)
        self.accept()
    
    def get_data(self):
        """获取数据"""
        return {
            'name': self.ui.nameEdit.text().strip(),
            'command': self.ui.commandEdit.text().strip(),
            'is_hex': self.ui.hexRadio.isChecked()
        }
