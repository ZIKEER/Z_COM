#ifndef SOCKET_READER_H
#define SOCKET_READER_H

#include <QThread>
#include <QMutex>
#include <QByteArray>
#include <QMap>
#include <QElapsedTimer>

class QTcpServer;
class QTcpSocket;
class QUdpSocket;
class QAbstractSocket;

class SocketReaderThread : public QThread {
    Q_OBJECT

public:
    // TCP Server mode
    explicit SocketReaderThread(QTcpServer *server, const QString &mode,
                                QObject *parent = nullptr);
    // TCP Client / UDP mode
    explicit SocketReaderThread(QAbstractSocket *socket, const QString &mode,
                                QObject *parent = nullptr);

    void setFrameTimeout(int timeoutMs);
    void stop();

    // TCP server: send to current client
    bool sendToCurrent(const QByteArray &data);
    bool sendToAll(const QByteArray &data);
    int clientCount() const;

    QPair<QString, int> currentClient() const { return m_currentClient; }

signals:
    void dataReceived(const QByteArray &data);
    void errorOccurred(const QString &error);
    void clientEvent(const QString &eventType, const QPair<QString, int> &address);

protected:
    void run() override;

private:
    void runTcpServer();
    void runTcpClient();
    void runUdp();

    void removeClient(qintptr fd);

    QString m_mode;
    QTcpServer *m_tcpServer = nullptr;
    QAbstractSocket *m_socket = nullptr;
    QUdpSocket *m_udpSocket = nullptr;

    double m_frameTimeoutSec = 0.05;
    bool m_running = true;

    // TCP server client management
    mutable QMutex m_clientsMutex;
    QMap<qintptr, QTcpSocket *> m_clients;
    QMap<qintptr, QPair<QString, int>> m_clientAddrs;
    QPair<QString, int> m_currentClient;

    QByteArray m_buffer;
    QMutex m_bufferMutex;
    QElapsedTimer m_lastByteTimer;
    QElapsedTimer m_bufferStartTimer;
    bool m_bufferActive = false;
};

#endif // SOCKET_READER_H
