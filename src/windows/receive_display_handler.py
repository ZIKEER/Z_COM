from datetime import datetime
from PySide6.QtCore import QTimer
from PySide6.QtGui import QTextCursor, QFont, QAction
from PySide6.QtCore import Qt

from src.core.ansi_parser import escape_html
from src.windows.status_bar_controller import (
    DISPLAY_TIMESTAMP_COLOR, DISPLAY_ARROW_COLOR, DISPLAY_DATA_COLOR,
    MAX_DISPLAY_LINES, DISPLAY_PRUNE_LINES,
)


class ReceiveDisplayHandler:
    """接收区数据显示管理：批量拼包、格式化、裁剪、ANSI 处理。"""

    def __init__(self, receive_text_edit, data_handler, ansi_parser, logger,
                 get_display_mode, get_display_ansi):
        self._text_edit = receive_text_edit
        self._data_handler = data_handler
        self._ansi_parser = ansi_parser
        self._logger = logger
        self._get_display_mode = get_display_mode
        self._get_display_ansi = get_display_ansi

        self.receive_count = 0
        self._pending_data = bytearray()
        self._append_count = 0

        mono_font = QFont("Consolas", 10)
        mono_font.setStyleHint(QFont.StyleHint.Monospace)
        self._text_edit.setFont(mono_font)
        self._text_edit.setUndoRedoEnabled(False)

        self._flush_timer = QTimer()
        self._flush_timer.setSingleShot(True)
        self._flush_timer.setInterval(50)
        self._flush_timer.timeout.connect(self._flush_pending)

    def set_batch_window(self, ms):
        self._flush_timer.setInterval(max(int(ms), 1))

    def on_data_received(self, data):
        if not data:
            return
        self._pending_data.extend(data)
        self._flush_timer.start()

    def _flush_pending(self):
        if not self._pending_data:
            return
        data = bytes(self._pending_data)
        tail = self._find_incomplete_ansi_tail(data)
        if tail > 0:
            self._pending_data = bytearray(data[-tail:])
            data = data[:-tail]
        else:
            self._pending_data.clear()
        if data:
            self.append_data(data, '←', 'RECEIVE')

    _EVENT_COLORS = {
        'green': '#4CAF50',
        'red': '#F44336',
        'orange': '#FF9800',
    }

    def append_data(self, data, arrow, log_type, client_prefix=None):
        mode = self._get_display_mode()
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
        display_arrow = (client_prefix + arrow) if client_prefix else arrow

        if log_type == 'RECEIVE':
            self.receive_count += len(data)
        # send_count 由调用方管理

        hex_str = self._data_handler.bytes_to_hex(data)
        ascii_str = self._data_handler.bytes_to_ascii(data)
        self._logger.log(timestamp, log_type, hex_str, ascii_str)

        html = self._format_display(data, mode, timestamp, display_arrow)
        self._text_edit.append(html)

    def append_event(self, text, color='orange'):
        ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
        c = self._EVENT_COLORS.get(color, color)
        self._text_edit.append(f'<span style="color:{c};">[{ts}] {escape_html(text)}</span>')
        self._logger.log_event(text)

        if self._text_edit.parent():
            # auto-scroll 由 MainWindow 控制
            pass

        self._append_count += 1
        if self._append_count >= 50:
            self._append_count = 0
            self._prune_if_needed()

    def _format_display(self, data, mode, timestamp, arrow):
        hex_str = self._data_handler.bytes_to_hex(data)
        display_ansi = self._get_display_ansi()

        if display_ansi and mode != 'HEX':
            ascii_colored = self._ansi_parser.bytes_to_html(data, self._data_handler.bytes_to_ascii)
        else:
            ascii_colored = escape_html(self._data_handler.bytes_to_ascii(data))

        ts_tag = f'<span style="color:{DISPLAY_TIMESTAMP_COLOR};">[{timestamp}]</span>'
        arrow_tag = f'<span style="color:{DISPLAY_ARROW_COLOR}; font-weight:bold;">{arrow}</span>'
        data_tag = lambda text: f'<span style="color:{DISPLAY_DATA_COLOR};">{text}</span>'

        lines = []
        if mode in ('HEX', 'MIXED'):
            lines.append(f'{arrow_tag} HEX: {data_tag(hex_str)}')
        if mode in ('ASCII', 'MIXED'):
            lines.append(f'{arrow_tag} ASCII: {data_tag(ascii_colored)}')

        if mode == 'MIXED':
            return f'{ts_tag}<br>' + '<br>'.join(lines)
        return f'{ts_tag} ' + lines[0]

    def _prune_if_needed(self):
        doc = self._text_edit.document()
        if doc.blockCount() < MAX_DISPLAY_LINES:
            return
        prune_block = doc.findBlockByNumber(DISPLAY_PRUNE_LINES)
        if not prune_block.isValid():
            return
        cursor = QTextCursor(doc)
        cursor.setPosition(0)
        cursor.setPosition(prune_block.position(), QTextCursor.MoveMode.KeepAnchor)
        cursor.removeSelectedText()

    def clear(self):
        self._pending_data.clear()
        self._flush_timer.stop()
        self._text_edit.clear()
        self.receive_count = 0

    def flush(self):
        self._flush_pending()

    def setup_context_menu(self, toggle_ansi_callback):
        self._text_edit.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self._text_edit.customContextMenuRequested.connect(
            lambda pos: self._show_context_menu(pos, toggle_ansi_callback)
        )

    def _show_context_menu(self, pos, toggle_ansi_callback):
        menu = self._text_edit.createStandardContextMenu()
        menu.addSeparator()
        ansi_action = QAction("ANSI颜色显示", menu)
        ansi_action.setCheckable(True)
        ansi_action.setChecked(self._get_display_ansi())
        ansi_action.toggled.connect(toggle_ansi_callback)
        menu.addAction(ansi_action)
        menu.exec_(self._text_edit.mapToGlobal(pos))

    @staticmethod
    def _find_incomplete_ansi_tail(data):
        i = len(data) - 1
        while i >= 0:
            if data[i] == 0x1B:
                break
            if 0x40 <= data[i] <= 0x7E:
                return 0
            i -= 1
        if i < 0:
            return 0
        if i + 1 < len(data) and data[i + 1] == 0x5B:
            for j in range(i + 2, len(data)):
                if 0x40 <= data[j] <= 0x7E:
                    return 0
            return len(data) - i
        return 0
