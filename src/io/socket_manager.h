#ifndef SOCKET_MANAGER_H
#define SOCKET_MANAGER_H

#include "io_transport.h"

#include <QTcpServer>
#include <QTcpSocket>
#include <QUdpSocket>

class SocketManager : public IOTransport {
    Q_OBJECT

public:
    explicit SocketManager(QObject *parent = nullptr);

    QList<QPair<QString, QString>> getAvailableDevices() override;

    // Get local IP addresses
    static QStringList getLocalIPs();

    // Current mode
    QString mode() const { return m_mode; }

    // Current client address (for TCP server)
    QPair<QString, int> currentClient() const;

signals:
    void clientEvent(const QString &eventType, const QPair<QString, int> &address);

protected:
    bool connectImpl(const QVariantMap &params) override;
    void closeResource() override;
    bool sendBytes(const QByteArray &data) override;

private:
    bool openSocket(const QString &host, int port, const QString &protocol, const QString &role);

    QString m_mode; // tcp_server, tcp_client, udp_server, udp_client
    QTcpServer *m_tcpServer = nullptr;
    QTcpSocket *m_tcpSocket = nullptr;
    QUdpSocket *m_udpSocket = nullptr;
    QPair<QString, int> m_remoteAddr;
    QPair<QString, int> m_currentClient;
};

#endif // SOCKET_MANAGER_H
