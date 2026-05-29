#include "serial_manager.h"
#include "serial_reader.h"

SerialManager::SerialManager(QObject *parent)
    : IOTransport(parent), m_serial(new QSerialPort(this))
{
}

QList<QPair<QString, QString>> SerialManager::getAvailablePorts() {
    QList<QPair<QString, QString>> ports;
    for (const QSerialPortInfo &info : QSerialPortInfo::availablePorts()) {
        ports.append({info.portName(), info.description()});
    }
    return ports;
}

QList<QPair<QString, QString>> SerialManager::getAvailableDevices() {
    return getAvailablePorts();
}

bool SerialManager::connectImpl(const QVariantMap &params) {
    QString port = params.value("port", m_settings.value("port")).toString();
    if (port.isEmpty()) {
        emit errorOccurred("No port specified");
        return false;
    }

    m_serial->setPortName(port);
    applySerialParams();

    if (!m_serial->open(QIODevice::ReadWrite)) {
        emit errorOccurred("Cannot open port: " + m_serial->errorString());
        return false;
    }

    // Create reader thread
    int frameTimeout = m_settings.value("frame_timeout", 50).toInt();
    auto *reader = new SerialReaderThread(m_serial, frameTimeout);
    connect(reader, &SerialReaderThread::dataReceived, this, &IOTransport::dataReceived);
    connect(reader, &SerialReaderThread::errorOccurred, this, &IOTransport::errorOccurred);
    startReaderThread(reader);

    return true;
}

void SerialManager::closeResource() {
    if (m_serial->isOpen()) {
        m_serial->close();
    }
}

bool SerialManager::sendBytes(const QByteArray &data) {
    if (!m_serial->isOpen()) return false;
    qint64 written = m_serial->write(data);
    return written == data.size();
}

void SerialManager::applySerialParams() {
    // Baudrate
    int baudrate = m_settings.value("baudrate", 115200).toInt();
    m_serial->setBaudRate(baudrate);

    // Data bits
    int databits = m_settings.value("databits", 8).toInt();
    switch (databits) {
    case 5: m_serial->setDataBits(QSerialPort::Data5); break;
    case 6: m_serial->setDataBits(QSerialPort::Data6); break;
    case 7: m_serial->setDataBits(QSerialPort::Data7); break;
    default: m_serial->setDataBits(QSerialPort::Data8); break;
    }

    // Stop bits
    float stopbits = m_settings.value("stopbits", 1.0).toFloat();
    if (stopbits == 1.5f) m_serial->setStopBits(QSerialPort::OneAndHalfStop);
    else if (stopbits == 2.0f) m_serial->setStopBits(QSerialPort::TwoStop);
    else m_serial->setStopBits(QSerialPort::OneStop);

    // Parity
    QString parity = m_settings.value("parity", "None").toString();
    if (parity == "Even") m_serial->setParity(QSerialPort::EvenParity);
    else if (parity == "Odd") m_serial->setParity(QSerialPort::OddParity);
    else if (parity == "Mark") m_serial->setParity(QSerialPort::MarkParity);
    else if (parity == "Space") m_serial->setParity(QSerialPort::SpaceParity);
    else m_serial->setParity(QSerialPort::NoParity);

    // Flow control
    QString fc = m_settings.value("flowcontrol", "None").toString();
    if (fc == "RTS-CTS") m_serial->setFlowControl(QSerialPort::HardwareControl);
    else if (fc == "XON-XOFF") m_serial->setFlowControl(QSerialPort::SoftwareControl);
    else m_serial->setFlowControl(QSerialPort::NoFlowControl);
}

void SerialManager::reconfigure() {
    if (m_serial->isOpen()) {
        applySerialParams();
    }
}
