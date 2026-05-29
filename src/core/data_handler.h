#ifndef DATA_HANDLER_H
#define DATA_HANDLER_H

#include <QByteArray>
#include <QString>
#include <QMap>

class DataHandler {
public:
    // Convert bytes to space-separated hex string
    static QString bytesToHex(const QByteArray &data);

    // Convert bytes to displayable ASCII string
    // Printable ASCII as-is, control chars as Unicode symbols, high bytes as \xNN
    static QString bytesToAscii(const QByteArray &data);

    // Format data for display based on mode ("HEX", "ASCII", "MIXED")
    static QString formatDisplay(const QByteArray &data, const QString &mode);

    // Validate hex input string, returns empty QByteArray on failure
    static QByteArray validateHexInput(const QString &text, bool *ok = nullptr);

    // Control character map: 0x00-0x1F, 0x7F -> Unicode control picture chars
    static QChar controlCharMap(uchar ch);

private:
    static QMap<uchar, QChar> initControlCharMap();
};

#endif // DATA_HANDLER_H
