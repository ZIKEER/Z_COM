#include "data_handler.h"

#include <QTextStream>

QMap<uchar, QChar> DataHandler::initControlCharMap() {
    QMap<uchar, QChar> map;
    // 0x00-0x1F -> U+2400-U+241F
    for (uchar i = 0; i < 0x20; ++i) {
        map.insert(i, QChar(0x2400 + i));
    }
    // 0x7F (DEL) -> U+2421
    map.insert(0x7F, QChar(0x2421));
    return map;
}

QChar DataHandler::controlCharMap(uchar ch) {
    static QMap<uchar, QChar> map = initControlCharMap();
    return map.value(ch, QChar(ch));
}

QString DataHandler::bytesToHex(const QByteArray &data) {
    if (data.isEmpty()) return QString();

    QString result;
    result.reserve(data.size() * 3);
    for (int i = 0; i < data.size(); ++i) {
        if (i > 0) result += ' ';
        result += QString::number(static_cast<uchar>(data[i]), 16).rightJustified(2, '0').toUpper();
    }
    return result;
}

QString DataHandler::bytesToAscii(const QByteArray &data) {
    if (data.isEmpty()) return QString();

    QString result;
    result.reserve(data.size());
    for (int i = 0; i < data.size(); ++i) {
        uchar ch = static_cast<uchar>(data[i]);
        if (ch == 0x0A) {
            // LF: append control symbol + real newline
            result += controlCharMap(ch);
            result += '\n';
        } else if (ch < 0x20 || ch == 0x7F) {
            // Control characters -> Unicode symbols
            result += controlCharMap(ch);
        } else if (ch >= 0x80) {
            // High bytes -> \xNN
            result += QStringLiteral("\\x%1").arg(ch, 2, 16, QChar('0')).toUpper();
        } else {
            // Printable ASCII
            result += QChar(ch);
        }
    }
    return result;
}

QString DataHandler::formatDisplay(const QByteArray &data, const QString &mode) {
    if (mode == "HEX") {
        return bytesToHex(data);
    } else if (mode == "ASCII") {
        return bytesToAscii(data);
    } else if (mode == "MIXED") {
        // Mixed: show both hex and ascii
        QString hex = bytesToHex(data);
        QString ascii = bytesToAscii(data);
        return hex + "  |  " + ascii;
    }
    return bytesToAscii(data);
}

QByteArray DataHandler::validateHexInput(const QString &text, bool *ok) {
    // Strip spaces and newlines
    QString cleaned = text;
    cleaned.remove(' ');
    cleaned.remove('\n');
    cleaned.remove('\r');
    cleaned.remove('\t');

    if (cleaned.isEmpty()) {
        if (ok) *ok = false;
        return QByteArray();
    }

    // Must have even number of hex chars
    if (cleaned.size() % 2 != 0) {
        if (ok) *ok = false;
        return QByteArray();
    }

    // Validate hex characters
    for (const QChar &ch : cleaned) {
        if (!ch.isLetterOrNumber()) {
            if (ok) *ok = false;
            return QByteArray();
        }
    }

    QByteArray result = QByteArray::fromHex(cleaned.toLatin1());
    if (result.isEmpty() && !cleaned.isEmpty()) {
        if (ok) *ok = false;
        return QByteArray();
    }

    if (ok) *ok = true;
    return result;
}
