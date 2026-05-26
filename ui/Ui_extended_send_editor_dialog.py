# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'extended_send_editor_dialog.ui'
##
## Created by: Qt User Interface Compiler version 6.11.0
##
## WARNING! All changes made in this file will be lost when recompiling UI file!
################################################################################

from PySide6.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
    QMetaObject, QObject, QPoint, QRect,
    QSize, QTime, QUrl, Qt)
from PySide6.QtGui import (QBrush, QColor, QConicalGradient, QCursor,
    QFont, QFontDatabase, QGradient, QIcon,
    QImage, QKeySequence, QLinearGradient, QPainter,
    QPalette, QPixmap, QRadialGradient, QTransform)
from PySide6.QtWidgets import (QAbstractButton, QApplication, QDialog, QDialogButtonBox,
    QLabel, QPlainTextEdit, QSizePolicy, QVBoxLayout,
    QWidget)

class Ui_ExtendedSendEditorDialog(object):
    def setupUi(self, ExtendedSendEditorDialog):
        if not ExtendedSendEditorDialog.objectName():
            ExtendedSendEditorDialog.setObjectName(u"ExtendedSendEditorDialog")
        ExtendedSendEditorDialog.resize(560, 360)
        self.verticalLayout = QVBoxLayout(ExtendedSendEditorDialog)
        self.verticalLayout.setSpacing(6)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.verticalLayout.setContentsMargins(8, 8, 8, 8)
        self.hintLabel = QLabel(ExtendedSendEditorDialog)
        self.hintLabel.setObjectName(u"hintLabel")
        self.hintLabel.setWordWrap(True)

        self.verticalLayout.addWidget(self.hintLabel)

        self.dataPlainTextEdit = QPlainTextEdit(ExtendedSendEditorDialog)
        self.dataPlainTextEdit.setObjectName(u"dataPlainTextEdit")

        self.verticalLayout.addWidget(self.dataPlainTextEdit)

        self.buttonBox = QDialogButtonBox(ExtendedSendEditorDialog)
        self.buttonBox.setObjectName(u"buttonBox")
        self.buttonBox.setStandardButtons(QDialogButtonBox.Cancel|QDialogButtonBox.Ok)

        self.verticalLayout.addWidget(self.buttonBox)


        self.retranslateUi(ExtendedSendEditorDialog)

        QMetaObject.connectSlotsByName(ExtendedSendEditorDialog)
    # setupUi

    def retranslateUi(self, ExtendedSendEditorDialog):
        ExtendedSendEditorDialog.setWindowTitle(QCoreApplication.translate("ExtendedSendEditorDialog", u"\u9ad8\u7ea7\u7f16\u8f91\u53d1\u9001\u5185\u5bb9", None))
        self.hintLabel.setText(QCoreApplication.translate("ExtendedSendEditorDialog", u"\u652f\u6301\u76f4\u63a5\u8f93\u5165\u591a\u884c\u548c Tab\u3002\u82e5\u9700\u8981\u56de\u8f66\u5b57\u7b26 CR\uff0c\u8bf7\u5728\u5355\u884c\u8f93\u5165\u6846\u4e2d\u4f7f\u7528 \\r\u3002", None))
    # retranslateUi

