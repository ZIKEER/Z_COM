import os
import json
from src.core.extended_send_manager import ExtendedSendManager


class TestExtendedSendManager:
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
