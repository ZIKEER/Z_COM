import os
from PySide6.QtWidgets import (QWidget, QTableWidgetItem, QCheckBox, QPushButton, QSpinBox, 
                                QFileDialog, QMessageBox, QHeaderView, QMenu,
                                QHBoxLayout, QLabel, QLineEdit)
from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QAction, QFontMetrics, QCursor, QColor

import sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from ui.Ui_extended_send_widget import Ui_ExtendedSendWidget


class SendItemWidget(QWidget):
    """发送项自定义控件 - 包含数据输入框和发送按钮"""
    
    send_clicked = Signal(int)  # 发送信号
    data_changed = Signal(int, str)  # 数据改变信号
    comment_changed = Signal(int, str)  # 注释改变信号
    
    def __init__(self, item_id, data="", comment="", parent=None):
        super().__init__(parent)
        self.item_id = item_id
        self._data = data
        self._comment = comment
        
        layout = QHBoxLayout(self)
        layout.setContentsMargins(4, 2, 4, 2)
        layout.setSpacing(4)
        
        # 数据输入框
        self.data_edit = QLineEdit()
        self.data_edit.setPlaceholderText("输入数据内容...")
        self.data_edit.setText(data)
        self.data_edit.textChanged.connect(self._on_data_changed)
        layout.addWidget(self.data_edit, 1)
        
        # 发送按钮（显示注释）
        self.send_btn = QPushButton()
        self.send_btn.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.send_btn.customContextMenuRequested.connect(self._show_context_menu)
        self.send_btn.clicked.connect(lambda: self.send_clicked.emit(self.item_id))
        self.send_btn.setMinimumWidth(50)
        layout.addWidget(self.send_btn)
        
        self._update_button_text()
    
    def _update_button_text(self):
        """更新按钮文本"""
        if self._comment:
            metrics = QFontMetrics(self.send_btn.font())
            text_width = metrics.horizontalAdvance(self._comment) + 16
            self.send_btn.setMinimumWidth(min(max(text_width, 50), 200))
            self.send_btn.setText(self._comment)
        else:
            self.send_btn.setMinimumWidth(50)
            self.send_btn.setText("发送")
    
    def set_data(self, data):
        """设置数据"""
        self._data = data
        self.data_edit.setText(data)
    
    def set_comment(self, comment):
        """设置注释"""
        self._comment = comment
        self._update_button_text()
    
    def _on_data_changed(self, text):
        """数据内容改变"""
        self._data = text
        self.data_changed.emit(self.item_id, text)
    
    def _show_context_menu(self, pos):
        """显示右键菜单"""
        menu = QMenu(self)
        edit_action = QAction("编辑注释", self)
        edit_action.triggered.connect(self._edit_comment)
        menu.addAction(edit_action)
        menu.exec_(self.send_btn.mapToGlobal(pos))
    
    def _edit_comment(self):
        """编辑注释"""
        from PySide6.QtWidgets import QInputDialog
        text, ok = QInputDialog.getText(self, "编辑注释", "注释:", text=self._comment)
        if ok:
            self._comment = text
            self._update_button_text()
            self.comment_changed.emit(self.item_id, text)


class ExtendedSendWidget(QWidget):
    """扩展发送面板组件"""
    
    def __init__(self, extended_send_manager, parent=None):
        super().__init__(parent)
        self.manager = extended_send_manager
        self.ui = Ui_ExtendedSendWidget()
        self.ui.setupUi(self)
        
        # 操作模式：normal, delete, move
        self.operation_mode = "normal"
        
        # 设置表格属性
        self._setup_table()
        
        # 设置右键菜单
        self.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.customContextMenuRequested.connect(self._show_main_context_menu)
        
        self._setup_connections()
        self._refresh_table()
        
        # 监听数据变化
        self.manager.items_changed.connect(self._refresh_table)
    
    def _setup_table(self):
        """设置表格属性"""
        header = self.ui.dataTable.horizontalHeader()
        header.setSectionResizeMode(0, QHeaderView.ResizeMode.Fixed)  # HEX
        header.setSectionResizeMode(1, QHeaderView.ResizeMode.Stretch)  # 数据内容/注释
        header.setSectionResizeMode(2, QHeaderView.ResizeMode.Fixed)  # 序号
        header.setSectionResizeMode(3, QHeaderView.ResizeMode.Fixed)  # 延时
        
        self.ui.dataTable.setColumnWidth(0, 40)
        self.ui.dataTable.setColumnWidth(2, 50)
        self.ui.dataTable.setColumnWidth(3, 75)
        
        # 隐藏垂直表头
        self.ui.dataTable.verticalHeader().setVisible(False)
        
        # 设置行高
        self.ui.dataTable.verticalHeader().setDefaultSectionSize(36)
        
        # 设置右键菜单
        self.ui.dataTable.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.ui.dataTable.customContextMenuRequested.connect(self._show_table_context_menu)
        
        # 设置问号样式
        self.ui.helpLabel.setStyleSheet("""
            QLabel {
                color: #666;
                font-weight: bold;
                background-color: #E0E0E0;
                border-radius: 8px;
                padding: 2px;
                min-width: 16px;
                max-width: 16px;
                min-height: 16px;
                max-height: 16px;
            }
            QLabel:hover {
                background-color: #C0C0C0;
            }
        """)
        self.ui.helpLabel.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.ui.helpLabel.setCursor(QCursor(Qt.CursorShape.WhatsThisCursor))
        self.ui.helpLabel.mousePressEvent = self._show_help_message
    
    def _show_help_message(self, event):
        """显示帮助信息"""
        help_text = """扩展发送说明：

1. HEX列：勾选表示HEX格式，不勾选表示ASCII格式
2. 序号：0=不参与发送，1,2,3...=发送顺序
3. 延时：发送后等待时间（毫秒）
4. 左键点击发送按钮：发送单条数据
5. 右键点击发送按钮：编辑注释
6. 右键点击面板：弹出更多功能菜单"""
        QMessageBox.information(self, "扩展发送帮助", help_text)
    
    def _setup_connections(self):
        """设置信号连接"""
        self.ui.addButton.clicked.connect(self._add_item)
        self.ui.deleteButton.clicked.connect(self._on_delete_clicked)
        self.ui.moveUpButton.clicked.connect(self._on_move_up_clicked)
        self.ui.moveDownButton.clicked.connect(self._on_move_down_clicked)
        self.ui.loopSendButton.clicked.connect(self._on_loop_send_clicked)
        
        # 发送进度信号
        self.manager.send_started.connect(self._on_send_started)
        self.manager.send_finished.connect(self._on_send_finished)
        self.manager.send_progress.connect(self._on_send_progress)
        self.manager.error_occurred.connect(self._on_error)
    
    def _refresh_table(self):
        """刷新表格显示"""
        self.ui.dataTable.setRowCount(0)
        
        # 检查是否需要显示选择列
        show_select_column = self.operation_mode in ["delete", "move"]
        
        if show_select_column:
            # 添加选择列
            if self.ui.dataTable.columnCount() == 4:
                self.ui.dataTable.insertColumn(0)
                self.ui.dataTable.setHorizontalHeaderLabels(['选择', 'HEX', '数据内容/注释', '序号', '延时'])
                self.ui.dataTable.setColumnWidth(0, 35)
        
        for item in self.manager.items:
            row = self.ui.dataTable.rowCount()
            self.ui.dataTable.insertRow(row)
            
            item_id = item['id']
            
            col_offset = 1 if show_select_column else 0
            
            # 选择列（删除/移动模式）
            if show_select_column:
                select_check = QCheckBox()
                select_check.setChecked(False)
                # 居中显示
                check_widget = QWidget()
                check_layout = QHBoxLayout(check_widget)
                check_layout.addWidget(select_check)
                check_layout.setAlignment(Qt.AlignmentFlag.AlignCenter)
                check_layout.setContentsMargins(0, 0, 0, 0)
                self.ui.dataTable.setCellWidget(row, 0, check_widget)
                select_check.setProperty("item_id", item_id)
            
            # HEX选择（复选框）
            hex_check = QCheckBox()
            hex_check.setChecked(item.get('is_hex', True))
            hex_check.stateChanged.connect(lambda state, iid=item_id: self._on_hex_changed(iid, state))
            check_widget = QWidget()
            check_layout = QHBoxLayout(check_widget)
            check_layout.addWidget(hex_check)
            check_layout.setAlignment(Qt.AlignmentFlag.AlignCenter)
            check_layout.setContentsMargins(0, 0, 0, 0)
            self.ui.dataTable.setCellWidget(row, 0 + col_offset, check_widget)
            
            # 数据内容/注释
            data_text = item.get('data', '')
            comment_text = item.get('comment', '')
            
            item_widget = SendItemWidget(item_id, data_text, comment_text)
            item_widget.send_clicked.connect(self._send_single)
            item_widget.data_changed.connect(self._on_data_changed)
            item_widget.comment_changed.connect(self._on_comment_changed)
            self.ui.dataTable.setCellWidget(row, 1 + col_offset, item_widget)
            
            # 保存item_id
            hidden_item = QTableWidgetItem()
            hidden_item.setData(Qt.ItemDataRole.UserRole, item_id)
            self.ui.dataTable.setItem(row, 1 + col_offset, hidden_item)
            
            # 序号
            order_edit = QLineEdit()
            order_edit.setText(str(item.get('sort_order', 0)))
            order_edit.setAlignment(Qt.AlignmentFlag.AlignCenter)
            order_edit.setMaximumWidth(40)
            order_edit.setMaxLength(3)
            order_edit.textChanged.connect(lambda text, iid=item_id: self._on_order_changed(iid, text))
            self.ui.dataTable.setCellWidget(row, 2 + col_offset, order_edit)
            
            # 延时
            delay_widget = QWidget()
            delay_layout = QHBoxLayout(delay_widget)
            delay_layout.setContentsMargins(2, 0, 2, 0)
            delay_layout.setSpacing(2)
            
            delay_edit = QLineEdit()
            delay_edit.setText(str(item.get('delay', 1000)))
            delay_edit.setAlignment(Qt.AlignmentFlag.AlignCenter)
            delay_edit.setMaximumWidth(50)
            delay_edit.textChanged.connect(lambda text, iid=item_id: self._on_delay_changed(iid, text))
            delay_layout.addWidget(delay_edit)
            
            delay_unit = QLabel("ms")
            delay_layout.addWidget(delay_unit)
            
            self.ui.dataTable.setCellWidget(row, 3 + col_offset, delay_widget)
        
        # 更新按钮文本
        self._update_button_texts()
    
    def _update_button_texts(self):
        """更新按钮文本"""
        if self.operation_mode == "delete":
            self.ui.deleteButton.setText("确认删除")
            self.ui.deleteButton.setStyleSheet("background-color: #FF6B6B;")
        else:
            self.ui.deleteButton.setText("删除")
            self.ui.deleteButton.setStyleSheet("")
        
        if self.operation_mode == "move":
            self.ui.moveUpButton.setText("确认上移")
            self.ui.moveDownButton.setText("确认下移")
            self.ui.moveUpButton.setStyleSheet("background-color: #4ECDC4;")
            self.ui.moveDownButton.setStyleSheet("background-color: #4ECDC4;")
        else:
            self.ui.moveUpButton.setText("上移")
            self.ui.moveDownButton.setText("下移")
            self.ui.moveUpButton.setStyleSheet("")
            self.ui.moveDownButton.setStyleSheet("")
    
    def _get_selected_items(self):
        """获取选中的item_id列表"""
        selected = []
        if self.operation_mode in ["delete", "move"]:
            for row in range(self.ui.dataTable.rowCount()):
                widget = self.ui.dataTable.cellWidget(row, 0)
                if widget:
                    checkbox = widget.findChild(QCheckBox)
                    if checkbox and checkbox.isChecked():
                        selected.append(checkbox.property("item_id"))
        return selected
    
    def _exit_operation_mode(self):
        """退出操作模式"""
        self.operation_mode = "normal"
        # 移除选择列
        if self.ui.dataTable.columnCount() > 4:
            self.ui.dataTable.removeColumn(0)
        self._refresh_table()
    
    def _add_item(self):
        """添加数据条目"""
        self.manager.add_item("", is_hex=False, comment="", delay=1000, enabled=True)
        new_row = self.ui.dataTable.rowCount() - 1
        self.ui.dataTable.selectRow(new_row)
    
    def _on_delete_clicked(self):
        """删除按钮点击"""
        if self.operation_mode == "normal":
            if self.manager.items:
                self.operation_mode = "delete"
                self._refresh_table()
        elif self.operation_mode == "delete":
            selected = self._get_selected_items()
            if selected:
                reply = QMessageBox.question(self, "确认删除", f"确定要删除 {len(selected)} 条数据吗？",
                                             QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
                if reply == QMessageBox.StandardButton.Yes:
                    for item_id in selected:
                        self.manager.remove_item(item_id)
            self._exit_operation_mode()
        else:
            # 其他模式下退出
            self._exit_operation_mode()
    
    def _on_move_up_clicked(self):
        """上移按钮点击"""
        if self.operation_mode == "normal":
            if self.manager.items:
                self.operation_mode = "move"
                self._refresh_table()
        elif self.operation_mode == "move":
            selected = self._get_selected_items()
            if selected:
                for item_id in selected:
                    self.manager.move_item(item_id, -1)
            self._exit_operation_mode()
        else:
            self._exit_operation_mode()
            self.operation_mode = "move"
            self._refresh_table()
    
    def _on_move_down_clicked(self):
        """下移按钮点击"""
        if self.operation_mode == "normal":
            if self.manager.items:
                self.operation_mode = "move"
                self._refresh_table()
        elif self.operation_mode == "move":
            selected = self._get_selected_items()
            if selected:
                for item_id in selected:
                    self.manager.move_item(item_id, 1)
            self._exit_operation_mode()
        else:
            self._exit_operation_mode()
            self.operation_mode = "move"
            self._refresh_table()
    
    def _on_loop_send_clicked(self, checked):
        """循环发送按钮点击"""
        if checked:
            # 开始循环发送
            duplicates = self._check_order_duplicates()
            if duplicates:
                QMessageBox.warning(self, "序号重复", 
                                    f"序号 {', '.join(map(str, sorted(duplicates)))} 有重复，请修改")
                self.ui.loopSendButton.setChecked(False)
                return
            
            self.manager.send_multiple(loop=True)
            self.ui.loopSendButton.setText("停止发送")
        else:
            # 停止发送
            self.manager.stop_sending()
            self.ui.loopSendButton.setText("循环发送")
    
    def _check_order_duplicates(self):
        """检查序号是否有重复"""
        orders = []
        for item in self.manager.items:
            order = item.get('sort_order', 0)
            if order > 0:
                orders.append(order)
        
        seen = set()
        duplicates = set()
        for order in orders:
            if order in seen:
                duplicates.add(order)
            seen.add(order)
        
        return duplicates
    
    def _send_single(self, item_id):
        """发送单条数据"""
        self.manager.send_single(item_id)
    
    def _show_main_context_menu(self, pos):
        """显示主界面右键菜单"""
        menu = QMenu(self)
        
        send_action = QAction("发送选中", self)
        send_action.triggered.connect(self._send_selected)
        menu.addAction(send_action)
        
        clear_action = QAction("清空所有", self)
        clear_action.triggered.connect(self._clear_items)
        menu.addAction(clear_action)
        
        menu.addSeparator()
        
        import_action = QAction("导入配置", self)
        import_action.triggered.connect(self._import_config)
        menu.addAction(import_action)
        
        export_action = QAction("导出配置", self)
        export_action.triggered.connect(self._export_config)
        menu.addAction(export_action)
        
        menu.exec_(self.mapToGlobal(pos))
    
    def _show_table_context_menu(self, pos):
        """显示表格右键菜单"""
        self._show_main_context_menu(pos)
    
    def _send_selected(self):
        """发送选中数据"""
        duplicates = self._check_order_duplicates()
        if duplicates:
            QMessageBox.warning(self, "序号重复", 
                                f"序号 {', '.join(map(str, sorted(duplicates)))} 有重复，请修改")
            return
        
        self.manager.send_multiple(loop=False)
    
    def _clear_items(self):
        """清空所有数据"""
        if self.manager.items:
            reply = QMessageBox.question(self, "确认", "确定要清空所有数据吗？",
                                         QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
            if reply == QMessageBox.StandardButton.Yes:
                self.manager.clear_items()
    
    def _import_config(self):
        """导入配置"""
        file_path, _ = QFileDialog.getOpenFileName(
            self, "导入配置", "", "JSON文件 (*.json);;所有文件 (*)"
        )
        if file_path:
            if self.manager.import_from_file(file_path):
                QMessageBox.information(self, "成功", "导入成功")
    
    def _export_config(self):
        """导出配置"""
        file_path, _ = QFileDialog.getSaveFileName(
            self, "导出配置", "", "JSON文件 (*.json);;所有文件 (*)"
        )
        if file_path:
            if self.manager.export_to_file(file_path):
                QMessageBox.information(self, "成功", "导出成功")
    
    def _on_hex_changed(self, item_id, state):
        """HEX状态改变"""
        is_hex = state == Qt.CheckState.Checked.value
        self.manager.update_item(item_id, is_hex=is_hex)
    
    def _on_data_changed(self, item_id, data):
        """数据内容改变"""
        self.manager.update_item(item_id, data=data)
    
    def _on_comment_changed(self, item_id, comment):
        """注释改变"""
        self.manager.update_item(item_id, comment=comment)
    
    def _on_order_changed(self, item_id, text):
        """序号改变"""
        try:
            order = int(text) if text else 0
        except ValueError:
            order = 0
        self.manager.update_item(item_id, sort_order=order)
        
        # 检查序号重复
        duplicates = self._check_order_duplicates()
        if duplicates and order > 0 and order in duplicates:
            QMessageBox.warning(self, "序号重复", f"序号 {order} 已存在，请修改")
    
    def _on_delay_changed(self, item_id, text):
        """延时改变"""
        try:
            delay = int(text) if text else 0
        except ValueError:
            delay = 0
        self.manager.update_item(item_id, delay=delay)
    
    def _on_send_started(self):
        """发送开始"""
        self.ui.loopSendButton.setText("停止发送")
        self.ui.loopSendButton.setChecked(True)
    
    def _on_send_finished(self):
        """发送完成"""
        self.ui.loopSendButton.setText("循环发送")
        self.ui.loopSendButton.setChecked(False)
    
    def _on_send_progress(self, current_id, total):
        """发送进度更新"""
        pass
    
    def _on_error(self, error_msg):
        """错误处理"""
        QMessageBox.critical(self, "错误", error_msg)
