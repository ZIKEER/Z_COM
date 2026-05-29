#include "extended_send_manager.h"

#include <QFile>
#include <QJsonDocument>
#include <QDir>

ExtendedSendManager::ExtendedSendManager(SendFunc sendFunc, const QString &configDir,
                                         QObject *parent)
    : QObject(parent), m_sendFunc(std::move(sendFunc))
{
    QDir dir(configDir);
    if (!dir.exists()) {
        dir.mkpath(".");
    }
    m_configPath = dir.absoluteFilePath("extended_send.json");

    m_debounceTimer = new QTimer(this);
    m_debounceTimer->setSingleShot(true);
    m_debounceTimer->setInterval(500);
    connect(m_debounceTimer, &QTimer::timeout, this, &ExtendedSendManager::saveItems);

    loadItems();
}

int ExtendedSendManager::generateId() {
    static int counter = 0;
    return ++counter;
}

void ExtendedSendManager::loadItems() {
    QFile file(m_configPath);
    if (!file.open(QIODevice::ReadOnly)) return;

    QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
    if (!doc.isObject()) return;

    QJsonObject root = doc.object();
    QJsonArray items = root.value("items").toArray();

    int maxId = 0;
    m_items.clear();
    for (const auto &v : items) {
        QJsonObject obj = v.toObject();
        SendItem item;
        item.id = obj.value("id").toInt();
        item.data = obj.value("data").toString();
        item.isHex = obj.value("is_hex").toBool();
        item.comment = obj.value("comment").toString();
        item.delay = obj.value("delay").toInt(100);
        item.sortOrder = obj.value("sort_order").toInt(0);
        m_items.append(item);
        if (item.id > maxId) maxId = item.id;
    }

    // Update static counter
    static int *counterPtr = nullptr;
    if (!counterPtr) {
        static int counter = 0;
        counterPtr = &counter;
    }
    *counterPtr = qMax(*counterPtr, maxId);
}

void ExtendedSendManager::saveItems() {
    QJsonArray items;
    for (const auto &item : m_items) {
        QJsonObject obj;
        obj["id"] = item.id;
        obj["data"] = item.data;
        obj["is_hex"] = item.isHex;
        obj["comment"] = item.comment;
        obj["delay"] = item.delay;
        obj["sort_order"] = item.sortOrder;
        items.append(obj);
    }

    QJsonObject root;
    root["items"] = items;

    QFile file(m_configPath);
    if (file.open(QIODevice::WriteOnly)) {
        file.write(QJsonDocument(root).toJson());
    }
}

void ExtendedSendManager::scheduleSave() {
    m_debounceTimer->start();
}

void ExtendedSendManager::flush() {
    if (m_debounceTimer->isActive()) {
        m_debounceTimer->stop();
        saveItems();
    }
}

int ExtendedSendManager::addItem(const QString &data, bool isHex,
                                  const QString &comment, int delay) {
    SendItem item;
    item.id = generateId();
    item.data = data;
    item.isHex = isHex;
    item.comment = comment;
    item.delay = delay;
    item.sortOrder = m_items.size() + 1;

    m_items.append(item);
    emit itemsChanged();
    scheduleSave();
    return item.id;
}

void ExtendedSendManager::removeItem(int itemId) {
    for (int i = 0; i < m_items.size(); ++i) {
        if (m_items[i].id == itemId) {
            m_items.removeAt(i);
            emit itemsChanged();
            scheduleSave();
            return;
        }
    }
}

void ExtendedSendManager::updateItem(int itemId, const QString &key, const QVariant &value) {
    for (auto &item : m_items) {
        if (item.id == itemId) {
            if (key == "data") item.data = value.toString();
            else if (key == "is_hex") item.isHex = value.toBool();
            else if (key == "comment") item.comment = value.toString();
            else if (key == "delay") item.delay = value.toInt();
            else if (key == "sort_order") item.sortOrder = value.toInt();

            emit itemsChanged();
            scheduleSave();
            return;
        }
    }
}

void ExtendedSendManager::moveItem(int itemId, int direction) {
    for (int i = 0; i < m_items.size(); ++i) {
        if (m_items[i].id == itemId) {
            int newIdx = i + direction;
            if (newIdx >= 0 && newIdx < m_items.size()) {
                m_items.swapItemsAt(i, newIdx);
                // Update sort orders
                for (int j = 0; j < m_items.size(); ++j) {
                    m_items[j].sortOrder = j + 1;
                }
                emit itemsChanged();
                scheduleSave();
            }
            return;
        }
    }
}

void ExtendedSendManager::clearItems() {
    m_items.clear();
    emit itemsChanged();
    scheduleSave();
}

QList<SendItem> ExtendedSendManager::getSortedItems() const {
    QList<SendItem> sorted;
    for (const auto &item : m_items) {
        if (item.sortOrder > 0) {
            sorted.append(item);
        }
    }
    std::sort(sorted.begin(), sorted.end(), [](const SendItem &a, const SendItem &b) {
        return a.sortOrder < b.sortOrder;
    });
    return sorted;
}

void ExtendedSendManager::sendSingle(int itemId) {
    for (const auto &item : m_items) {
        if (item.id == itemId) {
            sendItem(item);
            return;
        }
    }
}

void ExtendedSendManager::sendMultiple(bool loop) {
    if (m_sending) return;

    m_sendingItems = getSortedItems();
    if (m_sendingItems.isEmpty()) {
        emit errorOccurred("No items to send");
        return;
    }

    m_sending = true;
    m_loopSend = loop;
    m_sendIndex = 0;
    emit sendStarted();
    sendNextItem();
}

void ExtendedSendManager::sendNextItem() {
    if (!m_sending || m_sendIndex >= m_sendingItems.size()) {
        if (m_loopSend && m_sending) {
            // Loop: restart
            m_sendIndex = 0;
            emit sendProgress(0, m_sendingItems.size());
            sendNextItem();
            return;
        }
        // Done
        m_sending = false;
        emit sendFinished();
        return;
    }

    emit sendProgress(m_sendIndex + 1, m_sendingItems.size());

    const SendItem &item = m_sendingItems[m_sendIndex];
    int delay = (m_sendIndex == 0) ? 0 : item.delay;

    QTimer::singleShot(delay, this, [this, item]() {
        if (!m_sending) return;
        sendItem(item);
        m_sendIndex++;
        sendNextItem();
    });
}

void ExtendedSendManager::stopSending() {
    m_sending = false;
    m_loopSend = false;
    emit sendFinished();
}

bool ExtendedSendManager::sendItem(const SendItem &item) {
    QByteArray data;
    if (item.isHex) {
        // Validate hex string
        QString cleaned = item.data;
        cleaned.remove(' ');
        if (cleaned.size() % 2 != 0) {
            emit errorOccurred("Invalid hex data (odd length): " + item.data);
            return false;
        }
        for (const QChar &ch : cleaned) {
            if (!ch.isLetterOrNumber()) {
                emit errorOccurred("Invalid hex data: " + item.data);
                return false;
            }
        }
        data = QByteArray::fromHex(cleaned.toLatin1());
    } else {
        // Decode ASCII escapes
        QString decoded = item.data;
        decoded.replace("\\r", "\r");
        decoded.replace("\\n", "\n");
        decoded.replace("\\t", "\t");
        decoded.replace("\\0", "\0");
        decoded.replace("\\\\", "\\");
        data = decoded.toUtf8();
    }

    if (m_sendFunc) {
        bool ok = m_sendFunc(data);
        if (ok) {
            emit dataSent(data);
        }
        return ok;
    }
    return false;
}

void ExtendedSendManager::importFromFile(const QString &filePath) {
    QFile file(filePath);
    if (!file.open(QIODevice::ReadOnly)) {
        emit errorOccurred("Cannot open file: " + filePath);
        return;
    }

    QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
    if (!doc.isObject()) {
        emit errorOccurred("Invalid JSON format");
        return;
    }

    QJsonObject root = doc.object();
    QJsonArray items = root.value("items").toArray();

    m_items.clear();
    for (const auto &v : items) {
        QJsonObject obj = v.toObject();
        SendItem item;
        item.id = obj.value("id").toInt();
        item.data = obj.value("data").toString();
        item.isHex = obj.value("is_hex").toBool();
        item.comment = obj.value("comment").toString();
        item.delay = obj.value("delay").toInt(100);
        item.sortOrder = obj.value("sort_order").toInt(0);
        m_items.append(item);
    }

    emit itemsChanged();
    saveItems();
}

void ExtendedSendManager::exportToFile(const QString &filePath) const {
    QJsonArray items;
    for (const auto &item : m_items) {
        QJsonObject obj;
        obj["id"] = item.id;
        obj["data"] = item.data;
        obj["is_hex"] = item.isHex;
        obj["comment"] = item.comment;
        obj["delay"] = item.delay;
        obj["sort_order"] = item.sortOrder;
        items.append(obj);
    }

    QJsonObject root;
    root["items"] = items;

    QFile file(filePath);
    if (file.open(QIODevice::WriteOnly)) {
        file.write(QJsonDocument(root).toJson());
    }
}
