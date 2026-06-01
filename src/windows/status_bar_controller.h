#ifndef STATUS_BAR_CONTROLLER_H
#define STATUS_BAR_CONTROLLER_H

#include <QLabel>
#include <QStatusBar>

class StatusBarController {
public:
    explicit StatusBarController(QStatusBar *statusBar);

    void updateCounts(qint64 sendCount, qint64 receiveCount);
    void setConnected(const QString &text);
    void setDisconnected();

    // Display constants
    static const int MAX_DISPLAY_LINES = 10000;
    static const int DISPLAY_PRUNE_LINES = 5000;

private:
    QLabel *m_statusLabel;
    QLabel *m_sendCountLabel;
    QLabel *m_receiveCountLabel;
};

#endif // STATUS_BAR_CONTROLLER_H
