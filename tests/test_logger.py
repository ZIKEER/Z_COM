import os
import pytest
from src.core.logger import Logger, MAX_LOG_FILE_SIZE


@pytest.fixture(autouse=True)
def _reset_logger_counter():
    Logger._global_counter = 0
    yield


def test_log_creates_file(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "RECEIVE", "48 65 6C 6C 6F", "Hello")
    log.flush()
    files = os.listdir(tmp_path)
    assert len(files) == 1
    assert files[0].startswith("log_")
    assert files[0].endswith(".txt")


def test_data_root_is_independent_of_working_directory(tmp_path, monkeypatch):
    data_root = tmp_path / "app"
    monkeypatch.chdir(tmp_path)
    log = Logger(data_root=str(data_root))
    assert log.log_dir == str(data_root / "logs")


def _read_log(tmp_path):
    log_file = os.listdir(tmp_path)[0]
    with open(os.path.join(tmp_path, log_file), "r", encoding="utf-8") as f:
        return f.read()


def test_log_content(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "RECEIVE", "48 65 6C 6C 6F", "Hello")
    log.flush()

    content = _read_log(tmp_path)
    assert "12:00:00.000" in content
    assert "HEX" in content
    assert "ASCII" in content
    assert "Hello" in content
    assert "48 65 6C 6C 6F" in content


def test_log_send_direction(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "SEND", "48 69", "Hi")
    log.flush()

    content = _read_log(tmp_path)
    assert "→" in content


def test_log_receive_direction(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "RECEIVE", "48 69", "Hi")
    log.flush()

    content = _read_log(tmp_path)
    assert "←" in content


def test_multiple_logs_same_file(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "RECEIVE", "41", "A")
    log.log("12:00:01.000", "RECEIVE", "42", "B")
    log.flush()

    content = _read_log(tmp_path)
    assert content.count("12:00:00.000") == 1
    assert content.count("12:00:01.000") == 1


def test_hex_data_in_log(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "RECEIVE", "00 FF AB", "\\x00\\xff\\xab")
    log.flush()

    content = _read_log(tmp_path)
    assert "00 FF AB" in content


def test_log_event(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log_event("软件启动 Z_COM V1.0.0")
    log.log_event("已连接 COM3")
    log.flush()

    content = _read_log(tmp_path)
    assert "软件启动 Z_COM V1.0.0" in content
    assert "已连接 COM3" in content
    assert "[" in content  # timestamp bracket


def test_new_file_on_init(tmp_path):
    log1 = Logger(log_dir=str(tmp_path))
    log1.log_event("first file")
    log1.flush()

    log2 = Logger(log_dir=str(tmp_path))
    log2.log_event("second file")
    log2.flush()

    files = sorted(os.listdir(tmp_path))
    assert len(files) == 2
    assert files[0] != files[1]

    with open(os.path.join(tmp_path, files[0]), "r", encoding="utf-8") as f:
        assert "first file" in f.read()
    with open(os.path.join(tmp_path, files[1]), "r", encoding="utf-8") as f:
        assert "second file" in f.read()


def test_max_file_size_rotation(tmp_path):
    log = Logger(log_dir=str(tmp_path))

    # Write enough data to exceed the max file size in one flush
    big_line = "X" * 1024 + "\n"
    count = MAX_LOG_FILE_SIZE // len(big_line) + 10
    for _ in range(count):
        log.log_event(big_line)
    log.flush()
    # After flush, file exceeds limit and rotates. Write one more entry to the new file.
    log.log_event("after rotation")
    log.flush()

    files = sorted(os.listdir(tmp_path))
    assert len(files) >= 2
    assert os.path.getsize(os.path.join(tmp_path, files[0])) >= MAX_LOG_FILE_SIZE


def test_file_name_format(tmp_path):
    import re
    log = Logger(log_dir=str(tmp_path))
    log.log_event("test")
    log.flush()

    files = os.listdir(tmp_path)
    assert len(files) == 1
    name = files[0]
    # log_YYYY-MM-DD_HHMMSS.txt or log_YYYY-MM-DD_HHMMSS_N.txt
    assert re.match(r'^log_\d{4}-\d{2}-\d{2}_\d{6}(_\d+)?\.txt$', name)
