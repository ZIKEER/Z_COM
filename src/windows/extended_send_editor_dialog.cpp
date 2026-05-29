#include "extended_send_editor_dialog.h"
#include "ui_extended_send_editor_dialog.h"

ExtendedSendEditorDialog::ExtendedSendEditorDialog(const QString &text, QWidget *parent)
    : QDialog(parent), ui(new Ui::ExtendedSendEditorDialog)
{
    ui->setupUi(this);
    ui->dataPlainTextEdit->setPlainText(text);
    ui->dataPlainTextEdit->setFont(QFont("Consolas", 10));
}

ExtendedSendEditorDialog::~ExtendedSendEditorDialog() {
    delete ui;
}

QString ExtendedSendEditorDialog::getText() const {
    return ui->dataPlainTextEdit->toPlainText();
}
