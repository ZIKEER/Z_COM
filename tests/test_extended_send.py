import os
import json
from unittest.mock import patch
from src.core.extended_send_manager import ExtendedSendManager, decode_ascii_escapes, encode_ascii_for_display
from src.windows.extended_send_widget import (
    SendItemWidget,
    DEFAULT_SEND_BUTTON_TEXT,
    ELLIPSIS_PREFIX_CHARS,
)


class TestExtendedSendManager:
    def test_decode_ascii_escapes(self):
        assert decode_ascii_escapes(r"A\r\nB\t\x21\\") == "A\r\nB\t!\\"

    def test_encode_ascii_for_display(self):
        assert encode_ascii_for_display("A\r\nB\t\\") == r"A\r\nB\t\\"

    def test_add_item(self, extended_send_manager):
        mgr = extended_send_manager
        item = mgr.add_item("48656C6C6F", is_hex=True, comment="hello")
        assert item["id"] == 1
        assert item["sort_order"] == 0
        assert len(mgr.items) == 1

    def test_remove_item(self, extended_send_manager):
        mgr = extended_send_manager
        item = mgr.add_item("48656C6C6F", is_hex=True)
        mgr.remove_item(item["id"])
        assert len(mgr.items) == 0

    def test_update_item(self, extended_send_manager):
        mgr = extended_send_manager
        item = mgr.add_item("48656C6C6F", is_hex=True)
        mgr.update_item(item["id"], comment="updated")
        assert mgr.items[0]["comment"] == "updated"

    def test_move_item_up(self, extended_send_manager):
        mgr = extended_send_manager
        a = mgr.add_item("A", is_hex=False)
        b = mgr.add_item("B", is_hex=False)
        mgr.move_item(b["id"], -1)
        assert mgr.items[0]["id"] == b["id"]

    def test_move_item_down(self, extended_send_manager):
        mgr = extended_send_manager
        a = mgr.add_item("A", is_hex=False)
        b = mgr.add_item("B", is_hex=False)
        mgr.move_item(a["id"], 1)
        assert mgr.items[1]["id"] == a["id"]

    def test_get_sorted_items_excludes_zero(self, extended_send_manager):
        mgr = extended_send_manager
        mgr.add_item("A", is_hex=False)
        mgr.add_item("B", is_hex=False)
        sorted_items = mgr.get_sorted_items()
        assert len(sorted_items) == 0

    def test_get_sorted_items(self, extended_send_manager):
        mgr = extended_send_manager
        a = mgr.add_item("A", is_hex=False)
        b = mgr.add_item("B", is_hex=False)
        a["sort_order"] = 2
        b["sort_order"] = 1
        sorted_items = mgr.get_sorted_items()
        assert len(sorted_items) == 2
        assert sorted_items[0]["id"] == b["id"]

    def test_clear_items(self, extended_send_manager):
        mgr = extended_send_manager
        mgr.add_item("A", is_hex=False)
        mgr.clear_items()
        assert len(mgr.items) == 0

    def test_send_single_triggers_data_sent(self, extended_send_manager, qtbot):
        mgr = extended_send_manager
        item = mgr.add_item("48656C6C6F", is_hex=True)

        with qtbot.waitSignal(mgr.data_sent, timeout=1000):
            mgr.send_single(item["id"])

    def test_send_single_ascii_escape_sequences(self, extended_send_manager, qtbot):
        mgr = extended_send_manager
        item = mgr.add_item(r"AT\r\nLOGIN\tOK\x21", is_hex=False)

        with qtbot.waitSignal(mgr.data_sent, timeout=1000) as blocker:
            mgr.send_single(item["id"])

        assert blocker.args == [b"AT\r\nLOGIN\tOK!"]

    def test_send_single_ascii_multiline_raw_text(self, extended_send_manager, qtbot):
        mgr = extended_send_manager
        item = mgr.add_item("line1\nline2\tend", is_hex=False)

        with qtbot.waitSignal(mgr.data_sent, timeout=1000) as blocker:
            mgr.send_single(item["id"])

        assert blocker.args == [b"line1\nline2\tend"]

    def test_send_multiple(self, extended_send_manager, qtbot):
        mgr = extended_send_manager
        mgr.add_item("41", is_hex=True, delay=1)
        mgr.add_item("42", is_hex=True, delay=1)
        a = mgr.items[-2]
        b = mgr.items[-1]
        a["sort_order"] = 1
        b["sort_order"] = 2

        with qtbot.waitSignal(mgr.send_finished, timeout=5000):
            mgr.send_multiple(loop=False)

    def test_stop_sending(self, extended_send_manager):
        mgr = extended_send_manager
        mgr.add_item("41", is_hex=True, delay=100000)
        mgr.items[-1]["sort_order"] = 1
        mgr.send_multiple(loop=True)
        assert mgr.is_sending is True
        mgr.stop_sending()
        assert mgr.is_sending is False
        assert mgr.is_looping is False

    def test_empty_data_emits_error(self, extended_send_manager, qtbot):
        mgr = extended_send_manager
        item = mgr.add_item("", is_hex=False)

        with qtbot.waitSignal(mgr.error_occurred, timeout=1000):
            mgr.send_single(item["id"])

    def test_export_import(self, extended_send_manager, tmp_path):
        mgr = extended_send_manager
        mgr.add_item("48656C6C6F", is_hex=True)

        file_path = str(tmp_path / "export.json")
        assert mgr.export_to_file(file_path) is True
        assert os.path.exists(file_path)

        mgr.clear_items()
        assert mgr.import_from_file(file_path) is True
        assert len(mgr.items) == 1

    def test_generate_id_increments(self, extended_send_manager):
        mgr = extended_send_manager
        a = mgr.add_item("A", is_hex=False)
        b = mgr.add_item("B", is_hex=False)
        assert b["id"] == a["id"] + 1


class TestSendItemWidget:
    def test_comment_button_shows_comment_instead_of_default_text(self, qapp):
        widget = SendItemWidget(1, data="ABC", comment="这是一个很长的注释")

        assert "这是" in widget.send_btn.text()
        assert "\n" in widget.send_btn.text() or len(widget.send_btn.text()) <= 2
        assert "注释:" in widget.send_btn.toolTip()

    def test_data_tooltip_contains_full_visible_content(self, qapp):
        widget = SendItemWidget(1, data="line1\nline2\tend", comment="备注")

        assert r"line1\nline2\tend" in widget.data_edit.toolTip()
        assert "注释: 备注" in widget.data_edit.toolTip()

    def test_default_send_button_text_without_comment(self, qapp):
        widget = SendItemWidget(1, data="ABC", comment="")

        assert widget.send_btn.text() == DEFAULT_SEND_BUTTON_TEXT

    def test_comment_button_text_is_two_line_prefix_when_long(self, qapp):
        widget = SendItemWidget(1, data="ABC", comment="这是一个很长很长的注释文本用于测试按钮截断")

        lines = widget.send_btn.text().splitlines()
        assert len(lines) <= 2
        assert lines[0].startswith("这")

    def test_comment_within_16_chars_is_fully_visible(self, qapp):
        comment = "一二三四五六七八九十一二三四五六"
        widget = SendItemWidget(1, data="ABC", comment=comment)

        assert widget.send_btn.text().replace("\n", "") == comment

    def test_comment_over_16_chars_uses_ellipsis(self, qapp):
        comment = "一二三四五六七八九十一二三四五六七八九十"
        widget = SendItemWidget(1, data="ABC", comment=comment)

        rendered = widget.send_btn.text().replace("\n", "")
        assert rendered.endswith("...")
        assert rendered == comment[:ELLIPSIS_PREFIX_CHARS] + "..."


class TestExtendedSendWidget:
    def test_context_menu_does_not_include_send_selected_action(self, qapp, extended_send_manager):
        from src.windows.extended_send_widget import ExtendedSendWidget

        widget = ExtendedSendWidget(extended_send_manager)
        captured = {}

        def fake_exec(menu, *_args, **_kwargs):
            captured["texts"] = [action.text() for action in menu.actions()]
            return None

        with patch("src.windows.extended_send_widget.QMenu.exec_", new=fake_exec):
            widget._show_main_context_menu(widget.rect().center())

        assert "发送选中" not in captured["texts"]
