#include "log_converter.h"

#include <QFile>
#include <QTextStream>
#include <QDir>
#include <QFileInfo>
#include <QRegularExpression>

QList<LogConverter::Entry> LogConverter::parseEntries(const QString &text)
{
    QList<Entry> entries;
    QStringList lines = text.split('\n');

    static QRegularExpression timestampRe(R"(^\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}\])");

    int i = 0;
    while (i < lines.size()) {
        QString line = lines[i];
        if (line.trimmed().isEmpty()) {
            i++;
            continue;
        }

        // 检查是否是时间戳行
        if (!timestampRe.match(line).hasMatch()) {
            i++;
            continue;
        }

        // 时间戳行，检查下一行是否是 HEX
        if (i + 1 < lines.size() && lines[i + 1].trimmed().startsWith(QStringLiteral("← HEX:"))) {
            Entry entry;
            entry.timestamp = line;

            // 提取 HEX 数据
            entry.hexData = lines[i + 1];
            i += 2;

            // 收集 ASCII 部分（多行，直到下一个时间戳或 EOF）
            if (i < lines.size() && lines[i].trimmed().startsWith(QStringLiteral("← ASCII:"))) {
                QStringList asciiLines;
                asciiLines.append(lines[i]);
                i++;

                while (i < lines.size() && !timestampRe.match(lines[i]).hasMatch()) {
                    asciiLines.append(lines[i]);
                    i++;
                }
                entry.asciiLines = asciiLines;
            }
            entries.append(entry);
        } else {
            // 事件条目（单行）
            Entry entry;
            entry.timestamp = line;
            entry.isEvent = true;
            entries.append(entry);
            i++;
        }
    }

    return entries;
}

QString LogConverter::extractTimestamp(const QString &line)
{
    static QRegularExpression re(R"(^(\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}\]))");
    QRegularExpressionMatch match = re.match(line);
    return match.hasMatch() ? match.captured(1) : line;
}

QString LogConverter::extractHexData(const QString &hexLine)
{
    static QRegularExpression re(R"(\s*←\s*HEX:\s*(.*))");
    QRegularExpressionMatch match = re.match(hexLine);
    return match.hasMatch() ? match.captured(1).trimmed() : hexLine;
}

QStringList LogConverter::extractAsciiLines(const QStringList &asciiSection)
{
    QStringList result;
    for (int i = 0; i < asciiSection.size(); ++i) {
        const QString &line = asciiSection[i];
        if (i == 0) {
            static QRegularExpression re(R"(\s*←\s*ASCII:\s*(.*))");
            QRegularExpressionMatch match = re.match(line);
            result.append(match.hasMatch() ? match.captured(1).trimmed() : line.trimmed());
        } else {
            result.append(line.trimmed());
        }
    }
    return result;
}

QString LogConverter::convertToHex(const QList<Entry> &entries)
{
    QStringList output;
    for (const Entry &entry : entries) {
        QString ts = extractTimestamp(entry.timestamp);
        if (entry.isEvent) {
            output.append(entry.timestamp);
        } else {
            QString hexData = extractHexData(entry.hexData);
            output.append(QStringLiteral("%1 %2").arg(ts, hexData));
        }
    }
    return output.join('\n') + '\n';
}

QString LogConverter::convertToASCII(const QList<Entry> &entries)
{
    QStringList output;
    for (const Entry &entry : entries) {
        QString ts = extractTimestamp(entry.timestamp);
        if (entry.isEvent) {
            output.append(entry.timestamp);
        } else if (!entry.asciiLines.isEmpty()) {
            QStringList asciiLines = extractAsciiLines(entry.asciiLines);
            output.append(QStringLiteral("%1 %2").arg(ts, asciiLines.first()));
            for (int i = 1; i < asciiLines.size(); ++i) {
                output.append(asciiLines[i]);
            }
        } else {
            output.append(ts);
        }
    }
    return output.join('\n') + '\n';
}

LogConverter::Result LogConverter::convertFile(const QString &inputPath, Format format,
                                               const QString &outputDir)
{
    Result result;

    QFile file(inputPath);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return result;
    }

    QTextStream in(&file);
    in.setEncoding(QStringConverter::Utf8);
    QString text = in.readAll();
    file.close();

    QList<Entry> entries = parseEntries(text);

    QFileInfo fileInfo(inputPath);
    QString stem = fileInfo.completeBaseName();
    QString parent = outputDir.isEmpty() ? fileInfo.absolutePath() : outputDir;

    // 确保输出目录存在
    QDir dir(parent);
    if (!dir.exists()) {
        dir.mkpath(".");
    }

    if (format == Format::Hex || format == Format::Both) {
        QString hexText = convertToHex(entries);
        QString hexPath = dir.absoluteFilePath(stem + "_HEX.txt");

        QFile hexFile(hexPath);
        if (hexFile.open(QIODevice::WriteOnly | QIODevice::Text)) {
            QTextStream out(&hexFile);
            out.setEncoding(QStringConverter::Utf8);
            out << hexText;
            hexFile.close();
            result.hexPath = hexPath;
        }
    }

    if (format == Format::ASCII || format == Format::Both) {
        QString asciiText = convertToASCII(entries);
        QString asciiPath = dir.absoluteFilePath(stem + "_ASCII.txt");

        QFile asciiFile(asciiPath);
        if (asciiFile.open(QIODevice::WriteOnly | QIODevice::Text)) {
            QTextStream out(&asciiFile);
            out.setEncoding(QStringConverter::Utf8);
            out << asciiText;
            asciiFile.close();
            result.asciiPath = asciiPath;
        }
    }

    return result;
}

int LogConverter::convertDirectory(const QString &dirPath, Format format,
                                   const QString &outputDir)
{
    QDir dir(dirPath);
    if (!dir.exists()) {
        return 0;
    }

    QStringList filters;
    filters << "log_*.txt";
    QFileInfoList fileList = dir.entryInfoList(filters, QDir::Files, QDir::Name);

    int count = 0;
    for (const QFileInfo &fileInfo : fileList) {
        Result result = convertFile(fileInfo.absoluteFilePath(), format, outputDir);
        if (!result.hexPath.isEmpty() || !result.asciiPath.isEmpty()) {
            count++;
        }
    }

    return count;
}
