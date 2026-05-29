#include "socket_reader.h"

#include <QTcpServer>
#include <QTcpSocket>
#include <QUdpSocket>

SocketReaderThread::SocketReaderThread(QTcpServer *server, const QString &mode, QObject *parent)
    : QThread(parent), m_mode(mode), m_tcpServer(server)
{
    m_udpSocket = qobject_cast<QUdpSocket *>(server);
}

SocketReaderThread::SocketReaderThread(QAbstractSocket *socket, const QString &mode, QObject *parent)
    : QThread(parent), m_mode(mode), m_socket(socket)
{
    m_udpSocket = qobject_cast<QUdpSocket *>(socket);
}

void SocketReaderThread::setFrameTimeout(int timeoutMs) {
    m_frameTimeoutSec = timeoutMs / 1000.0;
}

void SocketReaderThread::stop() {
    m_running = false;
    requestInterruption();

    // Close all clients
    {
        QMutexLocker locker(&m_clientsMutex);
        for (auto *client : m_clients) {
            client->close();
            client->deleteLater();
        }
        m_clients.clear();
    }

    if (m_socket) {
        m_socket->close();
    }

    quit();
    wait(2000);
}

bool SocketReaderThread::sendToCurrent(const QByteArray &data) {
    QMutexLocker locker(&m_clientsMutex);
    for (auto it = m_clients.begin(); it != m_clients.end(); ++it) {
        QPair<QString, int> addr = m_clientAddrs.value(it.key());
        if (addr == m_currentClient) {
            return it.value()->write(data) == data.size();
        }
    }
    return false;
}

bool SocketReaderThread::sendToAll(const QByteArray &data) {
    QMutexLocker locker(&m_clientsMutex);
    bool allOk = true;
    for (auto *client : m_clients) {
        if (client->write(data) != data.size()) {
            allOk = false;
        }
    }
    return allOk;
}

int SocketReaderThread::clientCount() const {
    QMutexLocker locker(&m_clientsMutex);
    return m_clients.size();
}

void SocketReaderThread::run() {
    if (m_mode == "tcp_server") {
        runTcpServer();
    } else if (m_mode == "tcp_client") {
        runTcpClient();
    } else {
        runUdp();
    }
}

void SocketReaderThread::runTcpServer() {
    if (!m_tcpServer) return;

    while (m_running && !isInterruptionRequested()) {
        // Accept new connections
        while (m_tcpServer->hasPendingConnections()) {
            QTcpSocket *client = m_tcpServer->nextPendingConnection();
            if (client) {
                qintptr fd = client->socketDescriptor();
                QString host = client->peerAddress().toString();
                int port = client->peerPort();

                {
                    QMutexLocker locker(&m_clientsMutex);
                    m_clients[fd] = client;
                    m_clientAddrs[fd] = {host, port};
                    if (m_currentClient.first.isEmpty()) {
                        m_currentClient = {host, port};
                    }
                }

                emit clientEvent("connected", {host, port});
            }
        }

        // Read from clients
        {
            QMutexLocker locker(&m_clientsMutex);
            for (auto it = m_clients.begin(); it != m_clients.end(); ) {
                QTcpSocket *client = it.value();
                if (client->state() != QAbstractSocket::ConnectedState) {
                    QPair<QString, int> addr = m_clientAddrs.value(it.key());
                    emit clientEvent("disconnected", addr);

                    // Switch current client
                    m_clientAddrs.remove(it.key());
                    client->deleteLater();
                    it = m_clients.erase(it);

                    if (!m_clients.isEmpty()) {
                        auto first = m_clients.begin();
                        m_currentClient = m_clientAddrs.value(first.key());
                    } else {
                        m_currentClient = {};
                    }
                    continue;
                }

                if (client->bytesAvailable() > 0) {
                    QByteArray data = client->readAll();
                    if (!data.isEmpty()) {
                        emit dataReceived(data);
                    }
                }
                ++it;
            }
        }

        msleep(10);
    }
}

void SocketReaderThread::runTcpClient() {
    if (!m_socket) return;

    while (m_running && !isInterruptionRequested()) {
        if (m_socket->state() != QAbstractSocket::ConnectedState) {
            emit errorOccurred("Connection lost");
            break;
        }

        if (m_socket->waitForReadyRead(10)) {
            QByteArray data = m_socket->readAll();
            if (!data.isEmpty()) {
                emit dataReceived(data);
            }
        }
    }
}

void SocketReaderThread::runUdp() {
    if (!m_udpSocket) return;

    while (m_running && !isInterruptionRequested()) {
        if (m_udpSocket->hasPendingDatagrams()) {
            QByteArray data;
            data.resize(m_udpSocket->pendingDatagramSize());
            QHostAddress sender;
            quint16 senderPort;
            m_udpSocket->readDatagram(data.data(), data.size(), &sender, &senderPort);

            // Store sender as current client for UDP server
            if (m_mode == "udp_server") {
                m_currentClient = {sender.toString(), senderPort};
            }

            emit dataReceived(data);
        } else {
            msleep(10);
        }
    }
}
