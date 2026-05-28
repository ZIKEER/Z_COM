from PySide6.QtWidgets import QLabel


# 显示颜色常量
DISPLAY_TIMESTAMP_COLOR = "#00CED1"
DISPLAY_ARROW_COLOR = "#000000"
DISPLAY_DATA_COLOR = "#000000"
MAX_DISPLAY_LINES = 5000
DISPLAY_PRUNE_LINES = 2500


class StatusBarController:
    """管理状态栏标签和字节计数。"""

    def __init__(self, status_bar):
        self._status_bar = status_bar
        self._status_bar.setStyleSheet("QStatusBar::item{border:0}")
        self._status_bar.show()

        self.status_label = QLabel("已断开")
        self.send_count_label = QLabel("发送: 0 字节")
        self.receive_count_label = QLabel("接收: 0 字节")
        sep1 = QLabel("|")
        sep2 = QLabel("|")

        self._status_bar.addPermanentWidget(self.status_label)
        self._status_bar.addPermanentWidget(sep1)
        self._status_bar.addPermanentWidget(self.send_count_label)
        self._status_bar.addPermanentWidget(sep2)
        self._status_bar.addPermanentWidget(self.receive_count_label)

        self.status_label.setStyleSheet("color: red;")

    def update_counts(self, send_count, receive_count):
        self.send_count_label.setText(f"发送: {send_count} 字节")
        self.receive_count_label.setText(f"接收: {receive_count} 字节")

    def set_connected(self, text):
        self.status_label.setText(text)
        self.status_label.setStyleSheet("color: green;")

    def set_disconnected(self):
        self.status_label.setText("已断开")
        self.status_label.setStyleSheet("color: red;")
