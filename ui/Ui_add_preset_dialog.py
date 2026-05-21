# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'add_preset_dialog.ui'
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
    QHBoxLayout, QLabel, QLineEdit, QRadioButton,
    QSizePolicy, QSpacerItem, QVBoxLayout, QWidget)

class Ui_AddPresetDialog(object):
    def setupUi(self, AddPresetDialog):
        if not AddPresetDialog.objectName():
            AddPresetDialog.setObjectName(u"AddPresetDialog")
        AddPresetDialog.resize(400, 200)
        self.verticalLayout = QVBoxLayout(AddPresetDialog)
        self.verticalLayout.setSpacing(12)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.verticalLayout.setContentsMargins(12, 12, 12, 12)
        self.nameLayout = QHBoxLayout()
        self.nameLayout.setObjectName(u"nameLayout")
        self.nameLabel = QLabel(AddPresetDialog)
        self.nameLabel.setObjectName(u"nameLabel")
        self.nameLabel.setMinimumSize(QSize(60, 0))

        self.nameLayout.addWidget(self.nameLabel)

        self.nameEdit = QLineEdit(AddPresetDialog)
        self.nameEdit.setObjectName(u"nameEdit")

        self.nameLayout.addWidget(self.nameEdit)


        self.verticalLayout.addLayout(self.nameLayout)

        self.commandLayout = QHBoxLayout()
        self.commandLayout.setObjectName(u"commandLayout")
        self.commandLabel = QLabel(AddPresetDialog)
        self.commandLabel.setObjectName(u"commandLabel")
        self.commandLabel.setMinimumSize(QSize(60, 0))

        self.commandLayout.addWidget(self.commandLabel)

        self.commandEdit = QLineEdit(AddPresetDialog)
        self.commandEdit.setObjectName(u"commandEdit")

        self.commandLayout.addWidget(self.commandEdit)


        self.verticalLayout.addLayout(self.commandLayout)

        self.formatLayout = QHBoxLayout()
        self.formatLayout.setObjectName(u"formatLayout")
        self.formatLabel = QLabel(AddPresetDialog)
        self.formatLabel.setObjectName(u"formatLabel")
        self.formatLabel.setMinimumSize(QSize(60, 0))

        self.formatLayout.addWidget(self.formatLabel)

        self.asciiRadio = QRadioButton(AddPresetDialog)
        self.asciiRadio.setObjectName(u"asciiRadio")
        self.asciiRadio.setChecked(True)

        self.formatLayout.addWidget(self.asciiRadio)

        self.hexRadio = QRadioButton(AddPresetDialog)
        self.hexRadio.setObjectName(u"hexRadio")

        self.formatLayout.addWidget(self.hexRadio)

        self.formatSpacer = QSpacerItem(40, 20, QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Minimum)

        self.formatLayout.addItem(self.formatSpacer)


        self.verticalLayout.addLayout(self.formatLayout)

        self.buttonBox = QDialogButtonBox(AddPresetDialog)
        self.buttonBox.setObjectName(u"buttonBox")
        self.buttonBox.setStandardButtons(QDialogButtonBox.StandardButton.Cancel|QDialogButtonBox.StandardButton.Ok)

        self.verticalLayout.addWidget(self.buttonBox)


        self.retranslateUi(AddPresetDialog)

        QMetaObject.connectSlotsByName(AddPresetDialog)
    # setupUi

    def retranslateUi(self, AddPresetDialog):
        AddPresetDialog.setWindowTitle(QCoreApplication.translate("AddPresetDialog", u"\u6dfb\u52a0\u9884\u8bbe\u547d\u4ee4", None))
        self.nameLabel.setText(QCoreApplication.translate("AddPresetDialog", u"\u540d\u79f0\uff1a", None))
        self.nameEdit.setPlaceholderText(QCoreApplication.translate("AddPresetDialog", u"\u8f93\u5165\u547d\u4ee4\u540d\u79f0\uff08\u53ef\u9009\uff09", None))
        self.commandLabel.setText(QCoreApplication.translate("AddPresetDialog", u"\u547d\u4ee4\uff1a", None))
        self.commandEdit.setPlaceholderText(QCoreApplication.translate("AddPresetDialog", u"\u8f93\u5165\u8981\u53d1\u9001\u7684\u547d\u4ee4\u5185\u5bb9", None))
        self.formatLabel.setText(QCoreApplication.translate("AddPresetDialog", u"\u683c\u5f0f\uff1a", None))
        self.asciiRadio.setText(QCoreApplication.translate("AddPresetDialog", u"ASCII", None))
        self.hexRadio.setText(QCoreApplication.translate("AddPresetDialog", u"HEX", None))
    # retranslateUi

