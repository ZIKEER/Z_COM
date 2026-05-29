#ifndef SERIAL_SETTINGS_DIALOG_H
#define SERIAL_SETTINGS_DIALOG_H

#include <QDialog>
#include <QVariantMap>

namespace Ui { class SerialSettingsDialog; }

class SerialSettingsDialog : public QDialog {
    Q_OBJECT

public:
    explicit SerialSettingsDialog(const QVariantMap &currentSettings,
                                  const QVariantMap &rttSettings,
                                  bool displayAnsi,
                                  QWidget *parent = nullptr);
    ~SerialSettingsDialog();

    QVariantMap getSettings() const;
    QVariantMap getRttSettings() const;
    QVariantMap getCommonSettings() const;

signals:
    void settingsChanged(const QVariantMap &settings);
    void rttSettingsChanged(const QVariantMap &settings);
    void commonSettingsChanged(const QVariantMap &settings);

private slots:
    void onAccept();

private:
    void loadRttSettings(const QVariantMap &settings);

    Ui::SerialSettingsDialog *ui;
};

#endif // SERIAL_SETTINGS_DIALOG_H
