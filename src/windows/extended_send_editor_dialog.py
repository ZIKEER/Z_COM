import os
import sys

from PySide6.QtGui import QFont
from PySide6.QtWidgets import QDialog

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from ui.Ui_extended_send_editor_dialog import Ui_ExtendedSendEditorDialog


class ExtendedSendEditorDialog(QDialog):
    def __init__(self, text="", parent=None):
        super().__init__(parent)
        self.ui = Ui_ExtendedSendEditorDialog()
        self.ui.setupUi(self)

        mono_font = QFont("Consolas", 10)
        mono_font.setStyleHint(QFont.StyleHint.Monospace)
        self.ui.dataPlainTextEdit.setFont(mono_font)
        self.ui.dataPlainTextEdit.setPlainText(text)

        self.ui.buttonBox.accepted.connect(self.accept)
        self.ui.buttonBox.rejected.connect(self.reject)

    def get_text(self):
        return self.ui.dataPlainTextEdit.toPlainText()
