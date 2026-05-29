#include "main_window.h"
#include "ui_main_window.h"
#include "serial_settings_dialog.h"
#include "extended_send_widget.h"
#include "core/log_converter.h"
#include "version.h"

#include <QMessageBox>
#include <QCloseEvent>
#include <QSerialPortInfo>
#include <QThread>
#include <QFileDialog>
#include <QInputDialog>

MainWindow::MainWindow(int instanceId, QWidget *parent)
    : QMainWindow(parent), ui(new Ui::MainWindow), m_instanceId(instanceId)
{
    ui->setupUi(this);
    setWindowTitle(QStringLiteral("%1 V%2 (Instance %3)")
                   .arg(Version::appName(), Version::versionString())
                   .arg(instanceId));

    // Initialize core managers
    QString configDir = "config";
    m_configManager = new ConfigManager(configDir, instanceId, this);
    m_dataHandler = new DataHandler;
    m_logger = new Logger("logs", instanceId);
    m_ansiParser = new AnsiParser;

    // Initialize IO managers
    m_serialManager = new SerialManager(this);
    m_rttManager = new RttManager(this);
    m_socketManager = new SocketManager(this);

    // Initialize extended send manager
    m_extSendManager = new ExtendedSendManager(
        [this](const QByteArray &data) -> bool {
            if (!currentIO()->isConnected()) return false;
            return currentIO()->sendData(QString::fromUtf8(data), false);
        },
        configDir + "/extended_send.json",
        this
    );

    // Initialize UI helpers
    m_displayHandler = new ReceiveDisplayHandler(
        ui->receiveTextEdit, m_dataHandler, m_ansiParser, m_logger,
        [this]() -> QString { return ui->hexRadio->isChecked() ? "HEX" :
                                     ui->asciiRadio->isChecked() ? "ASCII" : "MIXED"; },
        [this]() -> bool { return m_displayAnsi; },
        this
    );

    m_statusBarController = new StatusBarController(ui->statusbar);

    // Initialize extended send widget
    m_extSendWidget = new ExtendedSendWidget(m_extSendManager);
    ui->extendedSendContainer->setLayout(new QVBoxLayout);
    ui->extendedSendContainer->layout()->addWidget(m_extSendWidget);
    ui->extendedSendContainer->setVisible(false);

    // Timers
    m_autoSendTimer = new QTimer(this);
    m_autoSendTimer->setInterval(1000);
    connect(m_autoSendTimer, &QTimer::timeout, this, &MainWindow::sendData);

    m_memoryTimer = new QTimer(this);
    m_memoryTimer->setInterval(10000);
    connect(m_memoryTimer, &QTimer::timeout, [this]() {
        // Periodic memory cleanup
        qApp->processEvents();
    });
    m_memoryTimer->start();

    m_logFlushTimer = new QTimer(this);
    m_logFlushTimer->setInterval(1000);
    connect(m_logFlushTimer, &QTimer::timeout, [this]() {
        m_logger->flush();
        m_extSendManager->flush();
    });
    m_logFlushTimer->start();

    m_configDebounceTimer = new QTimer(this);
    m_configDebounceTimer->setSingleShot(true);
    m_configDebounceTimer->setInterval(500);

    initUI();
    setupConnections();
    loadConfig();
}

MainWindow::~MainWindow() {
    delete m_dataHandler;
    delete m_logger;
    delete m_ansiParser;
    delete ui;
}

void MainWindow::initUI() {
    // Populate baudrate combo
    QList<int> baudrates = {9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600};
    for (int baud : baudrates) {
        ui->baudrateCombo->addItem(QString::number(baud));
    }
    ui->baudrateCombo->setCurrentText("115200");

    // Default display mode
    ui->asciiRadio->setChecked(true);

    // Baudrate stack
    ui->baudrateStack->setCurrentIndex(0); // Serial page

    // Setup context menu for receive text
    m_displayHandler->setupContextMenu([this]() {
        m_displayAnsi = !m_displayAnsi;
    });
}

void MainWindow::setupConnections() {
    // Toolbar buttons
    connect(ui->refreshButton, &QPushButton::clicked, this, &MainWindow::refreshPorts);
    connect(ui->openButton, &QPushButton::clicked, this, &MainWindow::toggleConnection);
    connect(ui->settingsButton, &QPushButton::clicked, this, &MainWindow::showSerialSettings);
    connect(ui->sendButton, &QPushButton::clicked, this, &MainWindow::sendData);
    connect(ui->clearReceiveButton, &QPushButton::clicked, [this]() {
        ui->receiveTextEdit->clear();
    });

    // Toggle preset panel
    connect(ui->togglePresetButton, &QPushButton::toggled, this, &MainWindow::togglePresetPanel);

    // Auto send
    connect(ui->autoSendCheckBox, &QCheckBox::toggled, [this](bool checked) {
        if (checked) {
            m_autoSendTimer->setInterval(ui->intervalSpinBox->value());
            m_autoSendTimer->start();
        } else {
            m_autoSendTimer->stop();
        }
    });

    // Display mode change
    connect(ui->hexRadio, &QRadioButton::toggled, [this]() { /* refresh display */ });
    connect(ui->asciiRadio, &QRadioButton::toggled, [this]() { /* refresh display */ });
    connect(ui->mixedRadio, &QRadioButton::toggled, [this]() { /* refresh display */ });

    // IO manager signals
    auto connectIO = [this](IOTransport *io) {
        connect(io, &IOTransport::dataReceived, m_displayHandler, &ReceiveDisplayHandler::onDataReceived);
        connect(io, &IOTransport::connectionChanged, this, &MainWindow::onConnectionChanged);
        connect(io, &IOTransport::errorOccurred, this, &MainWindow::onError);
    };
    connectIO(m_serialManager);
    connectIO(m_rttManager);
    connectIO(m_socketManager);

    // Socket client events
    connect(m_socketManager, &SocketManager::clientEvent,
            this, &MainWindow::onSocketClientEvent);

    // Menu actions
    connect(ui->actionExit, &QAction::triggered, this, &QMainWindow::close);
    connect(ui->actionClearReceive, &QAction::triggered, [this]() {
        ui->receiveTextEdit->clear();
    });
    connect(ui->actionClearSend, &QAction::triggered, [this]() {
        ui->sendTextEdit->clear();
    });
    connect(ui->actionSettings, &QAction::triggered, this, &MainWindow::showSerialSettings);
    connect(ui->togglePresetAction, &QAction::triggered, [this](bool checked) {
        ui->togglePresetButton->setChecked(checked);
    });
    connect(ui->actionAbout, &QAction::triggered, this, &MainWindow::showAbout);

    // Log converter
    connect(ui->actionLogConverter, &QAction::triggered, this, &MainWindow::showLogConverter);

    // Extended send widget
    connect(m_extSendWidget, &ExtendedSendWidget::sendData, [this](const QByteArray &data) {
        if (currentIO()->isConnected()) {
            currentIO()->sendData(QString::fromUtf8(data), false);
            m_displayHandler->appendData(data, "->", "SEND");
        }
    });
}

void MainWindow::refreshPorts() {
    ui->portCombo->clear();

    // Serial ports
    for (const auto &port : SerialManager::getAvailablePorts()) {
        ui->portCombo->addItem(port.first + " - " + port.second, "serial");
    }

    // J-Link devices
    // TODO: Add J-Link scan in background thread

    // Socket modes
    ui->portCombo->addItem("TCP Server", "socket");
    ui->portCombo->addItem("TCP Client", "socket");
    ui->portCombo->addItem("UDP Server", "socket");
    ui->portCombo->addItem("UDP Client", "socket");
}

void MainWindow::showSerialSettings() {
    QVariantMap serialSettings;
    serialSettings["databits"] = m_configManager->get("databits", 8);
    serialSettings["stopbits"] = m_configManager->get("stopbits", 1.0);
    serialSettings["parity"] = m_configManager->get("parity", "None");
    serialSettings["flowcontrol"] = m_configManager->get("flowcontrol", "None");
    serialSettings["frame_timeout"] = m_configManager->get("frame_timeout", 50);

    QVariantMap rttSettings;
    rttSettings["chip"] = m_configManager->get("rtt_chip");
    rttSettings["speed"] = m_configManager->get("rtt_speed", 4000);
    rttSettings["reset"] = m_configManager->get("rtt_reset", false);
    rttSettings["start_address"] = m_configManager->get("rtt_start_address");
    rttSettings["range_size"] = m_configManager->get("rtt_range_size");
    rttSettings["chip_history"] = m_configManager->get("rtt_chip_history");

    auto *dialog = new SerialSettingsDialog(serialSettings, rttSettings, m_displayAnsi, this);

    connect(dialog, &SerialSettingsDialog::settingsChanged, [this](const QVariantMap &s) {
        for (auto it = s.begin(); it != s.end(); ++it) {
            m_configManager->set(it.key(), it.value());
        }
    });

    connect(dialog, &SerialSettingsDialog::rttSettingsChanged, [this](const QVariantMap &s) {
        m_configManager->set("rtt_chip", s["chip"]);
        m_configManager->set("rtt_speed", s["speed"]);
        m_configManager->set("rtt_reset", s["reset"]);
        m_configManager->set("rtt_start_address", s["start_address"]);
        m_configManager->set("rtt_range_size", s["range_size"]);
    });

    connect(dialog, &SerialSettingsDialog::commonSettingsChanged, [this](const QVariantMap &s) {
        m_configManager->set("frame_timeout", s["frame_timeout"]);
        m_displayAnsi = s["display_ansi"].toBool();
    });

    dialog->exec();
    dialog->deleteLater();
}

void MainWindow::toggleConnection() {
    if (currentIO()->isConnected()) {
        currentIO()->closeConnection();
        return;
    }

    QString portText = ui->portCombo->currentText();

    if (portText.contains("SOCKET") || portText.contains("TCP") || portText.contains("UDP")) {
        // Socket mode
        m_ioMode = IoMode::Socket;
        QString protocol = portText.contains("TCP") ? "tcp" : "udp";
        QString role = portText.contains("Server") ? "server" : "client";
        QString host = ui->ipCombo->currentText();
        int port = ui->portSpin->value();

        QVariantMap params;
        params["host"] = host;
        params["port"] = port;
        params["protocol"] = protocol;
        params["role"] = role;
        m_socketManager->openConnection(params);

    } else if (portText.contains("JLINK")) {
        // RTT mode
        m_ioMode = IoMode::Rtt;
        QVariantMap params;
        params["chip"] = m_configManager->get("rtt_chip");
        params["speed"] = m_configManager->get("rtt_speed", 4000);
        params["reset"] = m_configManager->get("rtt_reset", false);
        params["start_address"] = m_configManager->get("rtt_start_address");
        params["range_size"] = m_configManager->get("rtt_range_size");
        m_rttManager->openConnection(params);

    } else {
        // Serial mode
        m_ioMode = IoMode::Serial;
        QString portName = portText.split(" - ").first();
        QVariantMap params;
        params["port"] = portName;
        params["baudrate"] = ui->baudrateCombo->currentText().toInt();
        params["databits"] = m_configManager->get("databits", 8);
        params["stopbits"] = m_configManager->get("stopbits", 1.0);
        params["parity"] = m_configManager->get("parity", "None");
        params["flowcontrol"] = m_configManager->get("flowcontrol", "None");
        params["frame_timeout"] = m_configManager->get("frame_timeout", 50);
        m_serialManager->openConnection(params);
    }
}

void MainWindow::sendData() {
    if (!currentIO()->isConnected()) return;

    QString text = ui->sendTextEdit->toPlainText();
    if (text.isEmpty()) return;

    bool isHex = ui->sendHexRadio->isChecked();

    // Append newline if checked
    if (ui->appendNewLineCheckBox->isChecked() && !isHex) {
        text += "\r\n";
    }

    if (currentIO()->sendData(text, isHex)) {
        // Display sent data
        QByteArray data = isHex ? QByteArray::fromHex(text.toLatin1()) : text.toUtf8();
        m_displayHandler->appendData(data, "->", "SEND");
    }
}

void MainWindow::onConnectionChanged(bool connected) {
    if (connected) {
        ui->openButton->setText("Close");
        ui->openButton->setStyleSheet("background-color: #90EE90;");
        m_statusBarController->setConnected("Connected");
    } else {
        ui->openButton->setText("Open");
        ui->openButton->setStyleSheet("");
        m_statusBarController->setDisconnected();
    }
}

void MainWindow::onError(const QString &error) {
    m_displayHandler->appendEvent("Error: " + error, "#FF0000");
}

void MainWindow::onSocketClientEvent(const QString &eventType, const QPair<QString, int> &address) {
    QString msg = QStringLiteral("%1 %2:%3")
                  .arg(eventType, address.first)
                  .arg(address.second);
    m_displayHandler->appendEvent(msg, "#0000FF");
}

void MainWindow::togglePresetPanel(bool checked) {
    ui->extendedSendContainer->setVisible(checked);
    ui->togglePresetAction->setChecked(checked);
    m_configManager->set("preset_panel_visible", checked);
}

void MainWindow::loadConfig() {
    // Baudrate
    QString baudrate = m_configManager->get("baudrate", "115200").toString();
    int idx = ui->baudrateCombo->findText(baudrate);
    if (idx >= 0) ui->baudrateCombo->setCurrentIndex(idx);

    // Display mode
    QString mode = m_configManager->get("display_mode", "ASCII").toString();
    if (mode == "HEX") ui->hexRadio->setChecked(true);
    else if (mode == "MIXED") ui->mixedRadio->setChecked(true);
    else ui->asciiRadio->setChecked(true);

    // Auto scroll
    ui->autoScrollCheckBox->setChecked(m_configManager->get("auto_scroll", true).toBool());

    // Display ANSI
    m_displayAnsi = m_configManager->get("display_ansi", false).toBool();

    // Preset panel
    bool presetVisible = m_configManager->get("preset_panel_visible", false).toBool();
    ui->togglePresetButton->setChecked(presetVisible);
    ui->extendedSendContainer->setVisible(presetVisible);

    // Splitter sizes
    QVariant mainSplitterSizes = m_configManager->get("main_splitter_sizes");
    if (mainSplitterSizes.isValid()) {
        QList<int> sizes;
        for (const QVariant &v : mainSplitterSizes.toList()) {
            sizes.append(v.toInt());
        }
        if (!sizes.isEmpty()) {
            ui->mainSplitter->setSizes(sizes);
        }
    }

    QVariant topSplitterSizes = m_configManager->get("top_splitter_sizes");
    if (topSplitterSizes.isValid()) {
        QList<int> sizes;
        for (const QVariant &v : topSplitterSizes.toList()) {
            sizes.append(v.toInt());
        }
        if (!sizes.isEmpty()) {
            ui->topSplitter->setSizes(sizes);
        }
    }

    // Refresh ports
    refreshPorts();
}

void MainWindow::saveConfig() {
    m_configManager->save();
}

void MainWindow::saveConfigItem(const QString &key) {
    // Already handled by ConfigManager's debounce
}

void MainWindow::showLogConverter() {
    QString filePath = QFileDialog::getOpenFileName(
        this, QStringLiteral("选择日志文件"),
        m_logger->logDir(),
        QStringLiteral("日志文件 (*.txt);;所有文件 (*)")
    );
    if (filePath.isEmpty()) return;

    QStringList formats;
    formats << "HEX" << "ASCII" << "Both";

    bool ok;
    QString format = QInputDialog::getItem(
        this, QStringLiteral("输出格式"),
        QStringLiteral("选择转换格式:"),
        formats, 2, false, &ok
    );
    if (!ok) return;

    LogConverter::Format fmt;
    if (format == "HEX") fmt = LogConverter::Format::Hex;
    else if (format == "ASCII") fmt = LogConverter::Format::ASCII;
    else fmt = LogConverter::Format::Both;

    try {
        LogConverter::Result result = LogConverter::convertFile(filePath, fmt);

        QStringList parts;
        if (!result.hexPath.isEmpty())
            parts << QStringLiteral("  HEX: %1").arg(result.hexPath);
        if (!result.asciiPath.isEmpty())
            parts << QStringLiteral("  ASCII: %1").arg(result.asciiPath);

        QMessageBox::information(
            this, QStringLiteral("转换成功"),
            QStringLiteral("文件已生成：\n%1").arg(parts.join('\n'))
        );
    } catch (const std::exception &e) {
        QMessageBox::warning(
            this, QStringLiteral("转换失败"),
            QStringLiteral("转换出错：%1").arg(e.what())
        );
    }
}

void MainWindow::showAbout() {
    QMessageBox::about(this, QStringLiteral("关于"),
        QStringLiteral("<h3>%1 V%2</h3>"
                       "<p>基于 Qt C++ 开发的多协议调试工具</p>"
                       "<p>功能特性：</p>"
                       "<ul>"
                       "<li>串口通信</li>"
                       "<li>J-Link RTT</li>"
                       "<li>TCP/UDP Socket</li>"
                       "<li>扩展发送</li>"
                       "<li>日志记录与转换</li>"
                       "</ul>")
        .arg(Version::appName(), Version::versionString()));
}

void MainWindow::closeEvent(QCloseEvent *event) {
    // Save config
    m_configManager->set("baudrate", ui->baudrateCombo->currentText());
    m_configManager->set("display_mode",
        ui->hexRadio->isChecked() ? "HEX" :
        ui->asciiRadio->isChecked() ? "ASCII" : "MIXED");
    m_configManager->set("auto_scroll", ui->autoScrollCheckBox->isChecked());
    m_configManager->set("display_ansi", m_displayAnsi);
    m_configManager->set("preset_panel_visible", ui->togglePresetButton->isChecked());

    // Save splitter sizes
    QVariantList mainSizes;
    for (int size : ui->mainSplitter->sizes()) {
        mainSizes.append(size);
    }
    m_configManager->set("main_splitter_sizes", mainSizes);

    QVariantList topSizes;
    for (int size : ui->topSplitter->sizes()) {
        topSizes.append(size);
    }
    m_configManager->set("top_splitter_sizes", topSizes);

    m_configManager->save();

    // Disconnect
    if (currentIO()->isConnected()) {
        currentIO()->closeConnection();
    }

    // Stop timers
    m_autoSendTimer->stop();
    m_memoryTimer->stop();
    m_logFlushTimer->stop();

    // Flush
    m_logger->flush();
    m_extSendManager->flush();

    event->accept();
}

IOTransport *MainWindow::currentIO() const {
    switch (m_ioMode) {
    case IoMode::Serial: return m_serialManager;
    case IoMode::Rtt: return m_rttManager;
    case IoMode::Socket: return m_socketManager;
    }
    return m_serialManager;
}
