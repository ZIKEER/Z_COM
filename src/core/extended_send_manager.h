#ifndef EXTENDED_SEND_MANAGER_H
#define EXTENDED_SEND_MANAGER_H

#include <QObject>
#include <QTimer>
#include <QJsonObject>
#include <QJsonArray>
#include <functional>

struct SendItem {
    int id = 0;
    QString data;
    bool isHex = false;
    QString comment;
    int delay = 100;   // ms
    int sortOrder = 0; // 0 = excluded from sending
};

class ExtendedSendManager : public QObject {
    Q_OBJECT

public:
    using SendFunc = std::function<bool(const QByteArray &)>;

    explicit ExtendedSendManager(SendFunc sendFunc, const QString &configDir,
                                 QObject *parent = nullptr);

    // CRUD operations
    int addItem(const QString &data, bool isHex, const QString &comment, int delay);
    void removeItem(int itemId);
    void updateItem(int itemId, const QString &key, const QVariant &value);
    void moveItem(int itemId, int direction); // -1=up, +1=down
    void clearItems();

    // Get sorted items (sortOrder > 0)
    QList<SendItem> getSortedItems() const;

    // Get all items
    QList<SendItem> allItems() const { return m_items; }

    // Send operations
    void sendSingle(int itemId);
    void sendMultiple(bool loop);
    void stopSending();
    bool isSending() const { return m_sending; }

    // Persistence
    void flush();
    void importFromFile(const QString &filePath);
    void exportToFile(const QString &filePath) const;

signals:
    void sendStarted();
    void sendFinished();
    void sendProgress(int current, int total);
    void dataSent(const QByteArray &data);
    void errorOccurred(const QString &error);
    void itemsChanged();

private:
    void loadItems();
    void saveItems();
    void scheduleSave();
    void sendNextItem();
    bool sendItem(const SendItem &item);

    static int generateId();

    SendFunc m_sendFunc;
    QString m_configPath;
    QList<SendItem> m_items;
    QTimer *m_debounceTimer;

    // Send state
    bool m_sending = false;
    bool m_loopSend = false;
    QList<SendItem> m_sendingItems;
    int m_sendIndex = 0;
};

#endif // EXTENDED_SEND_MANAGER_H
