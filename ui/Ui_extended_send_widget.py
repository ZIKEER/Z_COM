# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'extended_send_widget.ui'
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
from PySide6.QtWidgets import (QApplication, QHBoxLayout, QHeaderView, QLabel,
    QPushButton, QSizePolicy, QSpacerItem, QTableWidget,
    QTableWidgetItem, QVBoxLayout, QWidget)

class Ui_ExtendedSendWidget(object):
    def setupUi(self, ExtendedSendWidget):
        if not ExtendedSendWidget.objectName():
            ExtendedSendWidget.setObjectName(u"ExtendedSendWidget")
        ExtendedSendWidget.resize(500, 400)
        self.verticalLayout = QVBoxLayout(ExtendedSendWidget)
        self.verticalLayout.setSpacing(6)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.verticalLayout.setContentsMargins(6, 6, 6, 6)
        self.dataTable = QTableWidget(ExtendedSendWidget)
        if (self.dataTable.columnCount() < 4):
            self.dataTable.setColumnCount(4)
        __qtablewidgetitem = QTableWidgetItem()
        self.dataTable.setHorizontalHeaderItem(0, __qtablewidgetitem)
        __qtablewidgetitem1 = QTableWidgetItem()
        self.dataTable.setHorizontalHeaderItem(1, __qtablewidgetitem1)
        __qtablewidgetitem2 = QTableWidgetItem()
        self.dataTable.setHorizontalHeaderItem(2, __qtablewidgetitem2)
        __qtablewidgetitem3 = QTableWidgetItem()
        self.dataTable.setHorizontalHeaderItem(3, __qtablewidgetitem3)
        self.dataTable.setObjectName(u"dataTable")
        self.dataTable.setColumnCount(4)

        self.verticalLayout.addWidget(self.dataTable)

        self.controlLayout = QHBoxLayout()
        self.controlLayout.setObjectName(u"controlLayout")
        self.addButton = QPushButton(ExtendedSendWidget)
        self.addButton.setObjectName(u"addButton")

        self.controlLayout.addWidget(self.addButton)

        self.deleteButton = QPushButton(ExtendedSendWidget)
        self.deleteButton.setObjectName(u"deleteButton")

        self.controlLayout.addWidget(self.deleteButton)

        self.moveUpButton = QPushButton(ExtendedSendWidget)
        self.moveUpButton.setObjectName(u"moveUpButton")

        self.controlLayout.addWidget(self.moveUpButton)

        self.moveDownButton = QPushButton(ExtendedSendWidget)
        self.moveDownButton.setObjectName(u"moveDownButton")

        self.controlLayout.addWidget(self.moveDownButton)

        self.actionSpacer = QSpacerItem(40, 20, QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Minimum)

        self.controlLayout.addItem(self.actionSpacer)

        self.loopSendButton = QPushButton(ExtendedSendWidget)
        self.loopSendButton.setObjectName(u"loopSendButton")
        self.loopSendButton.setCheckable(True)

        self.controlLayout.addWidget(self.loopSendButton)

        self.helpLabel = QLabel(ExtendedSendWidget)
        self.helpLabel.setObjectName(u"helpLabel")

        self.controlLayout.addWidget(self.helpLabel)


        self.verticalLayout.addLayout(self.controlLayout)


        self.retranslateUi(ExtendedSendWidget)

        QMetaObject.connectSlotsByName(ExtendedSendWidget)
    # setupUi

    def retranslateUi(self, ExtendedSendWidget):
        ExtendedSendWidget.setWindowTitle(QCoreApplication.translate("ExtendedSendWidget", u"\u6269\u5c55\u53d1\u9001", None))
        ___qtablewidgetitem = self.dataTable.horizontalHeaderItem(0)
        ___qtablewidgetitem.setText(QCoreApplication.translate("ExtendedSendWidget", u"HEX", None))
        ___qtablewidgetitem1 = self.dataTable.horizontalHeaderItem(1)
        ___qtablewidgetitem1.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u6570\u636e\u5185\u5bb9/\u6ce8\u91ca", None))
        ___qtablewidgetitem2 = self.dataTable.horizontalHeaderItem(2)
        ___qtablewidgetitem2.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u5e8f\u53f7", None))
        ___qtablewidgetitem3 = self.dataTable.horizontalHeaderItem(3)
        ___qtablewidgetitem3.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u5ef6\u65f6", None))
        self.addButton.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u6dfb\u52a0", None))
        self.deleteButton.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u5220\u9664", None))
        self.moveUpButton.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u4e0a\u79fb", None))
        self.moveDownButton.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u4e0b\u79fb", None))
        self.loopSendButton.setText(QCoreApplication.translate("ExtendedSendWidget", u"\u5faa\u73af\u53d1\u9001", None))
        self.helpLabel.setText(QCoreApplication.translate("ExtendedSendWidget", u"?", None))
#if QT_CONFIG(tooltip)
        self.helpLabel.setToolTip(QCoreApplication.translate("ExtendedSendWidget", u"\u6269\u5c55\u53d1\u9001\u8bf4\u660e\uff1a\n"
"\n"
"1. HEX\u5217\uff1a\u52fe\u9009\u8868\u793aHEX\u683c\u5f0f\uff0c\u4e0d\u52fe\u9009\u8868\u793aASCII\u683c\u5f0f\n"
"2. \u5e8f\u53f7\uff1a0=\u4e0d\u53c2\u4e0e\u53d1\u9001\uff0c1,2,3...=\u53d1\u9001\u987a\u5e8f\n"
"3. \u5ef6\u65f6\uff1a\u53d1\u9001\u540e\u7b49\u5f85\u65f6\u95f4\uff08\u6beb\u79d2\uff09\n"
"4. \u5de6\u952e\u70b9\u51fb\u53d1\u9001\u6309\u94ae\uff1a\u53d1\u9001\u5355\u6761\u6570\u636e\n"
"5. \u53f3\u952e\u70b9\u51fb\u53d1\u9001\u6309\u94ae\uff1a\u7f16\u8f91\u6ce8\u91ca\n"
"6. \u53f3\u952e\u70b9\u51fb\u8868\u683c\uff1a\u5f39\u51fa\u66f4\u591a\u529f\u80fd\u83dc\u5355", None))
#endif // QT_CONFIG(tooltip)
    # retranslateUi

