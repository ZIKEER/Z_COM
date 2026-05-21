from src.core.ansi_parser import AnsiParser, escape_html

parser = AnsiParser()


def dummy_to_ascii(data):
    return "".join(chr(b) if 0x20 <= b < 0x7F else f"\\x{b:02x}" for b in data)


def ascii_only(data):
    return "".join(chr(b) for b in data)


class TestEscapeHtml:
    def test_no_escape(self):
        assert escape_html("hello") == "hello"

    def test_escape_ampersand(self):
        assert escape_html("a&b") == "a&amp;b"

    def test_escape_lt(self):
        assert escape_html("a<b") == "a&lt;b"

    def test_escape_gt(self):
        assert escape_html("a>b") == "a&gt;b"

    def test_newline_to_br(self):
        assert escape_html("a\nb") == "a<br>b"

    def test_all_escapes(self):
        assert escape_html("<&>") == "&lt;&amp;&gt;"


class TestBytesToHtml:
    def test_no_ansi(self):
        html = parser.bytes_to_html(b"Hello", dummy_to_ascii)
        assert html == "Hello"

    def test_ansi_reset(self):
        html = parser.bytes_to_html(b"\x1B[0mHello", dummy_to_ascii)
        assert html == "Hello"

    def test_red_foreground(self):
        html = parser.bytes_to_html(b"\x1B[31mRed\x1B[0m", dummy_to_ascii)
        assert "color: #C00000" in html
        assert "Red" in html
        assert "</span>" in html

    def test_green_foreground(self):
        html = parser.bytes_to_html(b"\x1B[32mGreen", dummy_to_ascii)
        assert "color: #00C000" in html
        assert "Green" in html

    def test_bold(self):
        html = parser.bytes_to_html(b"\x1B[1mBold", dummy_to_ascii)
        assert "bold" in html

    def test_underline(self):
        html = parser.bytes_to_html(b"\x1B[4mUnder", dummy_to_ascii)
        assert "underline" in html

    def test_bright_colors(self):
        html = parser.bytes_to_html(b"\x1B[91mBright", dummy_to_ascii)
        assert "color: #FF4040" in html

    def test_256_colors(self):
        html = parser.bytes_to_html(b"\x1B[38;5;82mColor256", dummy_to_ascii)
        assert "Color256" in html

    def test_truecolor(self):
        html = parser.bytes_to_html(b"\x1B[38;2;255;128;0mOrange", dummy_to_ascii)
        assert "#FF8000" in html
        assert "Orange" in html

    def test_background_color(self):
        html = parser.bytes_to_html(b"\x1B[41mBgRed", dummy_to_ascii)
        assert "background: #C00000" in html

    def test_multiple_sequences(self):
        html = parser.bytes_to_html(b"\x1B[31mRed\x1B[32mGreen", dummy_to_ascii)
        assert "Red" in html
        assert "Green" in html

    def test_mixed_plain_and_ansi(self):
        html = parser.bytes_to_html(b"Plain\x1B[31mRed\x1B[0mPlain", dummy_to_ascii)
        assert html.count("Plain") == 2
        assert "Red" in html

    def test_html_special_chars_escaped(self):
        html = parser.bytes_to_html(b"<hello>", dummy_to_ascii)
        assert "&lt;" in html
        assert "&gt;" in html


class TestParseSgr:
    def test_reset(self):
        assert AnsiParser._parse_sgr("0") is None

    def test_empty(self):
        # 空字符串等价于 0 → reset → None
        result = AnsiParser._parse_sgr("")
        assert result is None

    def test_red_fg(self):
        result = AnsiParser._parse_sgr("31")
        assert result.get("color") == "#C00000"

    def test_red_bg(self):
        result = AnsiParser._parse_sgr("41")
        assert result.get("background") == "#C00000"

    def test_combined(self):
        result = AnsiParser._parse_sgr("1;31")
        assert result.get("bold") is True
        assert result.get("color") == "#C00000"
