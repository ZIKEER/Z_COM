#ifndef RTT_MANAGER_H
#define RTT_MANAGER_H

#include "io_transport.h"

// Forward declaration for J-Link SDK
// In a real build, this would include the J-Link SDK headers
// #include <JLinkARMDLL.h>

class RttManager : public IOTransport {
    Q_OBJECT

public:
    explicit RttManager(QObject *parent = nullptr);

    QList<QPair<QString, QString>> getAvailableDevices() override;

    // Get J-Link serial number if connected
    QString getSerialNumber() const;

protected:
    bool connectImpl(const QVariantMap &params) override;
    void closeResource() override;
    bool sendBytes(const QByteArray &data) override;

private:
    bool importJLink();

    void *m_jlink = nullptr; // Opaque pointer to J-Link object
    bool m_jlinkAvailable = false;
};

#endif // RTT_MANAGER_H
