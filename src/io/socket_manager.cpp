#include "socket_manager.h"
#include "socket_reader.h"

#include <QNetworkInterface>
#include <QHostAddress>

SocketManager::SocketManager(QObject *parent)
    : IOTransport(parent)
{
}

QStringList SocketManager::getLocalIPs() {
    QStringList ips;
    ips << "0.0.0.0" << "127.0.0.1";

    for (const QNetworkInterface &iface : QNetworkInterface::allInterfaces()) {
        if (!(iface.flags() & QNetworkInterface::IsUp)) continue;
        if (iface.flags() & QNetworkInterface::IsLoopBack) continue;

        for (const QNetworkAddressEntry &entry : iface.addressEntries()) {
            if (entry.ip().protocol() == QAbstractSocket::IPv4Protocol) {
                QString ip = entry.ip().toString();
                if (!ip.startsWith("127.") && !ips.contains(ip)) {
                    ips << ip;
                }
            }
        }
    }
    return ips;
}

QList<QPair<QString, QString>> SocketManager::getAvailableDevices() {
    QList<QPair<QString, QString>> devices;
    for (const QString &ip : getLocalIPs()) {
        devices.append({ip, ip});
    }
    return devices;
}

QPair<QString, int> SocketManager::currentClient() const {
    return m_currentClient;
}

bool SocketManager::connectImpl(const QVariantMap &params) {
    QString host = params.value("host", "127.0.0.1").toString();
    int port = params.value("port", 8080).toInt();
    QString protocol = params.value("protocol", "tcp").toString();
    QString role = params.value("role", "client").toString();

    return openSocket(host, port, protocol, role);
}

bool SocketManager::openSocket(const QString &host, int port,
                                const QString &protocol, const QString &role) {
    closeResource();

    if (protocol == "tcp") {
        if (role == "server") {
            m_mode = "tcp_server";
            m_tcpServer = new QTcpServer(this);

            if (!m_tcpServer->listen(QHostAddress::Any, port)) {
                emit errorOccurred("TCP Server listen failed: " + m_tcpServer->errorString());
                return false;
            }

            // Create reader thread for server
            auto *reader = new SocketReaderThread(m_tcpServer, m_mode, this);
            connect(reader, &SocketReaderThread::clientEvent,
                    this, &SocketManager::clientEvent);
            connect(reader, &SocketReaderThread::dataReceived, this, &IOTransport::dataReceived);
            connect(reader, &SocketReaderThread::errorOccurred, this, &IOTransport::errorOccurred);
            startReaderThread(reader);

            return true;

        } else {
            m_mode = "tcp_client";
            m_tcpSocket = new QTcpSocket(this);
            m_tcpSocket->connectToHost(host, port);

            if (!m_tcpSocket->waitForConnected(5000)) {
                emit errorOccurred("TCP connect failed: " + m_tcpSocket->errorString());
                return false;
            }

            // Create reader thread for client
            auto *reader = new SocketReaderThread(m_tcpSocket, m_mode, this);
            connect(reader, &SocketReaderThread::dataReceived, this, &IOTransport::dataReceived);
            connect(reader, &SocketReaderThread::errorOccurred, this, &IOTransport::errorOccurred);
            startReaderThread(reader);

            return true;
        }
    } else { // UDP
        m_mode = (role == "server") ? "udp_server" : "udp_client";
        m_udpSocket = new QUdpSocket(this);

        if (m_mode == "udp_server") {
            if (!m_udpSocket->bind(QHostAddress::Any, port)) {
                emit errorOccurred("UDP bind failed");
                return false;
            }
        } else {
            m_remoteAddr = {host, port};
        }

        // Create reader thread for UDP
        auto *reader = new SocketReaderThread(m_udpSocket, m_mode, this);
        connect(reader, &SocketReaderThread::dataReceived, this, &IOTransport::dataReceived);
        connect(reader, &SocketReaderThread::errorOccurred, this, &IOTransport::errorOccurred);
        startReaderThread(reader);

        return true;
    }
}

void SocketManager::closeResource() {
    if (m_tcpServer) {
        m_tcpServer->close();
        delete m_tcpServer;
        m_tcpServer = nullptr;
    }
    if (m_tcpSocket) {
        m_tcpSocket->disconnectFromHost();
        if (m_tcpSocket->state() != QAbstractSocket::UnconnectedState) {
            m_tcpSocket->waitForDisconnected(1000);
        }
        delete m_tcpSocket;
        m_tcpSocket = nullptr;
    }
    if (m_udpSocket) {
        m_udpSocket->close();
        delete m_udpSocket;
        m_udpSocket = nullptr;
    }
    m_mode.clear();
}

bool SocketManager::sendBytes(const QByteArray &data) {
    if (m_mode == "tcp_client") {
        if (!m_tcpSocket) return false;
        return m_tcpSocket->write(data) == data.size();

    } else if (m_mode == "tcp_server") {
        // Delegate to reader thread's send_to_current
        // For now, find the reader thread and call sendToCurrent
        if (m_readerThread) {
            auto *reader = qobject_cast<SocketReaderThread *>(m_readerThread);
            if (reader) {
                return reader->sendToCurrent(data);
            }
        }
        return false;

    } else if (m_mode == "udp_client") {
        if (!m_udpSocket) return false;
        qint64 sent = m_udpSocket->writeDatagram(data,
            QHostAddress(m_remoteAddr.first), m_remoteAddr.second);
        return sent == data.size();

    } else if (m_mode == "udp_server") {
        if (!m_udpSocket) return false;
        qint64 sent = m_udpSocket->writeDatagram(data,
            QHostAddress(m_currentClient.first), m_currentClient.second);
        return sent == data.size();
    }

    return false;
}
