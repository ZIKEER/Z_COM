"""ANSI 转义序列解析器"""


def escape_html(text):
    """HTML 转义处理"""
    return text.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;').replace('\n', '<br>')


class AnsiParser:
    """ANSI 转义序列解析器，将字节中的 ANSI 控制码转换为 HTML"""

    FG_COLORS = {
        30: '#000000', 31: '#C00000', 32: '#00C000', 33: '#C0C000',
        34: '#0000C0', 35: '#C000C0', 36: '#00C0C0', 37: '#C0C0C0',
        90: '#808080', 91: '#FF4040', 92: '#40FF40', 93: '#FFFF40',
        94: '#4040FF', 95: '#FF40FF', 96: '#40FFFF', 97: '#FFFFFF',
    }

    @staticmethod
    def bytes_to_html(data, to_ascii_func):
        """将原始字节中的 ANSI 转义序列转换为 HTML

        Args:
            data: 原始字节数据
            to_ascii_func: 字节转 ASCII 字符串的函数 (bytes) -> str
        """
        result = []
        i = 0
        current_css = {}
        span_open = False

        while i < len(data):
            if data[i] == 0x1B:
                if span_open:
                    result.append('</span>')
                    span_open = False
                if i + 1 < len(data) and data[i + 1] == 0x5B:
                    j = i + 2
                    while j < len(data) and (data[j] < 0x40 or data[j] > 0x7E):
                        j += 1
                    if j < len(data) and data[j] == 0x6D:
                        params_str = data[i + 2:j].decode('ascii', errors='replace')
                        css = AnsiParser._parse_sgr(params_str)
                        if css is None:
                            current_css = {}
                        elif css:
                            current_css.update(css)
                    i = j + 1
                else:
                    i += 1
            else:
                start = i
                while i < len(data) and data[i] != 0x1B:
                    i += 1
                text = to_ascii_func(data[start:i])
                text_html = escape_html(text)
                if not text_html:
                    continue
                if current_css and not span_open:
                    inline = '; '.join(f'{k}: {v}' for k, v in current_css.items())
                    result.append(f'<span style="{inline}">')
                    span_open = True
                result.append(text_html)

        if span_open:
            result.append('</span>')

        return ''.join(result)

    @staticmethod
    def _parse_sgr(params_str):
        """解析 SGR 参数（ANSI 颜色/样式控制码）"""
        styles = {}
        if not params_str:
            params = [0]
        else:
            params = [int(p) if p else 0 for p in params_str.split(';')]
        i = 0
        while i < len(params):
            p = params[i]
            if p == 0:
                return None
            elif p == 1:
                styles['bold'] = True
            elif p == 4:
                styles['underline'] = True
            elif 30 <= p <= 37:
                styles['color'] = AnsiParser.FG_COLORS.get(p)
            elif 90 <= p <= 97:
                styles['color'] = AnsiParser.FG_COLORS.get(p)
            elif 40 <= p <= 47:
                styles['background'] = AnsiParser.FG_COLORS.get(p - 10)
            elif 100 <= p <= 107:
                styles['background'] = AnsiParser.FG_COLORS.get(p - 10)
            elif p == 38 and i + 2 < len(params):
                if params[i + 1] == 5:
                    i += 2
                elif params[i + 1] == 2 and i + 4 < len(params):
                    r, g, b = params[i + 2], params[i + 3], params[i + 4]
                    styles['color'] = f'#{r:02X}{g:02X}{b:02X}'
                    i += 4
            elif p == 48 and i + 2 < len(params):
                if params[i + 1] == 5:
                    i += 2
                elif params[i + 1] == 2 and i + 4 < len(params):
                    r, g, b = params[i + 2], params[i + 3], params[i + 4]
                    styles['background'] = f'#{r:02X}{g:02X}{b:02X}'
                    i += 4
            i += 1
        return styles if styles else {}
