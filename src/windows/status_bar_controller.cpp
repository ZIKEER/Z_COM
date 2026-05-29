#include "status_bar_controller.h"

StatusBarController::StatusBarController(QStatusBar *statusBar) {
    m_statusLabel = new QLabel("Disconnected");
    m_statusLabel->setStyleSheet("color: red; font-weight: bold;");
    statusBar->addPermanentWidget(m_statusLabel);

    m_sendCountLabel = new QLabel("TX: 0");
    statusBar->addPermanentWidget(m_sendCountLabel);

    m_receiveCountLabel = new QLabel("RX: 0");
    statusBar->addPermanentWidget(m_receiveCountLabel);
}

void StatusBarController::updateCounts(qint64 sendCount, qint64 receiveCount) {
    m_sendCountLabel->setText(QStringLiteral("TX: %1").arg(sendCount));
    m_receiveCountLabel->setText(QStringLiteral("RX: %1").arg(receiveCount));
}

void StatusBarController::setConnected(const QString &text) {
    m_statusLabel->setText(text);
    m_statusLabel->setStyleSheet("color: green; font-weight: bold;");
}

void StatusBarController::setDisconnected() {
    m_statusLabel->setText("Disconnected");
    m_statusLabel->setStyleSheet("color: red; font-weight: bold;");
}
