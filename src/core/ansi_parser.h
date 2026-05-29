#ifndef ANSI_PARSER_H
#define ANSI_PARSER_H

#include <QByteArray>
#include <QString>
#include <QMap>
#include <functional>

class AnsiParser {
public:
    // Convert raw bytes (potentially containing ANSI escape sequences) to HTML
    // toAsciiFunc: function to convert bytes to displayable ASCII string
    using AsciiFunc = std::function<QString(const QByteArray &)>;

    static QString bytesToHtml(const QByteArray &data, AsciiFunc toAsciiFunc);

    // Escape HTML special characters
    static QString escapeHtml(const QString &text);

private:
    // Parse SGR (Select Graphic Rendition) parameters to CSS
    static QString parseSgr(const QString &paramsStr);

    static QMap<int, QString> initFgColors();
};

#endif // ANSI_PARSER_H
