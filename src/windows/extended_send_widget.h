#ifndef EXTENDED_SEND_WIDGET_H
#define EXTENDED_SEND_WIDGET_H

#include <QWidget>
#include <QTableWidget>
#include <QLineEdit>
#include <QPushButton>

#include "core/extended_send_manager.h"

// Custom widget for one send item row
class SendItemWidget : public QWidget {
    Q_OBJECT

public:
    explicit SendItemWidget(const SendItem &item, QWidget *parent = nullptr);

    void setData(const QString &data);
    void setComment(const QString &comment);

signals:
    void sendClicked(int itemId);
    void dataChanged(int itemId, const QString &data);
    void commentChanged(int itemId, const QString &comment);

private:
    void buildCommentButtonContent(const QString &comment);
    void showDataContextMenu(const QPoint &pos);
    void openAdvancedEditor();
    void editComment();

    SendItem m_item;
    QLineEdit *m_dataEdit;
    QPushButton *m_commentButton;
};

// The extended send panel
class ExtendedSendWidget : public QWidget {
    Q_OBJECT

public:
    explicit ExtendedSendWidget(ExtendedSendManager *manager, QWidget *parent = nullptr);

    void refreshTable();

signals:
    void sendData(const QByteArray &data);

private slots:
    void addItem();
    void onDeleteClicked();
    void onMoveUpClicked();
    void onMoveDownClicked();
    void onStartSendClicked();

private:
    void setupContextMenu();

    ExtendedSendManager *m_manager;
    QTableWidget *m_table;
    QPushButton *m_deleteButton = nullptr;
    QPushButton *m_moveUpButton = nullptr;
    QPushButton *m_moveDownButton = nullptr;

    enum class OperationMode { Normal, Delete, MoveUp, MoveDown };
    OperationMode m_operationMode = OperationMode::Normal;

    void exitOperationMode();
    QList<int> getSelectedItemIds() const;
};

#endif // EXTENDED_SEND_WIDGET_H
