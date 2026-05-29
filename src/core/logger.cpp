#include "logger.h"

#include <QTextStream>
#include <QCoreApplication>

QMutex Logger::s_counterMutex;
int Logger::s_globalCounter = 0;

Logger::Logger(const QString &logDir, int instanceId)
    : m_instanceId(instanceId)
{
    // Resolve log directory based on instance ID
    if (instanceId <= 1) {
        m_logDir = logDir;
    } else {
        m_logDir = QCoreApplication::applicationDirPath()
                   + "/instance_" + QString::number(instanceId) + "/" + logDir;
    }

    QDir dir(m_logDir);
    if (!dir.exists()) {
        dir.mkpath(".");
    }

    createNewLogFile();
}

Logger::~Logger() {
    flush();
}

void Logger::createNewLogFile() {
    QMutexLocker locker(&s_counterMutex);
    s_globalCounter++;
    int counter = s_globalCounter;
    locker.unlock();

    QString timestamp = QDateTime::currentDateTime().toString("yyyy-MM-dd_HHmmss");
    m_logPath = m_logDir + "/log_" + timestamp + ".txt";

    // Make unique if needed
    if (counter > 1) {
        m_logPath = m_logDir + "/log_" + timestamp + "_" + QString::number(counter) + ".txt";
    }

    if (m_file.isOpen()) {
        m_file.close();
    }
    m_file.setFileName(m_logPath);
    m_file.open(QIODevice::WriteOnly | QIODevice::Text | QIODevice::Append);
}

void Logger::log(const QDateTime &timestamp, const QString &direction,
                 const QString &hexStr, const QString &asciiStr) {
    // Direction arrow: RECEIVE -> <-, SEND -> ->
    QString arrow = (direction == "RECEIVE") ? "<-" : "->";

    QString entry = QStringLiteral("[%1] %2 %3\n    HEX: %4\n    ASCII: %5")
                    .arg(timestamp.toString("HH:mm:ss.zzz"))
                    .arg(arrow, direction, hexStr, asciiStr);

    QMutexLocker locker(&m_mutex);
    m_buffer.append(entry);
}

void Logger::logEvent(const QString &text) {
    QString entry = QStringLiteral("[%1] %2")
                    .arg(QDateTime::currentDateTime().toString("HH:mm:ss.zzz"))
                    .arg(text);

    QMutexLocker locker(&m_mutex);
    m_buffer.append(entry);
}

void Logger::flush() {
    QMutexLocker locker(&m_mutex);
    if (m_buffer.isEmpty()) return;

    if (m_file.isOpen()) {
        QTextStream stream(&m_file);
        for (const QString &entry : m_buffer) {
            stream << entry << "\n";
        }
        stream.flush();
    }

    m_buffer.clear();
    rotateIfNeeded();
}

void Logger::rotateIfNeeded() {
    if (!m_file.isOpen()) return;
    if (m_file.size() < MAX_LOG_FILE_SIZE) return;

    m_file.close();
    createNewLogFile();
}
