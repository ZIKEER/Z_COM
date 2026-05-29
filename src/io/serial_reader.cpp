#include "serial_reader.h"

#include <QSerialPort>

SerialReaderThread::SerialReaderThread(QSerialPort *serial, int frameTimeoutMs, QObject *parent)
    : QThread(parent), m_serial(serial)
{
    setFrameTimeout(frameTimeoutMs);
}

void SerialReaderThread::setFrameTimeout(int timeoutMs) {
    m_frameTimeoutSec = timeoutMs / 1000.0;
}

void SerialReaderThread::stop() {
    m_running = false;
    requestInterruption();
    emitBuffer();
    wait(1000);
}

void SerialReaderThread::run() {
    m_lastByteTimer.start();
    m_bufferStartTimer.start();

    while (m_running && !isInterruptionRequested()) {
        if (!m_serial->isOpen()) {
            msleep(50);
            continue;
        }

        // Read available data
        if (m_serial->waitForReadyRead(10)) {
            QByteArray data = m_serial->readAll();
            if (!data.isEmpty()) {
                QMutexLocker locker(&m_bufferMutex);
                m_buffer.append(data);
                m_lastByteTimer.start();

                if (!m_bufferActive) {
                    m_bufferActive = true;
                    m_bufferStartTimer.start();
                }
            }
        } else {
            // No data available, check frame timeout
            QMutexLocker locker(&m_bufferMutex);
            if (m_bufferActive && !m_buffer.isEmpty()) {
                double idleSec = m_lastByteTimer.elapsed() / 1000.0;
                double durationSec = m_bufferStartTimer.elapsed() / 1000.0;

                // Emit if idle gap exceeds timeout or continuous stream duration >= timeout
                if (idleSec >= m_frameTimeoutSec || durationSec >= m_frameTimeoutSec) {
                    emitBuffer();
                }
            }
            locker.unlock();
            msleep(10);
        }
    }

    emitBuffer();
}

void SerialReaderThread::emitBuffer() {
    QMutexLocker locker(&m_bufferMutex);
    if (!m_buffer.isEmpty()) {
        QByteArray data = m_buffer;
        m_buffer.clear();
        m_bufferActive = false;
        locker.unlock();
        emit dataReceived(data);
    }
}
