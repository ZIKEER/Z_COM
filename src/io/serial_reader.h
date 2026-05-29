#ifndef SERIAL_READER_H
#define SERIAL_READER_H

#include <QThread>
#include <QMutex>
#include <QByteArray>
#include <QElapsedTimer>

class QSerialPort;

class SerialReaderThread : public QThread {
    Q_OBJECT

public:
    explicit SerialReaderThread(QSerialPort *serial, int frameTimeoutMs,
                                QObject *parent = nullptr);

    void setFrameTimeout(int timeoutMs);
    void stop();

signals:
    void dataReceived(const QByteArray &data);
    void errorOccurred(const QString &error);

protected:
    void run() override;

private:
    void emitBuffer();

    QSerialPort *m_serial;
    double m_frameTimeoutSec;
    bool m_running = true;

    QByteArray m_buffer;
    QMutex m_bufferMutex;
    QElapsedTimer m_lastByteTimer;
    QElapsedTimer m_bufferStartTimer;
    bool m_bufferActive = false;
};

#endif // SERIAL_READER_H
