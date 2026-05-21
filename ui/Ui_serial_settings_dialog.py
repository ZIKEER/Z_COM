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
from PySide6.QtWidgets import (QAbstractButton, QApplication, QCheckBox, QComboBox,
    QDialog, QDialogButtonBox, QGridLayout, QGroupBox,
    QHBoxLayout, QLabel, QLineEdit, QSizePolicy,
    QSpacerItem, QSpinBox, QVBoxLayout, QWidget)

class Ui_SerialSettingsDialog(object):
    def setupUi(self, SerialSettingsDialog):
        if not SerialSettingsDialog.objectName():
            SerialSettingsDialog.setObjectName(u"SerialSettingsDialog")
        SerialSettingsDialog.resize(380, 450)
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

        self.frameTimeoutLayout = QHBoxLayout()
        self.frameTimeoutLayout.setObjectName(u"frameTimeoutLayout")
        self.frameTimeoutLabel = QLabel(SerialSettingsDialog)
        self.frameTimeoutLabel.setObjectName(u"frameTimeoutLabel")

        self.frameTimeoutLayout.addWidget(self.frameTimeoutLabel)

        self.frameTimeoutSpinBox = QSpinBox(SerialSettingsDialog)
        self.frameTimeoutSpinBox.setObjectName(u"frameTimeoutSpinBox")
        self.frameTimeoutSpinBox.setMinimum(10)
        self.frameTimeoutSpinBox.setMaximum(1000)
        self.frameTimeoutSpinBox.setValue(50)

        self.frameTimeoutLayout.addWidget(self.frameTimeoutSpinBox)

        self.frameTimeoutSpacer = QSpacerItem(40, 20, QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Minimum)

        self.frameTimeoutLayout.addItem(self.frameTimeoutSpacer)


        self.verticalLayout.addLayout(self.frameTimeoutLayout)

        self.frameTimeoutHint = QLabel(SerialSettingsDialog)
        self.frameTimeoutHint.setObjectName(u"frameTimeoutHint")
        self.frameTimeoutHint.setWordWrap(True)

        self.verticalLayout.addWidget(self.frameTimeoutHint)

        self.rttGroupBox = QGroupBox(SerialSettingsDialog)
        self.rttGroupBox.setObjectName(u"rttGroupBox")
        self.rttVBoxLayout = QVBoxLayout(self.rttGroupBox)
        self.rttVBoxLayout.setSpacing(6)
        self.rttVBoxLayout.setObjectName(u"rttVBoxLayout")
        self.rttGridLayout = QGridLayout()
        self.rttGridLayout.setSpacing(6)
        self.rttGridLayout.setObjectName(u"rttGridLayout")
        self.rttChipLabel = QLabel(self.rttGroupBox)
        self.rttChipLabel.setObjectName(u"rttChipLabel")

        self.rttGridLayout.addWidget(self.rttChipLabel, 0, 0, 1, 1)

        self.rttChipComboBox = QComboBox(self.rttGroupBox)
        self.rttChipComboBox.setObjectName(u"rttChipComboBox")
        sizePolicy = QSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.rttChipComboBox.sizePolicy().hasHeightForWidth())
        self.rttChipComboBox.setSizePolicy(sizePolicy)
        self.rttChipComboBox.setEditable(True)

        self.rttGridLayout.addWidget(self.rttChipComboBox, 0, 1, 1, 1)

        self.rttSpeedLabel = QLabel(self.rttGroupBox)
        self.rttSpeedLabel.setObjectName(u"rttSpeedLabel")

        self.rttGridLayout.addWidget(self.rttSpeedLabel, 1, 0, 1, 1)

        self.rttSpeedLayout = QHBoxLayout()
        self.rttSpeedLayout.setObjectName(u"rttSpeedLayout")
        self.rttSpeedSpinBox = QSpinBox(self.rttGroupBox)
        self.rttSpeedSpinBox.setObjectName(u"rttSpeedSpinBox")
        self.rttSpeedSpinBox.setMinimum(100)
        self.rttSpeedSpinBox.setMaximum(50000)
        self.rttSpeedSpinBox.setValue(4000)

        self.rttSpeedLayout.addWidget(self.rttSpeedSpinBox)

        self.rttResetCheckBox = QCheckBox(self.rttGroupBox)
        self.rttResetCheckBox.setObjectName(u"rttResetCheckBox")
        self.rttResetCheckBox.setChecked(True)

        self.rttSpeedLayout.addWidget(self.rttResetCheckBox)


        self.rttGridLayout.addLayout(self.rttSpeedLayout, 1, 1, 1, 1)


        self.rttVBoxLayout.addLayout(self.rttGridLayout)

        self.rttAddressHint = QLabel(self.rttGroupBox)
        self.rttAddressHint.setObjectName(u"rttAddressHint")

        self.rttVBoxLayout.addWidget(self.rttAddressHint)

        self.rttAddressLayout = QHBoxLayout()
        self.rttAddressLayout.setObjectName(u"rttAddressLayout")
        self.rttStartAddressLineEdit = QLineEdit(self.rttGroupBox)
        self.rttStartAddressLineEdit.setObjectName(u"rttStartAddressLineEdit")

        self.rttAddressLayout.addWidget(self.rttStartAddressLineEdit)

        self.rttRangeSizeLineEdit = QLineEdit(self.rttGroupBox)
        self.rttRangeSizeLineEdit.setObjectName(u"rttRangeSizeLineEdit")

        self.rttAddressLayout.addWidget(self.rttRangeSizeLineEdit)


        self.rttVBoxLayout.addLayout(self.rttAddressLayout)


        self.verticalLayout.addWidget(self.rttGroupBox)

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
        self.frameTimeoutLabel.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u62fc\u5305\u8d85\u65f6\uff1a", None))
        self.frameTimeoutSpinBox.setSuffix(QCoreApplication.translate("SerialSettingsDialog", u" ms", None))
        self.frameTimeoutHint.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u8bf4\u660e\uff1a\u62fc\u5305\u8d85\u65f6\u7528\u4e8e\u5c06\u540c\u4e00\u5e27\u6570\u636e\u5408\u5e76\u663e\u793a\uff0c\u8d85\u65f6\u65f6\u95f4\u8d8a\u957f\u5408\u5e76\u8d8a\u591a", None))
        self.rttGroupBox.setTitle(QCoreApplication.translate("SerialSettingsDialog", u"RTT \u8bbe\u5907\u914d\u7f6e (J-Link)", None))
        self.rttChipLabel.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u82af\u7247\u578b\u53f7\uff1a", None))
        self.rttSpeedLabel.setText(QCoreApplication.translate("SerialSettingsDialog", u"J-Link \u901f\u5ea6\uff1a", None))
        self.rttSpeedSpinBox.setSuffix(QCoreApplication.translate("SerialSettingsDialog", u" kHz", None))
        self.rttResetCheckBox.setText(QCoreApplication.translate("SerialSettingsDialog", u"\u8fde\u63a5\u65f6\u590d\u4f4d", None))
        self.rttAddressHint.setText(QCoreApplication.translate("SerialSettingsDialog", u"RTT \u5730\u5740\u641c\u7d22\u8303\u56f4\uff08\u53ef\u9009\uff0c\u683c\u5f0f\uff1a0x20000000 0x4000\uff09\uff1a", None))
        self.rttStartAddressLineEdit.setPlaceholderText(QCoreApplication.translate("SerialSettingsDialog", u"\u8d77\u59cb\u5730\u5740\uff08\u5982 0x20000000\uff09", None))
        self.rttRangeSizeLineEdit.setPlaceholderText(QCoreApplication.translate("SerialSettingsDialog", u"\u8303\u56f4\u5927\u5c0f\uff08\u5982 0x4000\uff09", None))
    # retranslateUi

