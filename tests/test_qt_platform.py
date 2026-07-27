from src.core import qt_platform


def test_configure_qt_platform_uses_xcb_on_wsl(monkeypatch):
    monkeypatch.setattr(qt_platform.sys, "platform", "linux")
    monkeypatch.setenv("WSL_DISTRO_NAME", "Ubuntu-24.04")
    monkeypatch.setenv("DISPLAY", ":0")
    monkeypatch.delenv("QT_QPA_PLATFORM", raising=False)

    qt_platform.configure_qt_platform()

    assert qt_platform.os.environ["QT_QPA_PLATFORM"] == "xcb"


def test_configure_qt_platform_preserves_explicit_backend(monkeypatch):
    monkeypatch.setattr(qt_platform.sys, "platform", "linux")
    monkeypatch.setenv("WSL_DISTRO_NAME", "Ubuntu-24.04")
    monkeypatch.setenv("DISPLAY", ":0")
    monkeypatch.setenv("QT_QPA_PLATFORM", "wayland")

    qt_platform.configure_qt_platform()

    assert qt_platform.os.environ["QT_QPA_PLATFORM"] == "wayland"
