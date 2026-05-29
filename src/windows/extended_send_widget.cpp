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
    menu.addAction("Advanced Edit", this, &SendItemWidget::openAdvancedEditor);
    menu.addSeparator();
    menu.addAction("Cut", m_dataEdit, &QLineEdit::cut);
    menu.addAction("Copy", m_dataEdit, &QLineEdit::copy);
    menu.addAction("Paste", m_dataEdit, &QLineEdit::paste);
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
    QString comment = QInputDialog::getText(this, "Edit Comment",
                                            "Comment:", QLineEdit::Normal,
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

    // Connect buttons
    connect(ui_form->addButton, &QPushButton::clicked, this, &ExtendedSendWidget::addItem);
    connect(ui_form->deleteButton, &QPushButton::clicked, this, &ExtendedSendWidget::onDeleteClicked);
    connect(ui_form->moveUpButton, &QPushButton::clicked, this, &ExtendedSendWidget::onMoveUpClicked);
    connect(ui_form->moveDownButton, &QPushButton::clicked, this, &ExtendedSendWidget::onMoveDownClicked);
    connect(ui_form->startSendButton, &QPushButton::clicked, this, &ExtendedSendWidget::onStartSendClicked);

    // Manager signals
    connect(m_manager, &ExtendedSendManager::itemsChanged, this, &ExtendedSendWidget::refreshTable);
    connect(m_manager, &ExtendedSendManager::dataSent, this, &ExtendedSendWidget::sendData);

    setupContextMenu();
    refreshTable();
}

void ExtendedSendWidget::refreshTable() {
    m_table->setRowCount(0);

    QList<SendItem> items = m_manager->allItems();
    for (const SendItem &item : items) {
        int row = m_table->rowCount();
        m_table->insertRow(row);

        // HEX checkbox
        auto *hexCheck = new QCheckBox;
        hexCheck->setChecked(item.isHex);
        m_table->setCellWidget(row, 0, hexCheck);

        // Data/Comment widget
        auto *itemWidget = new SendItemWidget(item);
        m_table->setCellWidget(row, 1, itemWidget);

        // Order
        auto *orderItem = new QTableWidgetItem(QString::number(item.sortOrder));
        m_table->setItem(row, 2, orderItem);

        // Delay
        auto *delayItem = new QTableWidgetItem(QString::number(item.delay));
        m_table->setItem(row, 3, delayItem);
    }
}

void ExtendedSendWidget::addItem() {
    m_manager->addItem("", false, "", 100);
}

void ExtendedSendWidget::onDeleteClicked() {
    if (m_operationMode != OperationMode::Delete) {
        m_operationMode = OperationMode::Delete;
        // TODO: Add selection checkboxes
        return;
    }

    // Delete selected items
    for (int row = m_table->rowCount() - 1; row >= 0; --row) {
        auto *check = qobject_cast<QCheckBox *>(m_table->cellWidget(row, 0));
        if (check && check->isChecked()) {
            // Get item ID from manager
            QList<SendItem> items = m_manager->allItems();
            if (row < items.size()) {
                m_manager->removeItem(items[row].id);
            }
        }
    }
    m_operationMode = OperationMode::Normal;
}

void ExtendedSendWidget::onMoveUpClicked() {
    int row = m_table->currentRow();
    if (row <= 0) return;

    QList<SendItem> items = m_manager->allItems();
    if (row < items.size()) {
        m_manager->moveItem(items[row].id, -1);
    }
}

void ExtendedSendWidget::onMoveDownClicked() {
    int row = m_table->currentRow();
    if (row < 0 || row >= m_table->rowCount() - 1) return;

    QList<SendItem> items = m_manager->allItems();
    if (row < items.size()) {
        m_manager->moveItem(items[row].id, 1);
    }
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
        menu.addAction("Clear All", [this]() { m_manager->clearItems(); });
        menu.addSeparator();
        menu.addAction("Import Config", [this]() {
            QString path = QFileDialog::getOpenFileName(this, "Import", "", "JSON (*.json)");
            if (!path.isEmpty()) m_manager->importFromFile(path);
        });
        menu.addAction("Export Config", [this]() {
            QString path = QFileDialog::getSaveFileName(this, "Export", "", "JSON (*.json)");
            if (!path.isEmpty()) m_manager->exportToFile(path);
        });
        menu.exec(mapToGlobal(pos));
    });
}
