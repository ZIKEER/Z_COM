import os
import sys


def configure_qt_platform():
    """WSLg 的 Wayland 窗口装饰不稳定，默认交给 XWayland 管理标题栏。"""
    is_wsl = "WSL_DISTRO_NAME" in os.environ or "WSL_INTEROP" in os.environ
    if sys.platform.startswith("linux") and is_wsl and os.environ.get("DISPLAY"):
        os.environ.setdefault("QT_QPA_PLATFORM", "xcb")
