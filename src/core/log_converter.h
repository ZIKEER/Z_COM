#ifndef LOG_CONVERTER_H
#define LOG_CONVERTER_H

#include <QString>
#include <QStringList>
#include <QMap>

/**
 * @brief 日志转换工具：将 MIX 格式的日志文件转换为 HEX-only 或 ASCII-only 格式
 */
class LogConverter
{
public:
    /**
     * @brief 转换格式枚举
     */
    enum class Format {
        Hex,
        ASCII,
        Both
    };

    /**
     * @brief 转换结果
     */
    struct Result {
        QString hexPath;
        QString asciiPath;
    };

    /**
     * @brief 转换单个文件
     * @param inputPath 输入文件路径
     * @param format 输出格式
     * @param outputDir 输出目录（为空则与输入文件同目录）
     * @return 转换结果
     */
    static Result convertFile(const QString &inputPath, Format format,
                              const QString &outputDir = QString());

    /**
     * @brief 批量转换目录下的日志文件
     * @param dirPath 目录路径
     * @param format 输出格式
     * @param outputDir 输出目录（为空则与输入文件同目录）
     * @return 转换的文件数量
     */
    static int convertDirectory(const QString &dirPath, Format format,
                                const QString &outputDir = QString());

private:
    /**
     * @brief 日志条目
     */
    struct Entry {
        QString timestamp;
        QString hexData;
        QStringList asciiLines;
        bool isEvent = false;  // 事件条目（无数据）
    };

    /**
     * @brief 解析日志条目
     */
    static QList<Entry> parseEntries(const QString &text);

    /**
     * @brief 提取时间戳
     */
    static QString extractTimestamp(const QString &line);

    /**
     * @brief 提取 HEX 数据
     */
    static QString extractHexData(const QString &hexLine);

    /**
     * @brief 提取 ASCII 数据
     */
    static QStringList extractAsciiLines(const QStringList &asciiSection);

    /**
     * @brief 转换为 HEX 格式
     */
    static QString convertToHex(const QList<Entry> &entries);

    /**
     * @brief 转换为 ASCII 格式
     */
    static QString convertToASCII(const QList<Entry> &entries);
};

#endif // LOG_CONVERTER_H
