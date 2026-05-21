from unittest.mock import MagicMock, Mock, patch


def _make_mock_jlink():
    jl = MagicMock()
    jl.rtt_read.return_value = b""
    jl.rtt_write.return_value = 0
    return jl


class TestRttManager:
    def test_init(self, qapp):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()
        assert mgr.is_connected is False
        assert mgr.settings["speed"] == 4000

    def test_get_available_devices_no_pylink(self, qapp):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()
        with patch.object(mgr, "_import_pylink", return_value=None):
            assert mgr.get_available_devices() == []

    def test_get_available_devices_with_pylink(self, qapp):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()

        mock_pylink = MagicMock()
        mock_jlink_temp = MagicMock()
        mock_jlink_temp.connected_emulators.return_value = []
        # 避免 fallback 路径创建虚假设备
        mock_jlink_temp.open.side_effect = Exception("mock no device")
        mock_pylink.JLink.return_value = mock_jlink_temp

        with patch.object(mgr, "_import_pylink", return_value=mock_pylink):
            assert mgr.get_available_devices() == []

    def test_update_settings(self, qapp):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()
        mgr.update_settings({"speed": 10000, "chip": "nRF52840"})
        assert mgr.settings["speed"] == 10000
        assert mgr.settings["chip"] == "nRF52840"

    def test_connect_with_pylink(self, qapp, qtbot):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()

        mock_pylink = MagicMock()
        mock_jlink = _make_mock_jlink()
        mock_pylink.JLink.return_value = mock_jlink

        with patch.object(mgr, "_import_pylink", return_value=mock_pylink), \
             patch("src.io.rtt_manager.RttReaderThread") as mock_thread_cls:
            mock_thread = MagicMock()
            mock_thread_cls.return_value = mock_thread

            with qtbot.waitSignal(mgr.connection_changed, timeout=2000):
                result = mgr.connect(serial_no=12345, chip="nRF52840", speed=4000,
                                     reset_flag=False, start_address=None, range_size=None)

        assert result is True
        assert mgr.is_connected is True

    def test_connect_failure(self, qapp, qtbot):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()

        mock_pylink = MagicMock()
        mock_jlink = _make_mock_jlink()
        mock_jlink.connect.side_effect = Exception("No J-Link found")
        mock_pylink.JLink.return_value = mock_jlink

        with patch.object(mgr, "_import_pylink", return_value=mock_pylink):
            with qtbot.waitSignal(mgr.error_occurred, timeout=2000):
                result = mgr.connect(serial_no=99999, chip="nRF52840", speed=4000,
                                     reset_flag=False, start_address=None, range_size=None)

        assert result is False
        assert mgr.is_connected is False

    def test_disconnect(self, qapp, qtbot):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()

        mock_jlink = _make_mock_jlink()
        mgr.jlink = mock_jlink
        mgr.is_connected = True

        with qtbot.waitSignal(mgr.connection_changed, timeout=2000):
            result = mgr.disconnect()

        assert result is True
        assert mgr.is_connected is False

    def test_send_data(self, qapp):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()

        mock_jlink = _make_mock_jlink()
        mgr.jlink = mock_jlink
        mgr.is_connected = True

        result = mgr.send_data(b"Hello")
        assert result is True
        args, _ = mock_jlink.rtt_write.call_args
        assert args[0] == 0
        assert bytes(args[1]) == b"Hello"

    def test_send_data_not_connected(self, qapp):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()
        mgr.is_connected = False
        result = mgr.send_data(b"Hello")
        assert result is False

    def test_send_data_hex_string(self, qapp):
        from src.io.rtt_manager import RttManager
        mgr = RttManager()

        mock_jlink = _make_mock_jlink()
        mgr.jlink = mock_jlink
        mgr.is_connected = True

        result = mgr.send_data("48656C6C6F", is_hex=True)
        assert result is True
        args, _ = mock_jlink.rtt_write.call_args
        assert args[0] == 0
        assert bytes(args[1]) == b"Hello"
