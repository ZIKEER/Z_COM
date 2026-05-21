# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'main_window.ui'
##
## Created by: Qt User Interface Compiler version 6.11.0
##
## WARNING! All changes made in this file will be lost when recompiling UI file!
################################################################################

from PySide6.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
    QMetaObject, QObject, QPoint, QRect,
    QSize, QTime, QUrl, Qt)
from PySide6.QtGui import (QAction, QBrush, QColor, QConicalGradient,
    QCursor, QFont, QFontDatabase, QGradient,
    QIcon, QImage, QKeySequence, QLinearGradient,
    QPainter, QPalette, QPixmap, QRadialGradient,
    QTransform)
from PySide6.QtWidgets import (QApplication, QCheckBox, QGroupBox, QHBoxLayout,
    QLabel, QMainWindow, QMenu, QMenuBar,
    QPushButton, QRadioButton, QSizePolicy, QSpacerItem,
    QSpinBox, QSplitter, QStatusBar, QTextEdit,
    QVBoxLayout, QWidget)

class Ui_MainWindow(object):
    def setupUi(self, MainWindow):
        if not MainWindow.objectName():
            MainWindow.setObjectName(u"MainWindow")
        MainWindow.resize(890, 685)
        self.actionExit = QAction(MainWindow)
        self.actionExit.setObjectName(u"actionExit")
        self.actionClearReceive = QAction(MainWindow)
        self.actionClearReceive.setObjectName(u"actionClearReceive")
        self.actionClearSend = QAction(MainWindow)
        self.actionClearSend.setObjectName(u"actionClearSend")
        self.actionSettings = QAction(MainWindow)
        self.actionSettings.setObjectName(u"actionSettings")
        self.togglePresetAction = QAction(MainWindow)
        self.togglePresetAction.setObjectName(u"togglePresetAction")
        self.togglePresetAction.setCheckable(True)
        self.actionAbout = QAction(MainWindow)
        self.actionAbout.setObjectName(u"actionAbout")
        self.centralwidget = QWidget(MainWindow)
        self.centralwidget.setObjectName(u"centralwidget")
        self.verticalLayout = QVBoxLayout(self.centralwidget)
        self.verticalLayout.setSpacing(2)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.verticalLayout.setContentsMargins(2, 2, 2, 2)
        self.mainSplitter = QSplitter(self.centralwidget)
        self.mainSplitter.setObjectName(u"mainSplitter")
        self.mainSplitter.setOrientation(Qt.Orientation.Vertical)
        self.topSplitter = QSplitter(self.mainSplitter)
        self.topSplitter.setObjectName(u"topSplitter")
        self.topSplitter.setOrientation(Qt.Orientation.Horizontal)
        self.receiveGroup = QGroupBox(self.topSplitter)
        self.receiveGroup.setObjectName(u"receiveGroup")
        self.receiveGroup.setMinimumSize(QSize(50, 30))
        self.receiveLayout = QVBoxLayout(self.receiveGroup)
        self.receiveLayout.setSpacing(1)
        self.receiveLayout.setObjectName(u"receiveLayout")
        self.receiveLayout.setContentsMargins(2, 2, 2, 2)
        self.toolbarLayout = QHBoxLayout()
        self.toolbarLayout.setSpacing(3)
        self.toolbarLayout.setObjectName(u"toolbarLayout")
        self.hexRadio = QRadioButton(self.receiveGroup)
        self.hexRadio.setObjectName(u"hexRadio")

        self.toolbarLayout.addWidget(self.hexRadio)

        self.asciiRadio = QRadioButton(self.receiveGroup)
        self.asciiRadio.setObjectName(u"asciiRadio")
        self.asciiRadio.setChecked(True)

        self.toolbarLayout.addWidget(self.asciiRadio)

        self.mixedRadio = QRadioButton(self.receiveGroup)
        self.mixedRadio.setObjectName(u"mixedRadio")

        self.toolbarLayout.addWidget(self.mixedRadio)

        self.autoScrollCheckBox = QCheckBox(self.receiveGroup)
        self.autoScrollCheckBox.setObjectName(u"autoScrollCheckBox")
        self.autoScrollCheckBox.setChecked(True)

        self.toolbarLayout.addWidget(self.autoScrollCheckBox)

        self.clearReceiveButton = QPushButton(self.receiveGroup)
        self.clearReceiveButton.setObjectName(u"clearReceiveButton")

        self.toolbarLayout.addWidget(self.clearReceiveButton)

        self.toolbarSpacer = QSpacerItem(10, 10, QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Minimum)

        self.toolbarLayout.addItem(self.toolbarSpacer)


        self.receiveLayout.addLayout(self.toolbarLayout)

        self.receiveTextEdit = QTextEdit(self.receiveGroup)
        self.receiveTextEdit.setObjectName(u"receiveTextEdit")
        self.receiveTextEdit.setReadOnly(True)

        self.receiveLayout.addWidget(self.receiveTextEdit)

        self.topSplitter.addWidget(self.receiveGroup)
        self.extendedSendContainer = QWidget(self.topSplitter)
        self.extendedSendContainer.setObjectName(u"extendedSendContainer")
        self.extendedSendContainer.setMinimumSize(QSize(80, 0))
        self.extendedSendContainer.setVisible(False)
        self.topSplitter.addWidget(self.extendedSendContainer)
        self.mainSplitter.addWidget(self.topSplitter)
        self.sendGroup = QGroupBox(self.mainSplitter)
        self.sendGroup.setObjectName(u"sendGroup")
        self.sendGroup.setMinimumSize(QSize(0, 40))
        self.sendMainLayout = QHBoxLayout(self.sendGroup)
        self.sendMainLayout.setSpacing(2)
        self.sendMainLayout.setObjectName(u"sendMainLayout")
        self.sendMainLayout.setContentsMargins(2, 2, 2, 2)
        self.openButton = QPushButton(self.sendGroup)
        self.openButton.setObjectName(u"openButton")
        self.openButton.setMinimumSize(QSize(50, 0))
        self.openButton.setMaximumSize(QSize(50, 16777215))

        self.sendMainLayout.addWidget(self.openButton)

        self.sendCenterLayout = QVBoxLayout()
        self.sendCenterLayout.setSpacing(1)
        self.sendCenterLayout.setObjectName(u"sendCenterLayout")
        self.sendConfigLayout = QHBoxLayout()
        self.sendConfigLayout.setSpacing(3)
        self.sendConfigLayout.setObjectName(u"sendConfigLayout")
        self.sendAsciiRadio = QRadioButton(self.sendGroup)
        self.sendAsciiRadio.setObjectName(u"sendAsciiRadio")
        self.sendAsciiRadio.setChecked(True)

        self.sendConfigLayout.addWidget(self.sendAsciiRadio)

        self.sendHexRadio = QRadioButton(self.sendGroup)
        self.sendHexRadio.setObjectName(u"sendHexRadio")

        self.sendConfigLayout.addWidget(self.sendHexRadio)

        self.appendNewLineCheckBox = QCheckBox(self.sendGroup)
        self.appendNewLineCheckBox.setObjectName(u"appendNewLineCheckBox")

        self.sendConfigLayout.addWidget(self.appendNewLineCheckBox)

        self.sendConfigSpacer = QSpacerItem(10, 10, QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Minimum)

        self.sendConfigLayout.addItem(self.sendConfigSpacer)

        self.autoSendCheckBox = QCheckBox(self.sendGroup)
        self.autoSendCheckBox.setObjectName(u"autoSendCheckBox")

        self.sendConfigLayout.addWidget(self.autoSendCheckBox)

        self.intervalSpinBox = QSpinBox(self.sendGroup)
        self.intervalSpinBox.setObjectName(u"intervalSpinBox")
        self.intervalSpinBox.setMinimum(100)
        self.intervalSpinBox.setMaximum(60000)
        self.intervalSpinBox.setValue(1000)

        self.sendConfigLayout.addWidget(self.intervalSpinBox)

        self.msLabel = QLabel(self.sendGroup)
        self.msLabel.setObjectName(u"msLabel")

        self.sendConfigLayout.addWidget(self.msLabel)


        self.sendCenterLayout.addLayout(self.sendConfigLayout)

        self.sendTextEdit = QTextEdit(self.sendGroup)
        self.sendTextEdit.setObjectName(u"sendTextEdit")
        self.sendTextEdit.setMaximumSize(QSize(16777215, 16777215))

        self.sendCenterLayout.addWidget(self.sendTextEdit)


        self.sendMainLayout.addLayout(self.sendCenterLayout)

        self.sendButton = QPushButton(self.sendGroup)
        self.sendButton.setObjectName(u"sendButton")
        self.sendButton.setMinimumSize(QSize(50, 0))
        self.sendButton.setMaximumSize(QSize(50, 16777215))

        self.sendMainLayout.addWidget(self.sendButton)

        self.mainSplitter.addWidget(self.sendGroup)

        self.verticalLayout.addWidget(self.mainSplitter)

        MainWindow.setCentralWidget(self.centralwidget)
        self.statusbar = QStatusBar(MainWindow)
        self.statusbar.setObjectName(u"statusbar")
        MainWindow.setStatusBar(self.statusbar)
        self.menubar = QMenuBar(MainWindow)
        self.menubar.setObjectName(u"menubar")
        self.menubar.setGeometry(QRect(0, 0, 580, 18))
        self.menuFile = QMenu(self.menubar)
        self.menuFile.setObjectName(u"menuFile")
        self.menuEdit = QMenu(self.menubar)
        self.menuEdit.setObjectName(u"menuEdit")
        self.menuTool = QMenu(self.menubar)
        self.menuTool.setObjectName(u"menuTool")
        self.menuHelp = QMenu(self.menubar)
        self.menuHelp.setObjectName(u"menuHelp")
        MainWindow.setMenuBar(self.menubar)

        self.menubar.addAction(self.menuFile.menuAction())
        self.menubar.addAction(self.menuEdit.menuAction())
        self.menubar.addAction(self.menuTool.menuAction())
        self.menubar.addAction(self.menuHelp.menuAction())
        self.menuFile.addAction(self.actionExit)
        self.menuEdit.addAction(self.actionClearReceive)
        self.menuEdit.addAction(self.actionClearSend)
        self.menuTool.addAction(self.actionSettings)
        self.menuTool.addSeparator()
        self.menuTool.addAction(self.togglePresetAction)
        self.menuHelp.addAction(self.actionAbout)

        self.retranslateUi(MainWindow)

        QMetaObject.connectSlotsByName(MainWindow)
    # setupUi

    def retranslateUi(self, MainWindow):
        MainWindow.setWindowTitle(QCoreApplication.translate("MainWindow", u"\u4e32\u53e3\u52a9\u624b", None))
        self.actionExit.setText(QCoreApplication.translate("MainWindow", u"\u9000\u51fa", None))
        self.actionClearReceive.setText(QCoreApplication.translate("MainWindow", u"\u6e05\u7a7a\u63a5\u6536", None))
        self.actionClearSend.setText(QCoreApplication.translate("MainWindow", u"\u6e05\u7a7a\u53d1\u9001", None))
        self.actionSettings.setText(QCoreApplication.translate("MainWindow", u"\u4e32\u53e3\u8bbe\u7f6e", None))
        self.togglePresetAction.setText(QCoreApplication.translate("MainWindow", u"\u6269\u5c55\u53d1\u9001", None))
        self.actionAbout.setText(QCoreApplication.translate("MainWindow", u"\u5173\u4e8e", None))
        self.receiveGroup.setTitle(QCoreApplication.translate("MainWindow", u"\u663e\u793a\u533a\u57df", None))
        self.hexRadio.setText(QCoreApplication.translate("MainWindow", u"HEX", None))
        self.asciiRadio.setText(QCoreApplication.translate("MainWindow", u"ASCII", None))
        self.mixedRadio.setText(QCoreApplication.translate("MainWindow", u"HEX+ASCII", None))
        self.autoScrollCheckBox.setText(QCoreApplication.translate("MainWindow", u"\u81ea\u52a8\u6eda\u52a8", None))
        self.clearReceiveButton.setText(QCoreApplication.translate("MainWindow", u"\u6e05\u7a7a", None))
        self.sendGroup.setTitle(QCoreApplication.translate("MainWindow", u"\u53d1\u9001\u533a\u57df", None))
        self.openButton.setText(QCoreApplication.translate("MainWindow", u"\u6253\u5f00\n"
"\u4e32\u53e3", None))
        self.sendAsciiRadio.setText(QCoreApplication.translate("MainWindow", u"ASCII", None))
        self.sendHexRadio.setText(QCoreApplication.translate("MainWindow", u"HEX", None))
        self.appendNewLineCheckBox.setText(QCoreApplication.translate("MainWindow", u"\u6dfb\u52a0\u56de\u8f66\u6362\u884c", None))
        self.autoSendCheckBox.setText(QCoreApplication.translate("MainWindow", u"\u81ea\u52a8\u53d1\u9001", None))
        self.msLabel.setText(QCoreApplication.translate("MainWindow", u"ms", None))
        self.sendButton.setText(QCoreApplication.translate("MainWindow", u"\u53d1\n"
"\u9001", None))
        self.menuFile.setTitle(QCoreApplication.translate("MainWindow", u"\u6587\u4ef6", None))
        self.menuEdit.setTitle(QCoreApplication.translate("MainWindow", u"\u7f16\u8f91", None))
        self.menuTool.setTitle(QCoreApplication.translate("MainWindow", u"\u5de5\u5177", None))
        self.menuHelp.setTitle(QCoreApplication.translate("MainWindow", u"\u5e2e\u52a9", None))
    # retranslateUi

