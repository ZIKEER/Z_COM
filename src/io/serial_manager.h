#ifndef SERIAL_MANAGER_H
#define SERIAL_MANAGER_H

#include "io_transport.h"

#include <QSerialPort>
#include <QSerialPortInfo>

class SerialManager : public IOTransport {
    Q_OBJECT

public:
    explicit SerialManager(QObject *parent = nullptr);

    // Get available serial ports
    static QList<QPair<QString, QString>> getAvailablePorts();
    QList<QPair<QString, QString>> getAvailableDevices() override;

    // Reconfigure port without reconnecting
    void reconfigure();

    // Get/set serial port object (for reader thread)
    QSerialPort *serialPort() const { return m_serial; }

protected:
    bool connectImpl(const QVariantMap &params) override;
    void closeResource() override;
    bool sendBytes(const QByteArray &data) override;

private:
    void applySerialParams();

    QSerialPort *m_serial;
};

#endif // SERIAL_MANAGER_H
