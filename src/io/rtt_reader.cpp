#include "rtt_reader.h"

RttReaderThread::RttReaderThread(void *jlink, int bufferIdx, int readSize,
                                 int readIntervalMs, int frameTimeoutMs, QObject *parent)
    : QThread(parent), m_jlink(jlink), m_bufferIdx(bufferIdx),
      m_readSize(readSize), m_readIntervalMs(readIntervalMs)
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
    m_lastByteTimer.start();
    m_bufferStartTimer.start();

    while (m_running && !isInterruptionRequested()) {
        // TODO: Read from J-Link RTT buffer
        // This would call jlink.rtt_read(bufferIdx, readSize)
        // For now, just sleep
        msleep(m_readIntervalMs);

        // Check frame timeout
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
    }

    emitBuffer();
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
