#ifndef RTT_READER_H
#define RTT_READER_H

#include <QThread>
#include <QMutex>
#include <QByteArray>
#include <QElapsedTimer>

class RttManager;

/**
 * @brief RTT 读取线程 - 从 J-Link RTT 缓冲区读取数据
 */
class RttReaderThread : public QThread {
    Q_OBJECT

public:
    explicit RttReaderThread(RttManager *manager, int bufferIdx = 0,
                             int readSize = 4096, int readIntervalMs = 2,
                             int frameTimeoutMs = 50, QObject *parent = nullptr);

    void setFrameTimeout(int timeoutMs);
    void stop();

    static const int EMIT_THRESHOLD = 4096;

signals:
    void dataReceived(const QByteArray &data);
    void errorOccurred(const QString &error);

protected:
    void run() override;

private:
    void emitBuffer();

    RttManager *m_manager;
    int m_bufferIdx;
    int m_readSize;
    int m_readIntervalMs;
    double m_frameTimeoutSec;
    bool m_running = true;

    QByteArray m_buffer;
    QMutex m_bufferMutex;
    QElapsedTimer m_lastByteTimer;
    QElapsedTimer m_bufferStartTimer;
    bool m_bufferActive = false;
};

#endif // RTT_READER_H
