import os
import json


class ConfigManager:
    """配置管理类"""
    
    def __init__(self, config_dir="config"):
        self.config_dir = config_dir
        self.config_file = os.path.join(config_dir, "settings.json")
        self.default_config = {
            'port': '',
            'baudrate': '115200',
            'databits': '8',
            'stopbits': '1',
            'parity': 'None',
            'flowcontrol': 'None',
            'display_mode': 'ASCII',  # HEX, ASCII, MIXED
            'send_mode': 'ASCII',     # ASCII, HEX
            'auto_scroll': True,
            'auto_send_interval': 1000
        }
        self.config = self.default_config.copy()
        self._load_config()
    
    def _load_config(self):
        """加载配置"""
        try:
            if os.path.exists(self.config_file):
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    saved_config = json.load(f)
                    self.config.update(saved_config)
        except Exception as e:
            print(f"加载配置失败: {e}")
    
    def _save_config(self):
        """保存配置"""
        try:
            os.makedirs(self.config_dir, exist_ok=True)
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(self.config, f, ensure_ascii=False, indent=2)
        except Exception as e:
            print(f"保存配置失败: {e}")
    
    def get(self, key, default=None):
        """获取配置值"""
        return self.config.get(key, default)
    
    def set(self, key, value):
        """设置配置值"""
        self.config[key] = value
    
    def save(self):
        """保存配置到文件"""
        self._save_config()
    
    def get_serial_settings(self):
        """获取串口设置"""
        return {
            'baudrate': int(self.config.get('baudrate', 115200)),
            'databits': int(self.config.get('databits', 8)),
            'stopbits': float(self.config.get('stopbits', 1)),
            'parity': self.config.get('parity', 'None'),
            'flowcontrol': self.config.get('flowcontrol', 'None')
        }
