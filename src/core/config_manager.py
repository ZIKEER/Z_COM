import os
import json


_DAP_DEVICES = [
    {"vid": 3368, "pid": 516, "name": "ARM DAPLink"},
    {"vid": 3368, "pid": 4112, "name": "ARM mbed CMSIS-DAP"},
    {"vid": 1155, "pid": 14155, "name": "ST-Link/V2-1 CMSIS-DAP"},
    {"vid": 1155, "pid": 14158, "name": "ST-Link/V3 CMSIS-DAP"},
    {"vid": 8138, "pid": 544, "name": "Maxim MAX32625 CMSIS-DAP"},
    {"vid": 8137, "pid": 304, "name": "NXP LPC-Link2"},
    {"vid": 1046, "pid": 58456, "name": "Nu-Link CMSIS-DAP"},
    {"vid": 49737, "pid": 49154, "name": "Nu-Link2 CMSIS-DAP"},
    {"vid": 17224, "pid": 21920, "name": "WCH CMSIS-DAP"},
    {"vid": 11946, "pid": 12, "name": "Raspberry Pi Pico CMSIS-DAP"},
    {"vid": 10374, "pid": 45, "name": "Seeed XIAO CMSIS-DAP"},
    {"vid": 4966, "pid": 257, "name": "JLink CMSIS-DAP"},
    {"vid": 49745, "pid": 61450, "name": "H7-TOOL CMSIS-DAP"},
]


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
            'rtt_frame_timeout': 50,
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
            'frame_timeout': int(self.config.get('rtt_frame_timeout', 50)),
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

    # --- DAPLink VID/PID 白名单 ---

    def get_dap_devices(self):
        """加载 DAPLink 设备白名单

        Returns:
            list: [{'vid': int, 'pid': int, 'name': str}, ...]
        """
        dap_file = os.path.join(self.config_dir, "dap_devices.json")
        if not os.path.exists(dap_file):
            dap_file = os.path.join("config", "dap_devices.json")
        try:
            if os.path.exists(dap_file):
                with open(dap_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    devices = data.get('devices', [])
                    if devices:
                        return devices
        except Exception as e:
            print(f"加载 dap_devices.json 失败: {e}")
        return []



def _load_external_dap_devices():
    candidates = [
        os.path.join("config", "dap_devices.json"),
        os.path.join(os.getcwd(), "config", "dap_devices.json"),
    ]
    for dap_file in candidates:
        try:
            if os.path.exists(dap_file):
                with open(dap_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                devices = data.get('devices', [])
                if isinstance(devices, list):
                    return devices
        except Exception as e:
            print(f"加载 dap_devices.json 失败: {e}")
    return []


def get_known_dap_devices():
    """返回内置和外部配置合并后的 CMSIS-DAP 设备列表。"""
    merged = {}
    for item in _DAP_DEVICES + _load_external_dap_devices():
        try:
            vid = int(item['vid'])
            pid = int(item['pid'])
        except Exception:
            continue
        merged[(vid, pid)] = {
            'vid': vid,
            'pid': pid,
            'name': item.get('name', f'{vid:04X}:{pid:04X}'),
        }
    return list(merged.values())


def get_known_dap_vendor_ids():
    return {item['vid'] for item in get_known_dap_devices()}


def is_dap_device(vid, pid):
    """检查 VID/PID 是否在已知 CMSIS-DAP 设备列表中"""
    return any(
        d['vid'] == vid and d['pid'] == pid for d in get_known_dap_devices()
    )


def lookup_dap_device(device_id):
    """根据 device_id 字符串查找 VID/PID

    device_id 格式: VID_PID_serial 或 serial
    Returns:
        (vid, pid) or None
    """
    for d in get_known_dap_devices():
        uid = f"{d['vid']:04X}_{d['pid']:04X}"
        if uid in device_id:
            return d['vid'], d['pid']
    return None
