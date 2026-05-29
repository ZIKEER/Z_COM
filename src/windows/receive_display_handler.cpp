#include "receive_display_handler.h"

#include <QDateTime>
#include <QTextCursor>
#include <QMenu>
#include <QAction>

ReceiveDisplayHandler::ReceiveDisplayHandler(QTextEdit *textEdit,
                                             DataHandler *dataHandler,
                                             AnsiParser *ansiParser,
                                             Logger *logger,
                                             DisplayModeFunc displayModeFunc,
                                             DisplayAnsiFunc displayAnsiFunc,
                                             QObject *parent)
    : QObject(parent), m_textEdit(textEdit), m_dataHandler(dataHandler),
      m_ansiParser(ansiParser), m_logger(logger),
      m_displayModeFunc(std::move(displayModeFunc)),
      m_displayAnsiFunc(std::move(displayAnsiFunc))
{
    m_flushTimer = new QTimer(this);
    m_flushTimer->setSingleShot(true);
    m_flushTimer->setInterval(16); // ~60fps
    connect(m_flushTimer, &QTimer::timeout, this, &ReceiveDisplayHandler::flushPending);
}

void ReceiveDisplayHandler::setBatchWindow(int ms) {
    m_flushTimer->setInterval(ms);
}

void ReceiveDisplayHandler::onDataReceived(const QByteArray &data) {
    m_pendingData.append(data);
    if (!m_flushTimer->isActive()) {
        m_flushTimer->start();
    }
}

void ReceiveDisplayHandler::flushPending() {
    if (m_pendingData.isEmpty()) return;

    QByteArray data = m_pendingData;
    m_pendingData.clear();

    // Handle incomplete ANSI sequence at boundary
    QByteArray tail = findIncompleteAnsiTail(data);
    if (!tail.isEmpty()) {
        data.chop(tail.size());
        m_pendingData = tail;
        m_flushTimer->start();
    }

    appendData(data, "<-", "RECEIVE");
}

QByteArray ReceiveDisplayHandler::findIncompleteAnsiTail(const QByteArray &data) {
    // Look for ESC at the end without a terminator
    for (int i = data.size() - 1; i >= qMax(0, data.size() - 10); --i) {
        if (static_cast<uchar>(data[i]) == 0x1B) {
            // Found ESC, check if it has a complete sequence
            if (i + 1 < data.size() && static_cast<uchar>(data[i + 1]) == '[') {
                // CSI sequence - check for terminator
                for (int j = i + 2; j < data.size(); ++j) {
                    uchar c = static_cast<uchar>(data[j]);
                    if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || c == 'm') {
                        return QByteArray(); // Complete sequence
                    }
                }
                return data.mid(i); // Incomplete
            }
        }
    }
    return QByteArray();
}

void ReceiveDisplayHandler::appendData(const QByteArray &data, const QString &arrow,
                                        const QString &logType, const QString &clientPrefix) {
    if (data.isEmpty()) return;

    QString timestamp = QDateTime::currentDateTime().toString("HH:mm:ss.zzz");
    QString mode = m_displayModeFunc ? m_displayModeFunc() : "ASCII";

    // Format display
    QString html = formatDisplay(data, mode, timestamp, arrow);

    // Add client prefix if present
    if (!clientPrefix.isEmpty()) {
        html = QStringLiteral("<span style=\"color:#888;\">[%1]</span> ").arg(clientPrefix) + html;
    }

    // Append to text edit
    m_textEdit->append(html);

    // Auto scroll
    if (m_displayAnsiFunc && m_displayAnsiFunc()) {
        // Auto-scroll is handled by the caller
    }

    // Log
    if (m_logger && logType == "RECEIVE") {
        m_logger->log(QDateTime::currentDateTime(), logType,
                      DataHandler::bytesToHex(data),
                      DataHandler::bytesToAscii(data));
    }

    // Update counts
    if (logType == "RECEIVE") {
        m_receiveCount += data.size();
    } else if (logType == "SEND") {
        m_sendCount += data.size();
    }

    m_appendCount++;
    checkPrune();
}

void ReceiveDisplayHandler::appendEvent(const QString &text, const QString &color) {
    QString timestamp = QDateTime::currentDateTime().toString("HH:mm:ss.zzz");
    QString html = QStringLiteral("<span style=\"color:%1;\">[%2] %3</span>")
                   .arg(TIMESTAMP_COLOR, timestamp, text);

    if (color != "#000000") {
        html = QStringLiteral("<span style=\"color:%1;\">%2</span>").arg(color, html);
    }

    m_textEdit->append(html);

    if (m_logger) {
        m_logger->logEvent(text);
    }

    m_appendCount++;
    checkPrune();
}

QString ReceiveDisplayHandler::formatDisplay(const QByteArray &data, const QString &mode,
                                              const QString &timestamp, const QString &arrow) {
    bool showAnsi = m_displayAnsiFunc ? m_displayAnsiFunc() : false;

    // Build timestamp
    QString html = QStringLiteral("<span style=\"color:%1;\">[%2]</span> ")
                   .arg(TIMESTAMP_COLOR, timestamp);

    // Build arrow
    QString arrowColor = (arrow == "<-") ? "#00AA00" : "#0000AA";
    html += QStringLiteral("<span style=\"color:%1;font-weight:bold;\">%2</span> ")
            .arg(arrowColor, arrow);

    // Build data
    if (showAnsi && mode == "ASCII") {
        // ANSI color mode
        html += m_ansiParser->bytesToHtml(data, [](const QByteArray &d) {
            return DataHandler::bytesToAscii(d);
        });
    } else {
        QString display = DataHandler::formatDisplay(data, mode);
        html += QStringLiteral("<span style=\"color:%1;\">%2</span>")
                .arg(DATA_COLOR, AnsiParser::escapeHtml(display));
    }

    return html;
}

void ReceiveDisplayHandler::checkPrune() {
    if (m_appendCount % 50 == 0) {
        pruneIfNeeded();
    }
}

void ReceiveDisplayHandler::pruneIfNeeded() {
    QTextDocument *doc = m_textEdit->document();
    if (doc->blockCount() > MAX_DISPLAY_LINES) {
        QTextCursor cursor(doc);
        cursor.movePosition(QTextCursor::Start);
        cursor.movePosition(QTextCursor::Down, QTextCursor::MoveAnchor,
                           doc->blockCount() - DISPLAY_PRUNE_LINES);
        cursor.movePosition(QTextCursor::Start, QTextCursor::KeepAnchor);
        cursor.removeSelectedText();
    }
}

void ReceiveDisplayHandler::setupContextMenu(std::function<void()> toggleAnsiCallback) {
    m_textEdit->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(m_textEdit, &QTextEdit::customContextMenuRequested,
            [this, toggleAnsiCallback](const QPoint &pos) {
        QMenu menu;
        QAction *ansiAction = menu.addAction("ANSI Color Display");
        ansiAction->setCheckable(true);
        ansiAction->setChecked(m_displayAnsiFunc ? m_displayAnsiFunc() : false);
        connect(ansiAction, &QAction::triggered, [toggleAnsiCallback]() {
            if (toggleAnsiCallback) toggleAnsiCallback();
        });

        menu.addSeparator();
        menu.addAction("Clear", [this]() { m_textEdit->clear(); });

        menu.exec(m_textEdit->viewport()->mapToGlobal(pos));
    });
}
