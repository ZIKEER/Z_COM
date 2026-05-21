from unittest.mock import MagicMock, patch

import src.io.serial_manager as sm_mod


class TestSerialManager:
    def test_init(self, qapp):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()
        assert mgr.is_connected is False
        assert mgr.settings["baudrate"] == 115200

    def test_get_available_ports(self, qapp):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()
        with patch.object(sm_mod.serial.tools.list_ports, "comports", return_value=[]):
            assert mgr.get_available_ports() == []

    def test_get_available_ports_with_data(self, qapp):
        from src.io.serial_manager import SerialManager

        mock_port = MagicMock()
        mock_port.device = "COM3"
        mock_port.description = "USB Serial Port (COM3)"
        mgr = SerialManager()

        with patch.object(sm_mod.serial.tools.list_ports, "comports", return_value=[mock_port]):
            ports = mgr.get_available_ports()
            assert ports == [("COM3", "USB Serial Port (COM3)")]

    def test_update_settings(self, qapp):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()
        mgr.update_settings({"baudrate": 9600, "databits": 7})
        assert mgr.settings["baudrate"] == 9600
        assert mgr.settings["databits"] == 7

    def test_connect_calls_serial_open(self, qapp, qtbot):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()

        mock_ser = MagicMock()
        mgr.serial = mock_ser

        with patch.object(sm_mod, "SerialReaderThread") as mock_thread_cls:
            mock_thread = MagicMock()
            mock_thread_cls.return_value = mock_thread

            with qtbot.waitSignal(mgr.connection_changed, timeout=2000):
                result = mgr.connect("COM3")

        assert result is True
        assert mgr.is_connected is True
        mock_ser.open.assert_called_once()

    def test_connect_failure_emits_error(self, qapp, qtbot):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()

        mock_ser = MagicMock()
        mock_ser.open.side_effect = Exception("Port not found")
        mgr.serial = mock_ser

        with qtbot.waitSignal(mgr.error_occurred, timeout=2000):
            result = mgr.connect("COM99")

        assert result is False
        assert mgr.is_connected is False

    def test_disconnect(self, qapp, qtbot):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()

        mock_ser = MagicMock()
        mock_ser.is_open = True
        mock_ser.port = "COM3"
        mgr.serial = mock_ser
        mgr.is_connected = True

        with qtbot.waitSignal(mgr.connection_changed, timeout=2000):
            result = mgr.disconnect()

        assert result is True
        mock_ser.close.assert_called_once()

    def test_send_data(self, qapp):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()

        mock_ser = MagicMock()
        mgr.serial = mock_ser
        mgr.is_connected = True

        result = mgr.send_data(b"Hello")
        assert result is True
        mock_ser.write.assert_called_with(b"Hello")

    def test_send_data_not_connected(self, qapp):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()
        mgr.is_connected = False
        result = mgr.send_data(b"Hello")
        assert result is False

    def test_reconfigure(self, qapp):
        from src.io.serial_manager import SerialManager
        mgr = SerialManager()

        mock_ser = MagicMock()
        mock_ser.is_open = True
        mgr.serial = mock_ser
        mgr.is_connected = True

        result = mgr.reconfigure()
        assert result is True
