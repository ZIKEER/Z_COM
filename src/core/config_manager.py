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
            'frame_timeout': 50,
            'display_mode': 'ASCII',
            'send_mode': 'ASCII',
            'auto_scroll': True,
            'auto_send_interval': 1000,
            'display_ansi': False,
            'support_jlink': False,
            'rtt_chip': '',
            'rtt_speed': 4000,
            'rtt_reset': False,
            'rtt_start_address': '',
            'rtt_range_size': '',
            'rtt_chip_history': [],
            'rtt_frame_timeout': 50,
            'main_splitter_sizes': [590, 92],
            'top_splitter_sizes': [700, 320],
            'preset_panel_visible': False,
        }
        self.config = self.default_config.copy()
        self._load_config()

    @staticmethod
    def _coerce_int(value, default, minimum=None, maximum=None):
        try:
            result = int(value)
        except (TypeError, ValueError):
            result = default
        if minimum is not None:
            result = max(result, minimum)
        if maximum is not None:
            result = min(result, maximum)
        return result

    @staticmethod
    def _coerce_float(value, default, minimum=None, maximum=None):
        try:
            result = float(value)
        except (TypeError, ValueError):
            result = default
        if minimum is not None:
            result = max(result, minimum)
        if maximum is not None:
            result = min(result, maximum)
        return result

    @staticmethod
    def _coerce_bool(value, default=False):
        if isinstance(value, bool):
            return value
        if isinstance(value, str):
            normalized = value.strip().lower()
            if normalized in ('1', 'true', 'yes', 'on'):
                return True
            if normalized in ('0', 'false', 'no', 'off'):
                return False
        return default if value is None else bool(value)

    def get_int(self, key, default=0, minimum=None, maximum=None):
        return self._coerce_int(self.config.get(key, default), default, minimum, maximum)

    def get_float(self, key, default=0.0, minimum=None, maximum=None):
        return self._coerce_float(self.config.get(key, default), default, minimum, maximum)

    def get_bool(self, key, default=False):
        return self._coerce_bool(self.config.get(key, default), default)

    def get_int_list(self, key, default, expected_len=None, minimum=None, maximum=None):
        value = self.config.get(key, default)
        if not isinstance(value, list):
            return list(default)
        result = [
            self._coerce_int(item, fallback, minimum, maximum)
            for item, fallback in zip(value, default)
        ]
        if len(value) > len(default):
            result.extend(
                self._coerce_int(item, 0, minimum, maximum)
                for item in value[len(default):]
            )
        if expected_len is not None and len(result) != expected_len:
            return list(default)
        return result
    
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
            'baudrate': self.get_int('baudrate', 115200, minimum=1),
            'databits': self.get_int('databits', 8, minimum=5, maximum=8),
            'stopbits': self.get_float('stopbits', 1, minimum=1, maximum=2),
            'parity': self.config.get('parity', 'None'),
            'flowcontrol': self.config.get('flowcontrol', 'None'),
            'frame_timeout': self.get_int(
                'frame_timeout',
                self.get_int('rtt_frame_timeout', 50, minimum=1),
                minimum=1,
            ),
        }

    def get_rtt_settings(self):
        """获取 RTT 设置"""
        return {
            'chip': self.config.get('rtt_chip', ''),
            'speed': self.get_int('rtt_speed', 4000, minimum=1),
            'reset': self.get_bool('rtt_reset', False),
            'start_address': self.config.get('rtt_start_address', ''),
            'range_size': self.config.get('rtt_range_size', ''),
            'chip_history': self.config.get('rtt_chip_history', [])
            if isinstance(self.config.get('rtt_chip_history', []), list) else [],
            'frame_timeout': self.get_int(
                'frame_timeout',
                self.get_int('rtt_frame_timeout', 50, minimum=1),
                minimum=1,
            ),
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
