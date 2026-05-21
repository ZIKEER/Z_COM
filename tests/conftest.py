import sys
import os
import pytest
from PySide6.QtWidgets import QApplication

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from src.core.extended_send_manager import ExtendedSendManager
from src.core.config_manager import ConfigManager


@pytest.fixture(scope="session")
def qapp():
    app = QApplication.instance()
    if app is None:
        app = QApplication(sys.argv)
    return app


@pytest.fixture
def mock_send_func():
    def send_func(data):
        return True
    return send_func


@pytest.fixture
def tmp_config_dir(tmp_path):
    return str(tmp_path / "config")


@pytest.fixture
def config_manager(tmp_config_dir):
    cm = ConfigManager(config_dir=tmp_config_dir)
    cm._save_config()
    return cm


@pytest.fixture
def extended_send_manager(mock_send_func):
    mgr = ExtendedSendManager(mock_send_func)
    mgr.items = []
    return mgr
