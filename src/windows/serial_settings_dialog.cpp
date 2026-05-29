#include "serial_settings_dialog.h"
#include "ui_serial_settings_dialog.h"

SerialSettingsDialog::SerialSettingsDialog(const QVariantMap &currentSettings,
                                           const QVariantMap &rttSettings,
                                           bool displayAnsi,
                                           QWidget *parent)
    : QDialog(parent), ui(new Ui::SerialSettingsDialog)
{
    ui->setupUi(this);

    // Populate serial settings
    ui->databitsComboBox->addItems({"5", "6", "7", "8"});
    ui->stopbitsComboBox->addItems({"1", "1.5", "2"});
    ui->parityComboBox->addItems({"None", "Even", "Odd", "Mark", "Space"});
    ui->flowcontrolComboBox->addItems({"None", "RTS-CTS", "DTR-DSR", "XON-XOFF"});

    // Set current values
    ui->databitsComboBox->setCurrentText(currentSettings.value("databits", "8").toString());
    ui->stopbitsComboBox->setCurrentText(currentSettings.value("stopbits", "1").toString());
    ui->parityComboBox->setCurrentText(currentSettings.value("parity", "None").toString());
    ui->flowcontrolComboBox->setCurrentText(currentSettings.value("flowcontrol", "None").toString());

    // Common settings
    ui->frameTimeoutSpinBox->setValue(currentSettings.value("frame_timeout", 50).toInt());
    ui->ansiCheckBox->setChecked(displayAnsi);

    // RTT settings
    loadRttSettings(rttSettings);

    connect(ui->buttonBox, &QDialogButtonBox::accepted, this, &SerialSettingsDialog::onAccept);
    connect(ui->buttonBox, &QDialogButtonBox::rejected, this, &QDialog::reject);
}

SerialSettingsDialog::~SerialSettingsDialog() {
    delete ui;
}

void SerialSettingsDialog::loadRttSettings(const QVariantMap &settings) {
    // Chip history
    QStringList history = settings.value("chip_history").toStringList();
    ui->rttChipComboBox->clear();
    ui->rttChipComboBox->addItems(history);
    ui->rttChipComboBox->setEditable(true);
    if (!settings.value("chip").toString().isEmpty()) {
        ui->rttChipComboBox->setCurrentText(settings.value("chip").toString());
    }

    ui->rttSpeedSpinBox->setValue(settings.value("speed", 4000).toInt());
    ui->rttResetCheckBox->setChecked(settings.value("reset", false).toBool());
    ui->rttStartAddressLineEdit->setText(settings.value("start_address").toString());
    ui->rttRangeSizeLineEdit->setText(settings.value("range_size").toString());
}

void SerialSettingsDialog::onAccept() {
    // Emit all settings
    emit settingsChanged(getSettings());
    emit rttSettingsChanged(getRttSettings());
    emit commonSettingsChanged(getCommonSettings());
    accept();
}

QVariantMap SerialSettingsDialog::getSettings() const {
    QVariantMap s;
    s["databits"] = ui->databitsComboBox->currentText().toInt();
    s["stopbits"] = ui->stopbitsComboBox->currentText().toFloat();
    s["parity"] = ui->parityComboBox->currentText();
    s["flowcontrol"] = ui->flowcontrolComboBox->currentText();
    return s;
}

QVariantMap SerialSettingsDialog::getRttSettings() const {
    QVariantMap s;
    s["chip"] = ui->rttChipComboBox->currentText();
    s["speed"] = ui->rttSpeedSpinBox->value();
    s["reset"] = ui->rttResetCheckBox->isChecked();
    s["start_address"] = ui->rttStartAddressLineEdit->text();
    s["range_size"] = ui->rttRangeSizeLineEdit->text();
    return s;
}

QVariantMap SerialSettingsDialog::getCommonSettings() const {
    QVariantMap s;
    s["frame_timeout"] = ui->frameTimeoutSpinBox->value();
    s["display_ansi"] = ui->ansiCheckBox->isChecked();
    return s;
}
