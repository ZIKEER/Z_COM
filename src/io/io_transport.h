#ifndef IO_TRANSPORT_H
#define IO_TRANSPORT_H

#include <QObject>
#include <QThread>
#include <QMutex>
#include <QVariantMap>
#include <QByteArray>

// Abstract base class for all transport modes
class IOTransport : public QObject {
    Q_OBJECT

public:
    explicit IOTransport(QObject *parent = nullptr);
    ~IOTransport() override;

    // Connection lifecycle
    bool openConnection(const QVariantMap &params = {});
    void closeConnection();
    bool isConnected() const { return m_isConnected; }

    // Send data
    bool sendData(const QString &data, bool isHex);

    // Update transport settings
    void updateSettings(const QVariantMap &settings);

    // Get available devices
    virtual QList<QPair<QString, QString>> getAvailableDevices() = 0;

signals:
    void dataReceived(const QByteArray &data);
    void connectionChanged(bool connected);
    void errorOccurred(const QString &error);

protected:
    // Subclass must implement these
    virtual bool connectImpl(const QVariantMap &params) = 0;
    virtual void closeResource() = 0;
    virtual bool sendBytes(const QByteArray &data) = 0;

    // Reader thread management - subclass connects signals before calling startReaderThread
    void startReaderThread(QThread *thread);
    void stopReaderThread();

    // Parse send data (hex string or UTF-8)
    static QByteArray parseSendData(const QString &data, bool isHex);

    bool m_isConnected = false;
    QThread *m_readerThread = nullptr;
    QMutex m_mutex;
    QVariantMap m_settings;

private slots:
    void onThreadFinished();
};

#endif // IO_TRANSPORT_H
