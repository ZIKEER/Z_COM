#include "ansi_parser.h"

QString AnsiParser::escapeHtml(const QString &text) {
    QString result = text;
    result.replace('&', "&amp;");
    result.replace('<', "&lt;");
    result.replace('>', "&gt;");
    result.replace('\n', "<br>");
    return result;
}

QMap<int, QString> AnsiParser::initFgColors() {
    QMap<int, QString> map;
    // Standard colors 30-37
    map[30] = "#000000"; // Black
    map[31] = "#AA0000"; // Red
    map[32] = "#00AA00"; // Green
    map[33] = "#AA5500"; // Yellow
    map[34] = "#0000AA"; // Blue
    map[35] = "#AA00AA"; // Magenta
    map[36] = "#00AAAA"; // Cyan
    map[37] = "#AAAAAA"; // White
    // Bright colors 90-97
    map[90] = "#555555"; // Bright Black
    map[91] = "#FF5555"; // Bright Red
    map[92] = "#55FF55"; // Bright Green
    map[93] = "#FFFF55"; // Bright Yellow
    map[94] = "#5555FF"; // Bright Blue
    map[95] = "#FF55FF"; // Bright Magenta
    map[96] = "#55FFFF"; // Bright Cyan
    map[97] = "#FFFFFF"; // Bright White
    return map;
}

QString AnsiParser::parseSgr(const QString &paramsStr) {
    static QMap<int, QString> fgColors = initFgColors();

    if (paramsStr.isEmpty() || paramsStr == "0") {
        return "</span><span>"; // Reset
    }

    QStringList params = paramsStr.split(';');
    QString css;
    int i = 0;

    while (i < params.size()) {
        int code = params[i].toInt();

        switch (code) {
        case 0: // Reset
            return "</span><span>";
        case 1: // Bold
            css += "font-weight:bold;";
            break;
        case 4: // Underline
            css += "text-decoration:underline;";
            break;
        case 30: case 31: case 32: case 33:
        case 34: case 35: case 36: case 37: // Standard foreground
        case 90: case 91: case 92: case 93:
        case 94: case 95: case 96: case 97: // Bright foreground
            css += "color:" + fgColors.value(code) + ";";
            break;
        case 40: case 41: case 42: case 43:
        case 44: case 45: case 46: case 47: // Standard background
            css += "background-color:" + fgColors.value(code - 10) + ";";
            break;
        case 100: case 101: case 102: case 103:
        case 104: case 105: case 106: case 107: // Bright background
            css += "background-color:" + fgColors.value(code - 60) + ";";
            break;
        case 38: // Extended foreground
            if (i + 1 < params.size()) {
                int sub = params[i + 1].toInt();
                if (sub == 5 && i + 2 < params.size()) {
                    // 256-color: 38;5;N
                    int colorNum = params[i + 2].toInt();
                    // Simplified 256-color to hex
                    if (colorNum < 16) {
                        css += "color:" + fgColors.value(colorNum < 8 ? colorNum + 30 : colorNum + 82) + ";";
                    } else if (colorNum < 232) {
                        int idx = colorNum - 16;
                        int r = (idx / 36) * 51;
                        int g = ((idx % 36) / 6) * 51;
                        int b = (idx % 6) * 51;
                        css += QStringLiteral("color:rgb(%1,%2,%3);").arg(r).arg(g).arg(b);
                    } else {
                        int gray = (colorNum - 232) * 10 + 8;
                        css += QStringLiteral("color:rgb(%1,%1,%1);").arg(gray);
                    }
                    i += 2;
                } else if (sub == 2 && i + 4 < params.size()) {
                    // Truecolor: 38;2;R;G;B
                    int r = params[i + 2].toInt();
                    int g = params[i + 3].toInt();
                    int b = params[i + 4].toInt();
                    css += QStringLiteral("color:rgb(%1,%2,%3);").arg(r).arg(g).arg(b);
                    i += 4;
                }
            }
            break;
        case 48: // Extended background
            if (i + 1 < params.size()) {
                int sub = params[i + 1].toInt();
                if (sub == 5 && i + 2 < params.size()) {
                    int colorNum = params[i + 2].toInt();
                    if (colorNum < 16) {
                        css += "background-color:" + fgColors.value(colorNum < 8 ? colorNum + 30 : colorNum + 82) + ";";
                    } else if (colorNum < 232) {
                        int idx = colorNum - 16;
                        int r = (idx / 36) * 51;
                        int g = ((idx % 36) / 6) * 51;
                        int b = (idx % 6) * 51;
                        css += QStringLiteral("background-color:rgb(%1,%2,%3);").arg(r).arg(g).arg(b);
                    } else {
                        int gray = (colorNum - 232) * 10 + 8;
                        css += QStringLiteral("background-color:rgb(%1,%1,%1);").arg(gray);
                    }
                    i += 2;
                } else if (sub == 2 && i + 4 < params.size()) {
                    int r = params[i + 2].toInt();
                    int g = params[i + 3].toInt();
                    int b = params[i + 4].toInt();
                    css += QStringLiteral("background-color:rgb(%1,%2,%3);").arg(r).arg(g).arg(b);
                    i += 4;
                }
            }
            break;
        }
        ++i;
    }

    if (css.isEmpty()) return QString();
    return QStringLiteral("</span><span style=\"%1\">").arg(css);
}

QString AnsiParser::bytesToHtml(const QByteArray &data, AsciiFunc toAsciiFunc) {
    QString html;
    html.reserve(data.size() * 2);

    QByteArray nonAnsiBuffer;
    int i = 0;

    while (i < data.size()) {
        uchar ch = static_cast<uchar>(data[i]);

        if (ch == 0x1B) { // ESC
            // Flush non-ANSI buffer
            if (!nonAnsiBuffer.isEmpty()) {
                html += escapeHtml(toAsciiFunc(nonAnsiBuffer));
                nonAnsiBuffer.clear();
            }

            // Check for CSI sequence: ESC [ ... m
            if (i + 1 < data.size() && static_cast<uchar>(data[i + 1]) == '[') {
                // Find 'm' terminator
                int j = i + 2;
                QByteArray params;
                while (j < data.size()) {
                    uchar c = static_cast<uchar>(data[j]);
                    if (c == 'm') {
                        // Found SGR terminator
                        QString css = parseSgr(QString::fromLatin1(params));
                        if (!css.isEmpty()) {
                            html += css;
                        }
                        i = j + 1;
                        goto next;
                    } else if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')) {
                        // Other CSI sequence, skip
                        i = j + 1;
                        goto next;
                    }
                    params += static_cast<char>(c);
                    ++j;
                }
                // Incomplete sequence
                i = j;
            } else {
                // Standalone ESC
                nonAnsiBuffer += static_cast<char>(ch);
                ++i;
            }
        } else {
            nonAnsiBuffer += static_cast<char>(ch);
            ++i;
        }
        next:;
    }

    // Flush remaining buffer
    if (!nonAnsiBuffer.isEmpty()) {
        html += escapeHtml(toAsciiFunc(nonAnsiBuffer));
    }

    return html;
}
