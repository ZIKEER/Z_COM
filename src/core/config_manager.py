import os
import json


class ConfigManager:
    """配置管理类"""
    
    def __init__(self, config_dir=None, instance_id=1):
        if config_dir:
            self.config_dir = config_dir
        elif instance_id > 1:
            self.config_dir = os.path.join(f"instance_{instance_id}", "config")
        else:
            self.config_dir = "config"
        
        self.config_file = os.path.join(self.config_dir, "settings.json")
        self.default_config = {
            'port': '',
            'baudrate': '115200',
            'databits': '8',
            'stopbits': '1',
            'parity': 'None',
            'flowcontrol': 'None',
            'display_mode': 'ASCII',
            'send_mode': 'ASCII',
            'auto_scroll': True,
            'auto_send_interval': 1000,
            'display_ansi': False,
            'rtt_chip': '',
            'rtt_speed': 4000,
            'rtt_reset': False,
            'rtt_start_address': '',
            'rtt_range_size': '',
            'rtt_chip_history': [],
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

    def get_rtt_settings(self):
        """获取 RTT 设置"""
        return {
            'chip': self.config.get('rtt_chip', ''),
            'speed': int(self.config.get('rtt_speed', 4000)),
            'reset': self.config.get('rtt_reset', False),
            'start_address': self.config.get('rtt_start_address', ''),
            'range_size': self.config.get('rtt_range_size', ''),
            'chip_history': self.config.get('rtt_chip_history', []),
        }

    def add_rtt_chip_history(self, chip):
        """添加芯片到历史记录"""
        if not chip or not chip.strip():
            return
        chip = chip.strip()
        history = self.config.get('rtt_chip_history', [])
        if chip in history:
            history.remove(chip)
        history.insert(0, chip)
        if len(history) > 20:
            history = history[:20]
        self.config['rtt_chip_history'] = history
        self._save_config()
