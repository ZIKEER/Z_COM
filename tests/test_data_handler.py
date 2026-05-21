from src.core.data_handler import DataHandler

dh = DataHandler()


class TestBytesToHex:
    def test_empty(self):
        assert dh.bytes_to_hex(b"") == ""

    def test_single_byte(self):
        assert dh.bytes_to_hex(b"\xAB") == "AB"

    def test_multiple_bytes(self):
        assert dh.bytes_to_hex(b"Hello") == "48 65 6C 6C 6F"

    def test_all_zero(self):
        assert dh.bytes_to_hex(b"\x00\x00") == "00 00"


class TestBytesToAscii:
    def test_printable(self):
        assert dh.bytes_to_ascii(b"Hello") == "Hello"

    def test_control_chars(self):
        result = dh.bytes_to_ascii(b"\x00\x01\x7F")
        assert result == "\u2400\u2401\u2421"

    def test_newline(self):
        assert dh.bytes_to_ascii(b"\n") == "\u240a\n"

    def test_carriage_return(self):
        assert dh.bytes_to_ascii(b"\r") == "\u240d"

    def test_high_bytes(self):
        assert dh.bytes_to_ascii(b"\x80\xFF") == "\\x80\\xff"

    def test_combined(self):
        result = dh.bytes_to_ascii(b"H\x00e\x0Alo\r")
        assert "\u2400" in result
        assert "\u240a\n" in result
        assert "\u240d" in result
        assert "H" in result
        assert "e" in result
        assert "lo" in result


class TestFormatDisplay:
    def test_hex_mode(self):
        assert dh.format_display(b"\x48\x65", "HEX") == "48 65"

    def test_ascii_mode(self):
        assert dh.format_display(b"Hi", "ASCII") == "Hi"

    def test_mixed_mode(self):
        result = dh.format_display(b"Hi", "MIXED")
        assert "HEX" in result
        assert "ASCII" in result
        assert "48 69" in result
        assert "Hi" in result

    def test_unknown_mode(self):
        assert dh.format_display(b"Hi", "UNKNOWN") == ""


class TestValidateHexInput:
    def test_valid_hex(self):
        assert dh.validate_hex_input("48 65 6C") is True

    def test_valid_no_spaces(self):
        assert dh.validate_hex_input("48656C") is True

    def test_invalid_hex(self):
        assert dh.validate_hex_input("XYZ") is False

    def test_empty(self):
        assert dh.validate_hex_input("") is True
