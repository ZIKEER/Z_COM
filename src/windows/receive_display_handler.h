#ifndef RECEIVE_DISPLAY_HANDLER_H
#define RECEIVE_DISPLAY_HANDLER_H

#include <QObject>
#include <QTimer>
#include <QByteArray>
#include <QTextEdit>
#include <functional>

#include "core/data_handler.h"
#include "core/ansi_parser.h"
#include "core/logger.h"

class ReceiveDisplayHandler : public QObject {
    Q_OBJECT

signals:
    void countsChanged(qint64 sendCount, qint64 receiveCount);

public:
    using DisplayModeFunc = std::function<QString()>;
    using DisplayAnsiFunc = std::function<bool()>;

    explicit ReceiveDisplayHandler(QTextEdit *textEdit,
                                   DataHandler *dataHandler,
                                   AnsiParser *ansiParser,
                                   Logger *logger,
                                   DisplayModeFunc displayModeFunc,
                                   DisplayAnsiFunc displayAnsiFunc,
                                   QObject *parent = nullptr);

    void setBatchWindow(int ms);

    // Called when data is received from IO
    void onDataReceived(const QByteArray &data);

    // Append formatted data to display
    void appendData(const QByteArray &data, const QString &arrow,
                    const QString &logType, const QString &clientPrefix = QString());

    // Append event text
    void appendEvent(const QString &text, const QString &color = "#000000");

    // Get counts
    qint64 receiveCount() const { return m_receiveCount; }
    qint64 sendCount() const { return m_sendCount; }
    void resetCounts() { m_receiveCount = 0; m_sendCount = 0; }

    // Setup context menu with ANSI toggle
    void setupContextMenu(std::function<void()> toggleAnsiCallback);

    // Check for incomplete ANSI sequence at end of data
    static QByteArray findIncompleteAnsiTail(const QByteArray &data);

private:
    void flushPending();
    void checkPrune();
    void pruneIfNeeded();
    QString formatDisplay(const QByteArray &data, const QString &mode,
                         const QString &timestamp, const QString &arrow);

    QTextEdit *m_textEdit;
    DataHandler *m_dataHandler;
    AnsiParser *m_ansiParser;
    Logger *m_logger;
    DisplayModeFunc m_displayModeFunc;
    DisplayAnsiFunc m_displayAnsiFunc;

    QTimer *m_flushTimer;
    QTimer *m_memoryTimer;
    QByteArray m_pendingData;
    int m_appendCount = 0;
    qint64 m_receiveCount = 0;
    qint64 m_sendCount = 0;

    // Display limits
    static const int MAX_DISPLAY_LINES = 10000;
    static const int DISPLAY_PRUNE_LINES = 5000;
    static const int MEMORY_CHECK_INTERVAL_MS = 10000; // 10 seconds

    // Colors
    const QString TIMESTAMP_COLOR = "#00CED1";
    const QString ARROW_COLOR = "#000000";
    const QString DATA_COLOR = "#000000";
};

#endif // RECEIVE_DISPLAY_HANDLER_H
