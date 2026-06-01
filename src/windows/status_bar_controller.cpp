#include "status_bar_controller.h"

StatusBarController::StatusBarController(QStatusBar *statusBar) {
    m_statusLabel = new QLabel("Disconnected");
    m_statusLabel->setStyleSheet("color: red; font-weight: bold;");
    statusBar->addPermanentWidget(m_statusLabel);

    m_sendCountLabel = new QLabel("发送: 0 字节");
    statusBar->addPermanentWidget(m_sendCountLabel);

    m_receiveCountLabel = new QLabel("接收: 0 字节");
    statusBar->addPermanentWidget(m_receiveCountLabel);
}

void StatusBarController::updateCounts(qint64 sendCount, qint64 receiveCount) {
    m_sendCountLabel->setText(QStringLiteral("发送: %1 字节").arg(sendCount));
    m_receiveCountLabel->setText(QStringLiteral("接收: %1 字节").arg(receiveCount));
}

void StatusBarController::setConnected(const QString &text) {
    m_statusLabel->setText(text);
    m_statusLabel->setStyleSheet("color: green; font-weight: bold;");
}

void StatusBarController::setDisconnected() {
    m_statusLabel->setText("Disconnected");
    m_statusLabel->setStyleSheet("color: red; font-weight: bold;");
}
