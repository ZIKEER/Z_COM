import json
from unittest.mock import patch

import pytest

from src.core.file_utils import atomic_write_json


def test_atomic_write_json_replaces_file(tmp_path):
    target = tmp_path / "settings.json"
    target.write_text('{"old": true}', encoding="utf-8")

    atomic_write_json(target, {"name": "串口助手", "enabled": True})

    assert json.loads(target.read_text(encoding="utf-8")) == {
        "name": "串口助手",
        "enabled": True,
    }


def test_atomic_write_json_preserves_original_when_replace_fails(tmp_path):
    target = tmp_path / "settings.json"
    original = '{"old": true}'
    target.write_text(original, encoding="utf-8")

    with patch("src.core.file_utils.os.replace", side_effect=OSError("replace failed")):
        with pytest.raises(OSError, match="replace failed"):
            atomic_write_json(target, {"new": True})

    assert target.read_text(encoding="utf-8") == original
    assert list(tmp_path.glob("*.tmp")) == []
