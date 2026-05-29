#include "io_transport.h"

IOTransport::IOTransport(QObject *parent)
    : QObject(parent)
{
}

IOTransport::~IOTransport() {
    if (m_isConnected) {
        closeConnection();
    }
}

bool IOTransport::openConnection(const QVariantMap &params) {
    QMutexLocker locker(&m_mutex);
    if (m_isConnected) {
        // Disconnect first
        locker.unlock();
        closeConnection();
        locker.relock();
    }

    bool ok = connectImpl(params);
    if (ok) {
        m_isConnected = true;
        locker.unlock();
        emit connectionChanged(true);
    }
    return ok;
}

void IOTransport::closeConnection() {
    QMutexLocker locker(&m_mutex);
    if (!m_isConnected) return;

    stopReaderThread();
    closeResource();
    m_isConnected = false;
    locker.unlock();
    emit connectionChanged(false);
}

bool IOTransport::sendData(const QString &data, bool isHex) {
    if (!m_isConnected) return false;

    QByteArray bytes = parseSendData(data, isHex);
    if (bytes.isEmpty()) return false;

    return sendBytes(bytes);
}

void IOTransport::updateSettings(const QVariantMap &settings) {
    QMutexLocker locker(&m_mutex);
    // Merge settings
    for (auto it = settings.begin(); it != settings.end(); ++it) {
        m_settings[it.key()] = it.value();
    }
}

void IOTransport::startReaderThread(QThread *thread) {
    m_readerThread = thread;
    connect(thread, &QThread::finished, this, &IOTransport::onThreadFinished);
    thread->start();
}

void IOTransport::stopReaderThread() {
    if (!m_readerThread) return;

    QThread *thread = m_readerThread;
    m_readerThread = nullptr;

    // Request interruption and wait
    thread->requestInterruption();
    thread->quit();
    if (!thread->wait(2000)) {
        thread->terminate();
        thread->wait(1000);
    }
}

void IOTransport::onThreadFinished() {
    if (m_isConnected) {
        closeConnection();
    }
}

QByteArray IOTransport::parseSendData(const QString &data, bool isHex) {
    if (isHex) {
        QString cleaned = data;
        cleaned.remove(' ');
        cleaned.remove('\n');
        cleaned.remove('\r');
        return QByteArray::fromHex(cleaned.toLatin1());
    } else {
        return data.toUtf8();
    }
}
