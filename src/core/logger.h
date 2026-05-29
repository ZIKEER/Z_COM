#ifndef LOGGER_H
#define LOGGER_H

#include <QObject>
#include <QFile>
#include <QDir>
#include <QMutex>
#include <QStringList>
#include <QDateTime>

class Logger {
public:
    explicit Logger(const QString &logDir, int instanceId = 1);
    ~Logger();

    // Log a data entry with timestamp, direction, hex and ascii representations
    void log(const QDateTime &timestamp, const QString &direction,
             const QString &hexStr, const QString &asciiStr);

    // Log a plain event text
    void logEvent(const QString &text);

    // Flush buffer to file
    void flush();

    // Get current log file path
    QString currentLogPath() const { return m_logPath; }

private:
    void createNewLogFile();
    void rotateIfNeeded();

    static const qint64 MAX_LOG_FILE_SIZE = 10 * 1024 * 1024; // 10 MB
    static QMutex s_counterMutex;
    static int s_globalCounter;

    QString m_logDir;
    int m_instanceId;
    QString m_logPath;
    QFile m_file;
    QStringList m_buffer;
    QMutex m_mutex;
};

#endif // LOGGER_H
