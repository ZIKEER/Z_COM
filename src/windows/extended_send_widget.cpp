#include "extended_send_widget.h"
#include "extended_send_editor_dialog.h"
#include "ui_extended_send_widget.h"

#include <QHBoxLayout>
#include <QHeaderView>
#include <QMenu>
#include <QAction>
#include <QInputDialog>
#include <QFileDialog>
#include <QMessageBox>
#include <QCheckBox>

// SendItemWidget implementation
SendItemWidget::SendItemWidget(const SendItem &item, QWidget *parent)
    : QWidget(parent), m_item(item)
{
    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(2, 2, 2, 2);
    layout->setSpacing(4);

    m_dataEdit = new QLineEdit(item.data);
    m_dataEdit->setPlaceholderText("Data...");
    connect(m_dataEdit, &QLineEdit::textChanged, [this](const QString &text) {
        emit dataChanged(m_item.id, text);
    });

    m_commentButton = new QPushButton;
    buildCommentButtonContent(item.comment);
    connect(m_commentButton, &QPushButton::clicked, this, &SendItemWidget::editComment);

    layout->addWidget(m_dataEdit, 1);
    layout->addWidget(m_commentButton);

    // Context menu for data edit
    m_dataEdit->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(m_dataEdit, &QLineEdit::customContextMenuRequested,
            this, &SendItemWidget::showDataContextMenu);
}

void SendItemWidget::setData(const QString &data) {
    m_dataEdit->setText(data);
}

void SendItemWidget::setComment(const QString &comment) {
    m_item.comment = comment;
    buildCommentButtonContent(comment);
}

void SendItemWidget::buildCommentButtonContent(const QString &comment) {
    if (comment.isEmpty()) {
        m_commentButton->setText("Send");
        m_commentButton->setToolTip("Click to send, right-click to edit comment");
        m_commentButton->setMinimumWidth(60);
        return;
    }

    QString display = comment;
    if (display.length() > 16) {
        display = display.left(16) + "...";
    }

    // Auto-split for long comments
    if (display.length() > 8) {
        int mid = display.length() / 2;
        QString line1 = display.left(mid);
        QString line2 = display.mid(mid);
        m_commentButton->setText(line1 + "\n" + line2);
    } else {
        m_commentButton->setText(display);
    }

    m_commentButton->setToolTip(comment);
    m_commentButton->setMinimumWidth(60);
}

void SendItemWidget::showDataContextMenu(const QPoint &pos) {
    QMenu menu;
    menu.addAction(QStringLiteral("高级编辑"), this, &SendItemWidget::openAdvancedEditor);
    menu.addSeparator();
    menu.addAction(QStringLiteral("剪切"), m_dataEdit, &QLineEdit::cut);
    menu.addAction(QStringLiteral("复制"), m_dataEdit, &QLineEdit::copy);
    menu.addAction(QStringLiteral("粘贴"), m_dataEdit, &QLineEdit::paste);
    menu.exec(m_dataEdit->mapToGlobal(pos));
}

void SendItemWidget::openAdvancedEditor() {
    ExtendedSendEditorDialog dialog(m_dataEdit->text(), this);
    if (dialog.exec() == QDialog::Accepted) {
        m_dataEdit->setText(dialog.getText());
    }
}

void SendItemWidget::editComment() {
    bool ok;
    QString comment = QInputDialog::getText(this, QStringLiteral("编辑备注"),
                                            QStringLiteral("备注:"), QLineEdit::Normal,
                                            m_item.comment, &ok);
    if (ok) {
        setComment(comment);
        emit commentChanged(m_item.id, comment);
    }
}

// ExtendedSendWidget implementation
ExtendedSendWidget::ExtendedSendWidget(ExtendedSendManager *manager, QWidget *parent)
    : QWidget(parent), m_manager(manager)
{
    auto *ui_form = new Ui::ExtendedSendWidget;
    ui_form->setupUi(this);

    m_table = ui_form->dataTable;

    // Setup table
    m_table->setColumnCount(4);
    m_table->setHorizontalHeaderLabels({"HEX", "Data/Comment", "Order", "Delay"});
    m_table->horizontalHeader()->setStretchLastSection(true);
    m_table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    m_table->verticalHeader()->setVisible(false);
    m_table->setSelectionBehavior(QAbstractItemView::SelectRows);

    // Store button references for mode switching
    m_deleteButton = ui_form->deleteButton;
    m_moveUpButton = ui_form->moveUpButton;
    m_moveDownButton = ui_form->moveDownButton;

    // Connect buttons
    connect(ui_form->addButton, &QPushButton::clicked, this, &ExtendedSendWidget::addItem);
    connect(m_deleteButton, &QPushButton::clicked, this, &ExtendedSendWidget::onDeleteClicked);
    connect(m_moveUpButton, &QPushButton::clicked, this, &ExtendedSendWidget::onMoveUpClicked);
    connect(m_moveDownButton, &QPushButton::clicked, this, &ExtendedSendWidget::onMoveDownClicked);
    connect(ui_form->startSendButton, &QPushButton::clicked, this, &ExtendedSendWidget::onStartSendClicked);

    // Manager signals
    connect(m_manager, &ExtendedSendManager::itemsChanged, this, &ExtendedSendWidget::refreshTable);
    connect(m_manager, &ExtendedSendManager::dataSent, this, &ExtendedSendWidget::sendData);

    setupContextMenu();
    refreshTable();
}

void ExtendedSendWidget::refreshTable() {
    m_table->setRowCount(0);

    bool inOpMode = (m_operationMode != OperationMode::Normal);
    int colOffset = inOpMode ? 1 : 0;

    if (inOpMode) {
        m_table->setColumnCount(5);
        m_table->setHorizontalHeaderLabels({"Select", "HEX", "Data/Comment", "Order", "Delay"});
    } else {
        m_table->setColumnCount(4);
        m_table->setHorizontalHeaderLabels({"HEX", "Data/Comment", "Order", "Delay"});
    }

    QList<SendItem> items = m_manager->allItems();
    for (const SendItem &item : items) {
        int row = m_table->rowCount();
        m_table->insertRow(row);

        // Selection checkbox (only in operation mode)
        if (inOpMode) {
            auto *selCheck = new QCheckBox;
            m_table->setCellWidget(row, 0, selCheck);
        }

        // HEX checkbox
        auto *hexCheck = new QCheckBox;
        hexCheck->setChecked(item.isHex);
        m_table->setCellWidget(row, 0 + colOffset, hexCheck);

        // Data/Comment widget
        auto *itemWidget = new SendItemWidget(item);
        m_table->setCellWidget(row, 1 + colOffset, itemWidget);

        // Order
        auto *orderItem = new QTableWidgetItem(QString::number(item.sortOrder));
        m_table->setItem(row, 2 + colOffset, orderItem);

        // Delay
        auto *delayItem = new QTableWidgetItem(QString::number(item.delay));
        m_table->setItem(row, 3 + colOffset, delayItem);
    }
}

void ExtendedSendWidget::addItem() {
    m_manager->addItem("", false, "", 100);
}

void ExtendedSendWidget::onDeleteClicked() {
    if (m_operationMode != OperationMode::Delete) {
        exitOperationMode();
        m_operationMode = OperationMode::Delete;
        m_deleteButton->setText(QStringLiteral("确认删除"));
        m_deleteButton->setStyleSheet("background-color: #FF6B6B; color: white;");
        refreshTable();
        return;
    }

    // Second click: delete selected items
    QList<int> ids = getSelectedItemIds();
    if (ids.isEmpty()) {
        exitOperationMode();
        return;
    }

    for (int id : ids) {
        m_manager->removeItem(id);
    }
    exitOperationMode();
}

void ExtendedSendWidget::onMoveUpClicked() {
    if (m_operationMode != OperationMode::MoveUp) {
        exitOperationMode();
        m_operationMode = OperationMode::MoveUp;
        m_moveUpButton->setText(QStringLiteral("确认上移"));
        m_moveUpButton->setStyleSheet("background-color: #20B2AA; color: white;");
        refreshTable();
        return;
    }

    // Second click: move selected items up
    QList<int> ids = getSelectedItemIds();
    if (ids.isEmpty()) {
        exitOperationMode();
        return;
    }

    for (int id : ids) {
        m_manager->moveItem(id, -1);
    }
    exitOperationMode();
}

void ExtendedSendWidget::onMoveDownClicked() {
    if (m_operationMode != OperationMode::MoveDown) {
        exitOperationMode();
        m_operationMode = OperationMode::MoveDown;
        m_moveDownButton->setText(QStringLiteral("确认下移"));
        m_moveDownButton->setStyleSheet("background-color: #20B2AA; color: white;");
        refreshTable();
        return;
    }

    // Second click: move selected items down
    QList<int> ids = getSelectedItemIds();
    if (ids.isEmpty()) {
        exitOperationMode();
        return;
    }

    for (int id : ids) {
        m_manager->moveItem(id, 1);
    }
    exitOperationMode();
}

void ExtendedSendWidget::exitOperationMode() {
    m_operationMode = OperationMode::Normal;
    if (m_deleteButton) {
        m_deleteButton->setText(QStringLiteral("删除"));
        m_deleteButton->setStyleSheet("");
    }
    if (m_moveUpButton) {
        m_moveUpButton->setText(QStringLiteral("上移"));
        m_moveUpButton->setStyleSheet("");
    }
    if (m_moveDownButton) {
        m_moveDownButton->setText(QStringLiteral("下移"));
        m_moveDownButton->setStyleSheet("");
    }
    refreshTable();
}

QList<int> ExtendedSendWidget::getSelectedItemIds() const {
    QList<int> ids;
    QList<SendItem> items = m_manager->allItems();
    for (int row = 0; row < m_table->rowCount(); ++row) {
        auto *check = qobject_cast<QCheckBox *>(m_table->cellWidget(row, 0));
        if (check && check->isChecked() && row < items.size()) {
            ids.append(items[row].id);
        }
    }
    return ids;
}

void ExtendedSendWidget::onStartSendClicked() {
    if (m_manager->isSending()) {
        m_manager->stopSending();
        return;
    }

    m_manager->sendMultiple(false);
}

void ExtendedSendWidget::setupContextMenu() {
    setContextMenuPolicy(Qt::CustomContextMenu);
    connect(this, &QWidget::customContextMenuRequested, [this](const QPoint &pos) {
        QMenu menu;
        menu.addAction(QStringLiteral("发送选中"), [this]() {
            m_manager->sendMultiple(false);
        });
        menu.addAction(QStringLiteral("清空所有"), [this]() { m_manager->clearItems(); });
        menu.addSeparator();
        menu.addAction(QStringLiteral("导入配置"), [this]() {
            QString path = QFileDialog::getOpenFileName(this, "Import", "", "JSON (*.json)");
            if (!path.isEmpty()) m_manager->importFromFile(path);
        });
        menu.addAction(QStringLiteral("导出配置"), [this]() {
            QString path = QFileDialog::getSaveFileName(this, "Export", "", "JSON (*.json)");
            if (!path.isEmpty()) m_manager->exportToFile(path);
        });
        menu.exec(mapToGlobal(pos));
    });
}
