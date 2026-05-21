import sys
import os
import json
from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QListWidget, QPushButton, QLabel, QFileDialog, QMessageBox
from PySide6.QtCore import Qt, Signal, QPoint

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from src.add_preset_dialog import AddPresetDialog


class PresetWindow(QWidget):
    """扩展发送浮动窗口"""
    
    send_requested = Signal(str)  # 发送请求信号
    
    def __init__(self, parent=None):
        super().__init__(parent)
        self.parent_window = parent
        self.is_following = True  # 是否跟随主窗口
        self.offset = QPoint(0, 0)  # 相对于主窗口的偏移
        self.presets = []  # 预设命令列表
        self.config_file = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "config", "presets.json")
        
        self._init_ui()
        self._load_presets()
        self.setWindowFlags(Qt.WindowType.Window | Qt.WindowType.WindowStaysOnTopHint)
        self.setWindowTitle("扩展发送")
        self.setMinimumSize(300, 400)
    
    def _init_ui(self):
        """初始化界面"""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        
        # 标题
        title_label = QLabel("预设命令列表")
        layout.addWidget(title_label)
        
        # 预设命令列表
        self.preset_list = QListWidget()
        layout.addWidget(self.preset_list)
        
        # 按钮布局
        button_layout = QHBoxLayout()
        
        self.add_button = QPushButton("添加")
        self.add_button.clicked.connect(self._add_preset)
        button_layout.addWidget(self.add_button)
        
        self.edit_button = QPushButton("编辑")
        self.edit_button.clicked.connect(self._edit_preset)
        button_layout.addWidget(self.edit_button)
        
        self.delete_button = QPushButton("删除")
        self.delete_button.clicked.connect(self._delete_preset)
        button_layout.addWidget(self.delete_button)
        
        layout.addLayout(button_layout)
        
        # 导入导出按钮
        io_layout = QHBoxLayout()
        
        self.import_button = QPushButton("导入")
        self.import_button.clicked.connect(self._import_presets)
        io_layout.addWidget(self.import_button)
        
        self.export_button = QPushButton("导出")
        self.export_button.clicked.connect(self._export_presets)
        io_layout.addWidget(self.export_button)
        
        layout.addLayout(io_layout)
        
        # 双击发送
        self.preset_list.itemDoubleClicked.connect(self._on_item_double_clicked)
    
    def _load_presets(self):
        """加载预设命令"""
        try:
            if os.path.exists(self.config_file):
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    self.presets = json.load(f)
                self._update_list()
        except Exception as e:
            print(f"加载预设命令失败: {e}")
    
    def _save_presets(self):
        """保存预设命令"""
        try:
            os.makedirs(os.path.dirname(self.config_file), exist_ok=True)
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(self.presets, f, ensure_ascii=False, indent=2)
        except Exception as e:
            print(f"保存预设命令失败: {e}")
    
    def _update_list(self):
        """更新列表显示"""
        self.preset_list.clear()
        for preset in self.presets:
            name = preset.get('name', preset.get('command', ''))
            self.preset_list.addItem(name)
    
    def _add_preset(self):
        """添加预设命令"""
        dialog = AddPresetDialog(self)
        dialog.preset_added.connect(self._on_preset_added)
        dialog.exec()
    
    def _on_preset_added(self, data):
        """预设命令添加完成"""
        self.presets.append(data)
        self._save_presets()
        self._update_list()
    
    def _edit_preset(self):
        """编辑预设命令"""
        current_row = self.preset_list.currentRow()
        if current_row < 0:
            QMessageBox.warning(self, '警告', '请先选择要编辑的命令')
            return
        
        edit_data = self.presets[current_row]
        dialog = AddPresetDialog(self, edit_data)
        dialog.preset_added.connect(lambda data: self._on_preset_edited(current_row, data))
        dialog.exec()
    
    def _on_preset_edited(self, index, data):
        """预设命令编辑完成"""
        if 0 <= index < len(self.presets):
            self.presets[index] = data
            self._save_presets()
            self._update_list()
    
    def _delete_preset(self):
        """删除预设命令"""
        current_row = self.preset_list.currentRow()
        if current_row < 0:
            QMessageBox.warning(self, '警告', '请先选择要删除的命令')
            return
        
        reply = QMessageBox.question(self, '确认', '确定要删除选中的命令吗？',
                                     QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
        if reply == QMessageBox.StandardButton.Yes:
            del self.presets[current_row]
            self._save_presets()
            self._update_list()
    
    def _import_presets(self):
        """导入预设命令"""
        file_path, _ = QFileDialog.getOpenFileName(
            self, '导入预设命令', '', 'JSON文件 (*.json);;所有文件 (*)'
        )
        if file_path:
            try:
                with open(file_path, 'r', encoding='utf-8') as f:
                    imported = json.load(f)
                if isinstance(imported, list):
                    self.presets.extend(imported)
                    self._save_presets()
                    self._update_list()
                    QMessageBox.information(self, '成功', f'成功导入 {len(imported)} 条命令')
            except Exception as e:
                QMessageBox.critical(self, '错误', f'导入失败: {e}')
    
    def _export_presets(self):
        """导出预设命令"""
        if not self.presets:
            QMessageBox.warning(self, '警告', '没有可导出的命令')
            return
        
        file_path, _ = QFileDialog.getSaveFileName(
            self, '导出预设命令', '', 'JSON文件 (*.json);;所有文件 (*)'
        )
        if file_path:
            try:
                with open(file_path, 'w', encoding='utf-8') as f:
                    json.dump(self.presets, f, ensure_ascii=False, indent=2)
                QMessageBox.information(self, '成功', '导出成功')
            except Exception as e:
                QMessageBox.critical(self, '错误', f'导出失败: {e}')
    
    def _on_item_double_clicked(self, item):
        """双击发送预设命令"""
        current_row = self.preset_list.currentRow()
        if 0 <= current_row < len(self.presets):
            command = self.presets[current_row].get('command', '')
            if command:
                self.send_requested.emit(command)
    
    def update_position(self):
        """更新位置（跟随主窗口）"""
        if self.parent_window and self.is_following:
            parent_pos = self.parent_window.pos()
            self.move(parent_pos.x() + self.parent_window.width(), parent_pos.y())
    
    def moveEvent(self, event):
        """移动事件"""
        super().moveEvent(event)
        if self.parent_window and self.is_following:
            # 计算相对于主窗口的偏移
            parent_pos = self.parent_window.pos()
            self.offset = self.pos() - parent_pos
    
    def set_following(self, following):
        """设置是否跟随主窗口"""
        self.is_following = following
