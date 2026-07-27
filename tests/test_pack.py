import os

import pytest

import pack
import pack_nuitka


@pytest.mark.parametrize("separator", [";", ":"])
def test_make_add_data_arg_uses_platform_separator(monkeypatch, tmp_path, separator):
    monkeypatch.setattr(pack.os, "pathsep", separator)

    result = pack.make_add_data_arg(str(tmp_path), "ui", "ui")

    assert result == f"{os.path.join(tmp_path, 'ui')}{separator}ui"


@pytest.mark.parametrize(
    ("platform", "expected"),
    [("win32", "Z_COM.exe"), ("linux", "Z_COM"), ("darwin", "Z_COM")],
)
def test_executable_name_is_platform_specific(monkeypatch, platform, expected):
    monkeypatch.setattr(pack.sys, "platform", platform)
    monkeypatch.setattr(pack_nuitka.sys, "platform", platform)

    assert pack.get_executable_name("Z_COM") == expected
    assert pack_nuitka.get_executable_name("Z_COM") == expected


def test_linux_build_paths_are_platform_isolated(monkeypatch):
    monkeypatch.setattr(pack.sys, "platform", "linux")
    monkeypatch.setattr(pack.platform, "machine", lambda: "x86_64")

    dist_root, build_root, dist_dir = pack.get_build_paths("Z_COM_V1")

    assert dist_root == os.path.join("dist", "linux-x86_64")
    assert build_root == os.path.join("build", "linux-x86_64")
    assert dist_dir == os.path.join(dist_root, "Z_COM_V1")


def test_windows_build_paths_remain_compatible(monkeypatch):
    monkeypatch.setattr(pack.sys, "platform", "win32")

    dist_root, build_root, dist_dir = pack.get_build_paths("Z_COM_V1")

    assert dist_root == "dist"
    assert build_root == "build"
    assert dist_dir == os.path.join("dist", "Z_COM_V1")


def test_remove_unnecessary_files_is_case_insensitive(tmp_path):
    unused = tmp_path / "Qt6Pdf.DLL"
    unused.write_bytes(b"unused")

    pack.remove_unnecessary_files(tmp_path)

    assert not unused.exists()
