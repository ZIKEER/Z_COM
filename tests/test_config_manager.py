import os
import json
from src.core.config_manager import ConfigManager


class TestConfigManager:
    def test_default_values(self, tmp_config_dir):
        cm = ConfigManager(config_dir=tmp_config_dir)
        assert cm.get("baudrate") == "115200"
        assert cm.get("display_ansi") is False
        assert cm.get("nonexistent", "default") == "default"

    def test_set_and_get(self, tmp_config_dir):
        cm = ConfigManager(config_dir=tmp_config_dir)
        cm.set("baudrate", "9600")
        assert cm.get("baudrate") == "9600"

    def test_persist_to_disk(self, tmp_config_dir):
        cm = ConfigManager(config_dir=tmp_config_dir)
        cm.set("baudrate", "19200")
        cm.save()

        config_file = os.path.join(tmp_config_dir, "settings.json")
        assert os.path.exists(config_file)

        with open(config_file, "r") as f:
            saved = json.load(f)
        assert saved["baudrate"] == "19200"

    def test_load_from_disk(self, tmp_config_dir):
        os.makedirs(tmp_config_dir, exist_ok=True)
        config_file = os.path.join(tmp_config_dir, "settings.json")
        with open(config_file, "w") as f:
            json.dump({"baudrate": "38400"}, f)

        cm = ConfigManager(config_dir=tmp_config_dir)
        assert cm.get("baudrate") == "38400"
        assert cm.get("port") == ""

    def test_serial_settings(self, tmp_config_dir):
        cm = ConfigManager(config_dir=tmp_config_dir)
        s = cm.get_serial_settings()
        assert s["baudrate"] == 115200
        assert s["databits"] == 8
        assert s["stopbits"] == 1.0
        assert s["parity"] == "None"

    def test_rtt_settings(self, tmp_config_dir):
        cm = ConfigManager(config_dir=tmp_config_dir)
        r = cm.get_rtt_settings()
        assert r["speed"] == 4000
        assert r["chip"] == ""
        assert r["reset"] is False

    def test_rtt_chip_history(self, tmp_config_dir):
        cm = ConfigManager(config_dir=tmp_config_dir)
        cm.add_rtt_chip_history("nRF52840")
        assert cm.get("rtt_chip_history")[0] == "nRF52840"

        cm.add_rtt_chip_history("STM32F103")
        history = cm.get("rtt_chip_history")
        assert history[0] == "STM32F103"
        assert history[1] == "nRF52840"

    def test_rtt_chip_history_dedup(self, tmp_config_dir):
        cm = ConfigManager(config_dir=tmp_config_dir)
        cm.add_rtt_chip_history("nRF52840")
        cm.add_rtt_chip_history("nRF52840")
        assert len(cm.get("rtt_chip_history")) == 1

    def test_instance_isolation(self, tmp_path):
        cm1 = ConfigManager(config_dir=str(tmp_path / "inst1" / "config"))
        cm2 = ConfigManager(config_dir=str(tmp_path / "inst2" / "config"))
        cm1.set("baudrate", "9600")
        cm1.save()
        assert cm2.get("baudrate") == "115200"

    def test_load_failure_uses_defaults(self, tmp_config_dir):
        config_file = os.path.join(tmp_config_dir, "settings.json")
        os.makedirs(tmp_config_dir, exist_ok=True)
        with open(config_file, "w") as f:
            f.write("not json")

        cm = ConfigManager(config_dir=tmp_config_dir)
        assert cm.get("baudrate") == "115200"
