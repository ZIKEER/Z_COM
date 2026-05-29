#include "rtt_manager.h"
#include "rtt_reader.h"

#include <QDir>
#include <QCoreApplication>
#include <QDebug>

// J-Link constants
#define JLINK_TIF_SWD 1

RttManager::RttManager(QObject *parent)
    : IOTransport(parent)
{
    loadJLinkSDK();
}

RttManager::~RttManager()
{
    closeResource();
    if (m_jlinkLib) {
        m_jlinkLib->unload();
        delete m_jlinkLib;
    }
}

bool RttManager::loadJLinkSDK()
{
    if (m_jlinkAvailable) return true;

    // Try to find JLinkARM.dll in common locations
    QStringList searchPaths;

    // Application directory
    searchPaths << QCoreApplication::applicationDirPath();

    // J-Link installation directory (common locations)
    searchPaths << "C:/Program Files/SEGGER/JLink"
                << "C:/Program Files (x86)/SEGGER/JLink";

    // Environment variable
    QString jlinkDir = qgetenv("JLINK_DIR");
    if (!jlinkDir.isEmpty()) {
        searchPaths << jlinkDir;
    }

    // PATH environment
    QString pathEnv = qgetenv("PATH");
    for (const QString &path : pathEnv.split(';')) {
        if (!path.isEmpty()) {
            searchPaths << path;
        }
    }

    // Try to load DLL (64-bit for our 64-bit application)
    for (const QString &dir : searchPaths) {
        QString dllPath = QDir(dir).absoluteFilePath("JLink_x64.dll");
        if (!QFile::exists(dllPath)) {
            // Fallback to JLinkARM.dll (32-bit)
            dllPath = QDir(dir).absoluteFilePath("JLinkARM.dll");
        }
        if (QFile::exists(dllPath)) {
            m_jlinkLib = new QLibrary(dllPath, this);

            // Load function pointers (JLINKARM_ prefix for most functions)
            fpOpen = (JLINKARM_Open)m_jlinkLib->resolve("JLINKARM_Open");
            fpClose = (JLINKARM_Close)m_jlinkLib->resolve("JLINKARM_Close");
            fpIsOpen = (JLINKARM_IsOpen)m_jlinkLib->resolve("JLINKARM_IsOpen");
            fpGetSN = (JLINKARM_GetSN)m_jlinkLib->resolve("JLINKARM_GetSN");
            fpGetProductName = (JLINKARM_EMU_GetProductName)m_jlinkLib->resolve("JLINKARM_EMU_GetProductName");
            fpGetNumEmulators = (JLINKARM_EMU_GetNumDevices)m_jlinkLib->resolve("JLINKARM_EMU_GetNumDevices");
            fpGetEmuList = (JLINKARM_EMU_GetList)m_jlinkLib->resolve("JLINKARM_EMU_GetList");
            fpTIF_Select = (JLINKARM_TIF_Select)m_jlinkLib->resolve("JLINKARM_TIF_Select");
            fpSetSpeed = (JLINKARM_SetSpeed)m_jlinkLib->resolve("JLINKARM_SetSpeed");
            fpConnect = (JLINKARM_Connect)m_jlinkLib->resolve("JLINKARM_Connect");
            fpSelDevice = (JLINKARM_SelDevice)m_jlinkLib->resolve("JLINKARM_SelDevice");
            fpReset = (JLINKARM_Reset)m_jlinkLib->resolve("JLINKARM_Reset");
            fpHalt = (JLINKARM_Halt)m_jlinkLib->resolve("JLINKARM_Halt");
            fpGo = (JLINKARM_Go)m_jlinkLib->resolve("JLINKARM_Go");
            fpReadMem = (JLINKARM_ReadMem)m_jlinkLib->resolve("JLINKARM_ReadMem");
            fpWriteMem = (JLINKARM_WriteMem)m_jlinkLib->resolve("JLINKARM_WriteMem");
            fpGetDLLVersion = (JLINKARM_GetDLLVersion)m_jlinkLib->resolve("JLINKARM_GetDLLVersion");
            fpGetEmuCaps = (JLINKARM_GetEmuCaps)m_jlinkLib->resolve("JLINKARM_GetEmuCaps");
            fpGetHardwareVersion = (JLINKARM_GetHardwareVersion)m_jlinkLib->resolve("JLINKARM_GetHardwareVersion");
            fpGetFirmwareString = (JLINKARM_GetFirmwareString)m_jlinkLib->resolve("JLINKARM_GetFirmwareString");

            // RTT functions (JLINK_ prefix)
            fpRTT_Control = (JLINK_RTTERMINAL_Control)m_jlinkLib->resolve("JLINK_RTTERMINAL_Control");
            fpRTT_Read = (JLINK_RTTERMINAL_Read)m_jlinkLib->resolve("JLINK_RTTERMINAL_Read");
            fpRTT_Write = (JLINK_RTTERMINAL_Write)m_jlinkLib->resolve("JLINK_RTTERMINAL_Write");

            // Check if essential functions are loaded
            if (fpOpen && fpClose && fpIsOpen && fpConnect &&
                fpRTT_Control && fpRTT_Read && fpRTT_Write) {
                m_jlinkAvailable = true;
                qDebug() << "[RTT] J-Link SDK loaded from:" << dllPath;

                // Get DLL version
                if (fpGetDLLVersion) {
                    int version = fpGetDLLVersion();
                    qDebug() << "[RTT] J-Link DLL version:" << version;
                }

                return true;
            } else {
                qDebug() << "[RTT] Failed to load essential J-Link functions from:" << dllPath;
                delete m_jlinkLib;
                m_jlinkLib = nullptr;
            }
        }
    }

    qDebug() << "[RTT] JLinkARM.dll not found in search paths";
    return false;
}

QList<QPair<QString, QString>> RttManager::getAvailableDevices()
{
    QList<QPair<QString, QString>> devices;

    if (!m_jlinkAvailable) {
        if (!loadJLinkSDK()) {
            return devices;
        }
    }

    QMutexLocker locker(&m_mutex);

    // Get number of emulators
    int numEmulators = 0;
    if (fpGetNumEmulators) {
        numEmulators = fpGetNumEmulators();
        qDebug() << "[RTT] Found" << numEmulators << "J-Link emulators";
    }

    // Enumerate devices by opening each by index
    for (int i = 0; i < numEmulators; ++i) {
        int handle = fpOpen(i);
        if (handle >= 0) {
            int sn = fpGetSN ? fpGetSN() : 0;

            char name[256] = {0};
            if (fpGetProductName) {
                fpGetProductName(name, sizeof(name));
            }

            fpClose();

            QString displayName = QString("%1 (SN=%2)").arg(name).arg(sn);
            devices.append({QString::number(sn), displayName});
            qDebug() << "[RTT] Found J-Link: SN=" << sn << "Name=" << name;
        }
    }

    return devices;
}

QString RttManager::getSerialNumber() const
{
    QMutexLocker locker(&m_mutex);
    return QString::number(m_serialNo);
}

bool RttManager::connectImpl(const QVariantMap &params)
{
    if (!m_jlinkAvailable) {
        emit errorOccurred("J-Link SDK not available. JLinkARM.dll not found.");
        return false;
    }

    QMutexLocker locker(&m_mutex);

    // Get parameters
    QString chip = params.value("chip", "nRF52840_xxAA").toString();
    int speed = params.value("speed", 4000).toInt();
    bool reset = params.value("reset", false).toBool();

    // Open J-Link
    int handle = fpOpen(-1);  // -1 = default device
    if (handle < 0) {
        emit errorOccurred("Failed to open J-Link device");
        return false;
    }

    m_serialNo = fpGetSN ? fpGetSN() : 0;

    // Select SWD interface
    if (fpTIF_Select) {
        int result = fpTIF_Select(JLINK_TIF_SWD);
        if (result < 0) {
            emit errorOccurred("Failed to select SWD interface");
            fpClose();
            return false;
        }
    }

    // Set speed
    if (fpSetSpeed) {
        fpSetSpeed(speed);
    }

    // Select device (chip name)
    if (fpSelDevice && !chip.isEmpty()) {
        QByteArray chipUtf8 = chip.toUtf8();
        int result = fpSelDevice(chipUtf8.constData());
        qDebug() << "[RTT] SelDevice(" << chip << ") result:" << result;
    }

    // Connect to chip
    if (fpConnect) {
        int result = fpConnect();
        if (result < 0) {
            emit errorOccurred("Failed to connect to chip: " + chip);
            fpClose();
            return false;
        }
    }

    // Reset if requested
    if (reset && fpReset) {
        fpReset(10, 0); // ms=10, halt=0
    }

    // Start RTT
    if (fpRTT_Control) {
        int result = fpRTT_Control(1, nullptr); // 1 = Start
        if (result < 0) {
            emit errorOccurred("Failed to start RTT");
            fpClose();
            return false;
        }
    }

    m_connected = true;

    // Create reader thread
    auto *reader = new RttReaderThread(this, 0, 8192, 2, 50);
    connect(reader, &RttReaderThread::dataReceived, this, &IOTransport::dataReceived);
    connect(reader, &RttReaderThread::errorOccurred, this, &IOTransport::errorOccurred);
    startReaderThread(reader);

    return true;
}

void RttManager::closeResource()
{
    QMutexLocker locker(&m_mutex);

    if (m_connected && fpRTT_Control) {
        fpRTT_Control(0, nullptr); // 0 = Stop
    }

    if (fpClose) {
        fpClose();
    }

    m_connected = false;
    m_serialNo = 0;
}

bool RttManager::sendBytes(const QByteArray &data)
{
    if (!m_connected || !fpRTT_Write) {
        return false;
    }

    QMutexLocker locker(&m_mutex);

    int written = fpRTT_Write(0, data.constData(), data.size());
    return written == data.size();
}

int RttManager::readRTT(int bufferIdx, void *data, int size)
{
    if (!m_connected || !fpRTT_Read) {
        return 0;
    }

    QMutexLocker locker(&m_mutex);

    return fpRTT_Read(bufferIdx, data, size);
}
