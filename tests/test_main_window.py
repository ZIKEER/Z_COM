import pytest
from unittest.mock import patch


@pytest.fixture
def main_window(qapp, qtbot, tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    from src.windows.main_window import MainWindow
    win = MainWindow(instance_id=1, data_root=str(tmp_path))
    qtbot.addWidget(win)
    return win


class TestMainWindowInit:
    def test_title_contains_app_name(self, main_window):
        assert "Z_COM" in main_window.windowTitle()
        assert "V" in main_window.windowTitle()

    def test_initial_io_mode_is_serial(self, main_window):
        assert main_window.io_mode == "serial"

    def test_display_ansi_default_false(self, main_window):
        assert main_window.display_ansi is False

    def test_receive_count_zero(self, main_window):
        assert main_window._display_handler.receive_count == 0

    def test_send_count_zero(self, main_window):
        assert main_window.send_count == 0

    def test_baudrate_in_combo(self, main_window):
        texts = [main_window.ui.baudrateCombo.itemText(i) for i in range(main_window.ui.baudrateCombo.count())]
        assert "115200" in texts

    def test_port_combo_has_items(self, main_window):
        assert main_window.ui.portCombo.count() > 0

    def test_open_button_default_text(self, main_window):
        assert "打开" in main_window.ui.openButton.text()

    def test_status_label_disconnected(self, main_window):
        assert "已断开" in main_window._status_bar.status_label.text()

    def test_receive_text_edit_exists(self, main_window):
        assert main_window.ui.receiveTextEdit is not None

    def test_ascii_radio_checked_default(self, main_window):
        assert main_window.ui.asciiRadio.isChecked() is True


class TestMainWindowActions:
    def test_toggle_ansi_display(self, main_window):
        main_window.display_ansi = False
        main_window._toggle_ansi_display(True)
        assert main_window.display_ansi is True

        main_window._toggle_ansi_display(False)
        assert main_window.display_ansi is False

    def test_clear_receive(self, main_window):
        main_window.ui.receiveTextEdit.append("test data")
        assert main_window.ui.receiveTextEdit.toPlainText() != ""
        main_window._clear_receive()
        assert main_window.ui.receiveTextEdit.toPlainText() == ""

    def test_clear_send(self, main_window):
        main_window.ui.sendTextEdit.setPlainText("test")
        main_window._clear_send()
        assert main_window.ui.sendTextEdit.toPlainText() == ""

    def test_toggle_preset_panel_updates_visibility_and_checks(self, main_window):
        main_window._toggle_preset_panel(True)

        assert main_window.ui.extendedSendContainer.isHidden() is False
        assert main_window.ui.togglePresetButton.isChecked() is True
        assert main_window.ui.togglePresetAction.isChecked() is True

        main_window._toggle_preset_panel(False)

        assert main_window.ui.extendedSendContainer.isHidden() is True
        assert main_window.ui.togglePresetButton.isChecked() is False
        assert main_window.ui.togglePresetAction.isChecked() is False

    def test_splitter_move_saves_sizes_to_config(self, main_window):
        sizes = [420, 280]
        main_window.ui.topSplitter.setSizes(sizes)
        main_window._on_top_splitter_moved(0, 0)

        assert main_window.config_manager.get('top_splitter_sizes') == main_window.ui.topSplitter.sizes()

    def test_baudrate_stack_serial(self, main_window):
        main_window.ui.baudrateStack.setCurrentIndex(0)
        assert main_window.ui.baudrateStack.currentIndex() == 0
        assert main_window.ui.baudrateCombo.isVisibleTo(main_window)

    def test_display_mode_property(self, main_window):
        main_window.ui.hexRadio.setChecked(True)
        assert main_window._display_mode == "HEX"

        main_window.ui.asciiRadio.setChecked(True)
        assert main_window._display_mode == "ASCII"

        main_window.ui.mixedRadio.setChecked(True)
        assert main_window._display_mode == "MIXED"

    def test_error_message_shows_dialog(self, main_window, qtbot):
        with patch("src.windows.main_window.QMessageBox.critical") as critical:
            main_window._on_error("test error")
        critical.assert_called_once()


class TestMainWindowDataDisplay:
    def test_append_data_lines_increments_count(self, main_window):
        main_window._display_handler.receive_count = 0
        main_window._display_handler.append_data(b"test", "\u2190", "RECEIVE")
        assert main_window._display_handler.receive_count == 4

    def test_append_data_lines_send_count(self, main_window):
        main_window.send_count = 0
        main_window.send_count += len(b"test")
        main_window._display_handler.append_data(b"test", "\u2192", "SEND")
        assert main_window.send_count == 4


class TestMainWindowViaConfigManager:
    def test_config_manager_initialized(self, main_window):
        assert main_window.config_manager is not None

    def test_serial_manager_initialized(self, main_window):
        assert main_window.serial_manager is not None

    def test_socket_manager_initialized(self, main_window):
        assert main_window.socket_manager is not None

    def test_rtt_manager_initialized(self, main_window):
        assert main_window.rtt_manager is not None

    def test_io_property_serial(self, main_window):
        main_window.io_mode = "serial"
        assert main_window._io is main_window.serial_manager

    def test_io_property_socket(self, main_window):
        main_window.io_mode = "socket"
        assert main_window._io is main_window.socket_manager

    def test_io_property_rtt(self, main_window):
        main_window.io_mode = "rtt"
        assert main_window._io is main_window.rtt_manager


class TestMainWindowCleanup:
    def test_jlink_scan_thread_is_cleaned_up_on_close(self, qapp, qtbot, tmp_path, monkeypatch):
        monkeypatch.chdir(tmp_path)
        from src.windows.main_window import MainWindow

        win = MainWindow(instance_id=1, data_root=str(tmp_path))
        qtbot.addWidget(win)
        qtbot.waitUntil(lambda: win._jlink_scan_thread is None or not win._jlink_scan_thread.isRunning(), timeout=3000)

        win.close()

        assert win._jlink_scan_thread is None
