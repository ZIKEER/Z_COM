#ifndef MAIN_WINDOW_H
#define MAIN_WINDOW_H

#include <QMainWindow>
#include <QTimer>
#include <QThread>

#include "core/config_manager.h"
#include "core/data_handler.h"
#include "core/logger.h"
#include "core/ansi_parser.h"
#include "core/extended_send_manager.h"
#include "io/serial_manager.h"
#include "io/rtt_manager.h"
#include "io/socket_manager.h"
#include "windows/receive_display_handler.h"
#include "windows/status_bar_controller.h"

namespace Ui { class MainWindow; }

class ExtendedSendWidget;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(int instanceId = 1, QWidget *parent = nullptr);
    ~MainWindow() override;

protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void refreshPorts();
    void showSerialSettings();
    void toggleConnection();
    void sendData();
    void onConnectionChanged(bool connected);
    void onError(const QString &error);
    void onSocketClientEvent(const QString &eventType, const QPair<QString, int> &address);
    void togglePresetPanel(bool checked);
    void showLogConverter();
    void showAbout();
    void checkPortChanges();
    void onBaudrateChanged(const QString &text);

private:
    void initUI();
    void setupConnections();
    void loadConfig();
    void saveConfig();
    void saveConfigItem(const QString &key);

    // IO mode
    enum class IoMode { Serial, Rtt, Socket };
    IOTransport *currentIO() const;
    IoMode m_ioMode = IoMode::Serial;

    Ui::MainWindow *ui;
    int m_instanceId;

    // Managers
    ConfigManager *m_configManager;
    DataHandler *m_dataHandler;
    Logger *m_logger;
    AnsiParser *m_ansiParser;
    ExtendedSendManager *m_extSendManager;
    SerialManager *m_serialManager;
    RttManager *m_rttManager;
    SocketManager *m_socketManager;

    // UI helpers
    ReceiveDisplayHandler *m_displayHandler;
    StatusBarController *m_statusBarController;
    ExtendedSendWidget *m_extSendWidget;

    // Timers
    QTimer *m_autoSendTimer;
    QTimer *m_memoryTimer;
    QTimer *m_logFlushTimer;
    QTimer *m_configDebounceTimer;
    QTimer *m_portPollTimer;

    // State
    bool m_displayAnsi = false;
    QStringList m_lastPorts;
};

#endif // MAIN_WINDOW_H
