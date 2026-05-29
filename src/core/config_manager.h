#ifndef CONFIG_MANAGER_H
#define CONFIG_MANAGER_H

#include <QObject>
#include <QJsonObject>
#include <QJsonArray>
#include <QTimer>
#include <QDir>
#include <QMutex>

class ConfigManager : public QObject {
    Q_OBJECT

public:
    explicit ConfigManager(const QString &configDir, int instanceId = 1,
                           QObject *parent = nullptr);

    // Generic config access
    QVariant get(const QString &key, const QVariant &defaultValue = QVariant()) const;
    void set(const QString &key, const QVariant &value);
    void save();

    // Serial settings
    struct SerialSettings {
        QString port;
        int baudrate = 115200;
        int databits = 8;
        float stopbits = 1.0f;
        QString parity = "None";
        QString flowcontrol = "None";
        int frameTimeout = 50;
    };
    SerialSettings getSerialSettings() const;

    // RTT settings
    struct RttSettings {
        QString chip;
        int speed = 4000;
        bool reset = false;
        QString startAddress;
        QString rangeSize;
        QStringList chipHistory;
        int frameTimeout = 50;
    };
    RttSettings getRttSettings() const;
    void addRttChipHistory(const QString &chip);

signals:
    void configChanged(const QString &key, const QVariant &value);

private:
    void loadConfig();
    void saveConfig();
    void scheduleSave();

    QString m_configPath;
    QJsonObject m_config;
    QTimer *m_debounceTimer;
    mutable QMutex m_mutex;
};

#endif // CONFIG_MANAGER_H
