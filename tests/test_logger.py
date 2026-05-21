import os
from src.core.logger import Logger


def test_log_creates_file(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log(b"Hello", "RECEIVE", "12:00:00.000")
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
    log.log(b"Hello", "RECEIVE", "12:00:00.000")

    content = _read_log(tmp_path)
    assert "12:00:00.000" in content
    assert "HEX" in content
    assert "ASCII" in content
    assert "Hello" in content
    assert "48 65 6C 6C 6F" in content


def test_log_send_direction(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log(b"Hi", "SEND", "12:00:00.000")

    content = _read_log(tmp_path)
    assert "\u2192" in content


def test_log_receive_direction(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log(b"Hi", "RECEIVE", "12:00:00.000")

    content = _read_log(tmp_path)
    assert "\u2190" in content


def test_multiple_logs_same_file(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log(b"A", "RECEIVE", "12:00:00.000")
    log.log(b"B", "RECEIVE", "12:00:01.000")

    content = _read_log(tmp_path)
    assert content.count("12:00:00.000") == 1
    assert content.count("12:00:01.000") == 1


def test_hex_data_in_log(tmp_path):
    log = Logger(log_dir=str(tmp_path))
    log.log(b"\x00\xFF\xAB", "RECEIVE", "12:00:00.000")

    content = _read_log(tmp_path)
    assert "00 FF AB" in content
