#include "config_manager.h"

#include <QFile>
#include <QJsonDocument>
#include <QJsonArray>
#include <QStandardPaths>
#include <QCoreApplication>

ConfigManager::ConfigManager(const QString &configDir, int instanceId, QObject *parent)
    : QObject(parent)
{
    // Resolve config directory based on instance ID
    QString resolvedDir;
    if (instanceId <= 1) {
        resolvedDir = configDir;
    } else {
        resolvedDir = QCoreApplication::applicationDirPath()
                      + "/instance_" + QString::number(instanceId) + "/" + configDir;
    }

    QDir dir(resolvedDir);
    if (!dir.exists()) {
        dir.mkpath(".");
    }

    m_configPath = dir.absoluteFilePath("settings.json");

    // Debounce timer for frequent saves
    m_debounceTimer = new QTimer(this);
    m_debounceTimer->setSingleShot(true);
    m_debounceTimer->setInterval(500);
    connect(m_debounceTimer, &QTimer::timeout, this, &ConfigManager::saveConfig);

    loadConfig();
}

void ConfigManager::loadConfig() {
    QMutexLocker locker(&m_mutex);
    QFile file(m_configPath);
    if (!file.open(QIODevice::ReadOnly)) {
        // Use defaults
        m_config = QJsonObject();
        return;
    }

    QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
    if (doc.isObject()) {
        m_config = doc.object();
    }
}

void ConfigManager::saveConfig() {
    QMutexLocker locker(&m_mutex);
    QFile file(m_configPath);
    if (!file.open(QIODevice::WriteOnly)) {
        return;
    }
    file.write(QJsonDocument(m_config).toJson());
}

void ConfigManager::save() {
    saveConfig();
}

void ConfigManager::scheduleSave() {
    m_debounceTimer->start();
}

QVariant ConfigManager::get(const QString &key, const QVariant &defaultValue) const {
    QMutexLocker locker(&m_mutex);
    if (m_config.contains(key)) {
        return m_config.value(key).toVariant();
    }
    return defaultValue;
}

void ConfigManager::set(const QString &key, const QVariant &value) {
    QMutexLocker locker(&m_mutex);
    m_config.insert(key, QJsonValue::fromVariant(value));
    locker.unlock();

    emit configChanged(key, value);
    scheduleSave();
}

ConfigManager::SerialSettings ConfigManager::getSerialSettings() const {
    SerialSettings s;
    s.port = get("port", "").toString();
    s.baudrate = get("baudrate", 115200).toInt();
    s.databits = get("databits", 8).toInt();
    s.stopbits = get("stopbits", 1.0).toFloat();
    s.parity = get("parity", "None").toString();
    s.flowcontrol = get("flowcontrol", "None").toString();
    s.frameTimeout = get("frame_timeout", 50).toInt();
    return s;
}

ConfigManager::RttSettings ConfigManager::getRttSettings() const {
    RttSettings s;
    s.chip = get("rtt_chip", "").toString();
    s.speed = get("rtt_speed", 4000).toInt();
    s.reset = get("rtt_reset", false).toBool();
    s.startAddress = get("rtt_start_address", "").toString();
    s.rangeSize = get("rtt_range_size", "").toString();
    s.frameTimeout = get("frame_timeout", 50).toInt();

    // Load chip history
    QMutexLocker locker(&m_mutex);
    if (m_config.contains("rtt_chip_history")) {
        QJsonArray arr = m_config.value("rtt_chip_history").toArray();
        for (const auto &v : arr) {
            s.chipHistory.append(v.toString());
        }
    }
    return s;
}

void ConfigManager::addRttChipHistory(const QString &chip) {
    if (chip.isEmpty()) return;

    QMutexLocker locker(&m_mutex);
    QJsonArray arr;
    if (m_config.contains("rtt_chip_history")) {
        arr = m_config.value("rtt_chip_history").toArray();
    }

    // Remove duplicate if exists
    QJsonArray newArr;
    newArr.append(chip);
    for (const auto &v : arr) {
        if (v.toString() != chip) {
            newArr.append(v);
        }
    }

    // Limit to 20 entries
    while (newArr.size() > 20) {
        newArr.removeLast();
    }

    m_config.insert("rtt_chip_history", newArr);
    locker.unlock();
    scheduleSave();
}
