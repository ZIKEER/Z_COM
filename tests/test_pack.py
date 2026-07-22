import os

import pytest

import pack


@pytest.mark.parametrize("separator", [";", ":"])
def test_make_add_data_arg_uses_platform_separator(monkeypatch, tmp_path, separator):
    monkeypatch.setattr(pack.os, "pathsep", separator)

    result = pack.make_add_data_arg(str(tmp_path), "ui", "ui")

    assert result == f"{os.path.join(tmp_path, 'ui')}{separator}ui"
