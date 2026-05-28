import os
from src.core.logger import Logger


def test_log_creates_file(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "RECEIVE", "48 65 6C 6C 6F", "Hello")
    log.flush()
    files = os.listdir(tmp_path)
    assert len(files) == 1
    assert files[0].startswith("log_")
    assert files[0].endswith(".txt")


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
    assert "\u2192" in content


def test_log_receive_direction(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log("12:00:00.000", "RECEIVE", "48 69", "Hi")
    log.flush()

    content = _read_log(tmp_path)
    assert "\u2190" in content


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
