import os
import json
from PySide6.QtCore import QObject, Signal, QTimer


class ExtendedSendManager(QObject):
    """扩展发送管理类，负责管理多条数据的发送"""
    
    send_started = Signal()           # 发送开始信号
    send_finished = Signal()          # 发送完成信号
    send_progress = Signal(int, int)  # 发送进度信号 (当前序号, 总数)
    data_sent = Signal(bytes)         # 数据已发送信号
    error_occurred = Signal(str)      # 错误信号
    items_changed = Signal()          # 数据列表改变信号
    
    def __init__(self, send_func, parent=None):
        super().__init__(parent)
        self.send_func = send_func  # 注入的发送函数
        self.items = []  # 扩展发送数据列表
        self.is_sending = False
        self.is_looping = False
        self.current_index = 0
        self.send_timer = QTimer()
        self.send_timer.setSingleShot(True)
        self.send_timer.timeout.connect(self._on_send_timer_timeout)
        
        # 配置文件路径
        self.config_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "config")
        self.config_file = os.path.join(self.config_dir, "extended_send.json")
        
        # 加载配置
        self._load_config()
    
    def _load_config(self):
        """加载配置"""
        try:
            if os.path.exists(self.config_file):
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    config_data = json.load(f)
                    self.items = config_data.get('items', [])
                    self._reorder_items()
        except Exception as e:
            print(f"加载扩展发送配置失败: {e}")
    
    def _save_config(self):
        """保存配置"""
        try:
            os.makedirs(self.config_dir, exist_ok=True)
            config_data = self.save_to_config()
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(config_data, f, ensure_ascii=False, indent=2)
        except Exception as e:
            print(f"保存扩展发送配置失败: {e}")
    
    def add_item(self, data, is_hex=True, comment="", delay=1000):
        """添加一条数据"""
        item = {
            'id': self._generate_id(),
            'data': data,
            'is_hex': is_hex,
            'comment': comment,
            'delay': delay,
            'sort_order': 0
        }
        self.items.append(item)
        self._save_config()
        self.items_changed.emit()
        return item
    
    def remove_item(self, item_id):
        """删除一条数据"""
        self.items = [item for item in self.items if item['id'] != item_id]
        self._reorder_items()
        self._save_config()
        self.items_changed.emit()
    
    def update_item(self, item_id, **kwargs):
        """更新一条数据"""
        for item in self.items:
            if item['id'] == item_id:
                item.update(kwargs)
                self._save_config()
                self.items_changed.emit()
                break
    
    def move_item(self, item_id, direction):
        """移动数据顺序 (direction: -1=上移, 1=下移)"""
        index = next((i for i, item in enumerate(self.items) if item['id'] == item_id), None)
        if index is None:
            return
        
        new_index = index + direction
        if 0 <= new_index < len(self.items):
            self.items[index], self.items[new_index] = self.items[new_index], self.items[index]
            self._reorder_items()
            self._save_config()
            self.items_changed.emit()
    
    def _reorder_items(self):
        """重新排序"""
        for i, item in enumerate(self.items):
            item['sort_order'] = i
    
    def _generate_id(self):
        """生成唯一ID"""
        if not self.items:
            return 1
        return max(item['id'] for item in self.items) + 1
    
    def get_sorted_items(self):
        """获取序号大于0的数据项，按顺序排列"""
        return sorted(
            [item for item in self.items if item.get('sort_order', 0) > 0],
            key=lambda x: x['sort_order']
        )
    
    def clear_items(self):
        """清空所有数据"""
        self.items.clear()
        self._save_config()
        self.items_changed.emit()
    
    def send_single(self, item_id):
        """发送单条数据"""
        item = next((item for item in self.items if item['id'] == item_id), None)
        if item:
            self._send_item(item)
    
    def send_multiple(self, loop=False):
        """发送多条数据"""
        if self.is_sending:
            return
        
        self.is_sending = True
        self.is_looping = loop
        self.current_index = 0
        self.send_started.emit()
        
        sorted_items = self.get_sorted_items()
        if not sorted_items:
            self.is_sending = False
            self.send_finished.emit()
            return
        
        self._send_next_item(sorted_items)
    
    def _send_next_item(self, items):
        """发送下一条数据"""
        if not self.is_sending:
            return
        
        if self.current_index >= len(items):
            if self.is_looping:
                self.current_index = 0
            else:
                self.is_sending = False
                self.send_finished.emit()
                return
        
        item = items[self.current_index]
        success = self._send_item(item)
        
        if not success:
            self.is_sending = False
            self.send_finished.emit()
            return
        
        self.send_progress.emit(item['id'], len(items))
        
        self.current_index += 1
        
        # 延时后发送下一条
        if item['delay'] > 0:
            self.send_timer.start(item['delay'])
        else:
            self._send_next_item(items)
    
    def _on_send_timer_timeout(self):
        """发送延时定时器超时"""
        sorted_items = self.get_sorted_items()
        self._send_next_item(sorted_items)
    
    def _send_item(self, item):
        """发送单条数据，返回是否成功"""
        try:
            data = item['data']
            is_hex = item['is_hex']
            
            if not data:
                self.error_occurred.emit("数据为空")
                return False
            
            if is_hex:
                hex_str = data.replace(' ', '')
                send_data = bytes.fromhex(hex_str)
            else:
                send_data = data.encode('utf-8')
            
            # 调用注入的发送函数
            success = self.send_func(send_data)
            
            if not success:
                self.error_occurred.emit("发送失败")
                return False
            
            self.data_sent.emit(send_data)
            return True
        except Exception as e:
            self.error_occurred.emit(f"发送失败: {str(e)}")
            return False
    
    def stop_sending(self):
        """停止发送"""
        self.is_sending = False
        self.is_looping = False
        self.send_timer.stop()
    
    def load_from_config(self, config_data):
        """从配置数据加载"""
        self.items = config_data.get('items', [])
        self._reorder_items()
        self._save_config()
        self.items_changed.emit()
    
    def save_to_config(self):
        """保存为配置数据"""
        return {
            'items': self.items,
            'settings': {
                'loop_send': self.is_looping,
                'multi_send': True,
                'default_delay': 1000
            }
        }
    
    def import_from_file(self, file_path):
        """从文件导入配置"""
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                config_data = json.load(f)
            self.load_from_config(config_data)
            return True
        except Exception as e:
            self.error_occurred.emit(f"导入失败: {str(e)}")
            return False
    
    def export_to_file(self, file_path):
        """导出配置到文件"""
        try:
            config_data = self.save_to_config()
            with open(file_path, 'w', encoding='utf-8') as f:
                json.dump(config_data, f, ensure_ascii=False, indent=2)
            return True
        except Exception as e:
            self.error_occurred.emit(f"导出失败: {str(e)}")
            return False
