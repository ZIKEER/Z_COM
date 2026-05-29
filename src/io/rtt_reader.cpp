#include "rtt_reader.h"
#include "rtt_manager.h"

#include <QDebug>

RttReaderThread::RttReaderThread(RttManager *manager, int bufferIdx,
                                 int readSize, int readIntervalMs,
                                 int frameTimeoutMs, QObject *parent)
    : QThread(parent)
    , m_manager(manager)
    , m_bufferIdx(bufferIdx)
    , m_readSize(readSize)
    , m_readIntervalMs(readIntervalMs)
{
    setFrameTimeout(frameTimeoutMs);
}

void RttReaderThread::setFrameTimeout(int timeoutMs) {
    m_frameTimeoutSec = timeoutMs / 1000.0;
}

void RttReaderThread::stop() {
    m_running = false;
    requestInterruption();
    emitBuffer();
    wait(1000);
}

void RttReaderThread::run() {
    qDebug() << "[RTT Reader] Thread started";

    QByteArray readBuf(m_readSize, 0);
    m_lastByteTimer.start();
    m_bufferStartTimer.start();

    while (m_running && !isInterruptionRequested()) {
        // Read from RTT buffer
        int bytesRead = m_manager->readRTT(m_bufferIdx, readBuf.data(), m_readSize);

        if (bytesRead > 0) {
            QMutexLocker locker(&m_bufferMutex);
            m_buffer.append(readBuf.constData(), bytesRead);
            m_lastByteTimer.restart();

            if (!m_bufferActive) {
                m_bufferActive = true;
                m_bufferStartTimer.restart();
            }

            // Emit if buffer is large enough
            if (m_buffer.size() >= EMIT_THRESHOLD) {
                QByteArray data = m_buffer;
                m_buffer.clear();
                m_bufferActive = false;
                locker.unlock();
                emit dataReceived(data);
            }
        } else {
            // No data, check frame timeout
            QMutexLocker locker(&m_bufferMutex);
            if (m_bufferActive && !m_buffer.isEmpty()) {
                double idleSec = m_lastByteTimer.elapsed() / 1000.0;
                double durationSec = m_bufferStartTimer.elapsed() / 1000.0;

                if (m_buffer.size() >= EMIT_THRESHOLD ||
                    idleSec >= m_frameTimeoutSec ||
                    durationSec >= m_frameTimeoutSec) {
                    emitBuffer();
                }
            }

            // Sleep to avoid busy waiting
            msleep(m_readIntervalMs);
        }
    }

    // Emit any remaining data
    emitBuffer();

    qDebug() << "[RTT Reader] Thread stopped";
}

void RttReaderThread::emitBuffer() {
    QMutexLocker locker(&m_bufferMutex);
    if (!m_buffer.isEmpty()) {
        QByteArray data = m_buffer;
        m_buffer.clear();
        m_bufferActive = false;
        locker.unlock();
        emit dataReceived(data);
    }
}
