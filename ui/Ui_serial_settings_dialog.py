# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'serial_settings_dialog.ui'
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
from PySide6.QtWidgets import (QAbstractButton, QApplication, QComboBox, QDialog,
    QDialogButtonBox, QGridLayout, QLabel, QSizePolicy,
    QVBoxLayout, QWidget)

class Ui_SerialSettingsDialog(object):
    def setupUi(self, SerialSettingsDialog):
        if not SerialSettingsDialog.objectName():
            SerialSettingsDialog.setObjectName(u"SerialSettingsDialog")
        SerialSettingsDialog.resize(280, 200)
        self.verticalLayout = QVBoxLayout(SerialSettingsDialog)
        self.verticalLayout.setSpacing(8)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.verticalLayout.setContentsMargins(8, 8, 8, 8)
        self.gridLayout = QGridLayout()
        self.gridLayout.setSpacing(6)
        self.gridLayout.setObjectName(u"gridLayout")
        self.databitsLabel = QLabel(SerialSettingsDialog)
        self.databitsLabel.setObjectName(u"databitsLabel")

        self.gridLayout.addWidget(self.databitsLabel, 0, 0, 1, 1)

        self.databitsComboBox = QComboBox(SerialSettingsDialog)
        self.databitsComboBox.setObjectName(u"databitsComboBox")

        self.gridLayout.addWidget(self.databitsComboBox, 0, 1, 1, 1)

        self.stopbitsLabel = QLabel(SerialSettingsDialog)
        self.stopbitsLabel.setObjectName(u"stopbitsLabel")

        self.gridLayout.addWidget(self.stopbitsLabel, 0, 2, 1, 1)

        self.stopbitsComboBox = QComboBox(SerialSettingsDialog)
        self.stopbitsComboBox.setObjectName(u"stopbitsComboBox")

        self.gridLayout.addWidget(self.stopbitsComboBox, 0, 3, 1, 1)

        self.parityLabel = QLabel(SerialSettingsDialog)
        self.parityLabel.setObjectName(u"parityLabel")

        self.gridLayout.addWidget(self.parityLabel, 1, 0, 1, 1)

        self.parityComboBox = QComboBox(SerialSettingsDialog)
        self.parityComboBox.setObjectName(u"parityComboBox")

        self.gridLayout.addWidget(self.parityComboBox, 1, 1, 1, 1)

        self.flowcontrolLabel = QLabel(SerialSettingsDialog)
        self.flowcontrolLabel.setObjectName(u"flowcontrolLabel")

        self.gridLayout.addWidget(self.flowcontrolLabel, 1, 2, 1, 1)

        self.flowcontrolComboBox = QComboBox(SerialSettingsDialog)
        self.flowcontrolComboBox.setObjectName(u"flowcontrolComboBox")

        self.gridLayout.addWidget(self.flowcontrolComboBox, 1, 3, 1, 1)


        self.verticalLayout.addLayout(self.gridLayout)

        self.buttonBox = QDialogButtonBox(SerialSettingsDialog)
        self.buttonBox.setObjectName(u"buttonBox")
        self.buttonBox.setStandardButtons(QDialogButtonBox.Cancel|QDialogButtonBox.Ok)

        self.verticalLayout.addWidget(self.buttonBox)


        self.retranslateUi(SerialSettingsDialog)

        QMetaObject.connectSlotsByName(SerialSettingsDialog)
    # setupUi

    def retranslateUi(self, SerialSettingsDialog):
        SerialSettingsDialog.setWindowTitle(QCoreApplication.translate("SerialSettingsDialog", u"\u4e32\u53e3\u9ad8\u7ea7\u8bbe\u7f6e", None))
        self.databitsLabel.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u6570\u636e\u4f4d\uff1a", None))
        self.stopbitsLabel.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u505c\u6b62\u4f4d\uff1a", None))
        self.parityLabel.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u6821\u9a8c\u4f4d\uff1a", None))
        self.flowcontrolLabel.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u6d41\u63a7\u5236\uff1a", None))
    # retranslateUi

