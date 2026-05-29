#ifndef EXTENDED_SEND_EDITOR_DIALOG_H
#define EXTENDED_SEND_EDITOR_DIALOG_H

#include <QDialog>

namespace Ui { class ExtendedSendEditorDialog; }

class ExtendedSendEditorDialog : public QDialog {
    Q_OBJECT

public:
    explicit ExtendedSendEditorDialog(const QString &text, QWidget *parent = nullptr);
    ~ExtendedSendEditorDialog();

    QString getText() const;

private:
    Ui::ExtendedSendEditorDialog *ui;
};

#endif // EXTENDED_SEND_EDITOR_DIALOG_H
