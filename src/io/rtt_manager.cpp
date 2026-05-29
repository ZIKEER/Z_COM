#include "rtt_manager.h"
#include "rtt_reader.h"

#include <QDir>
#include <QCoreApplication>
#include <QDebug>

// J-Link constants
#define JLINK_TIF_SWD 1
#define JLINK_EMU_CAPS_RTT (1 << 12)

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

    // Try to load DLL
    for (const QString &dir : searchPaths) {
        QString dllPath = QDir(dir).absoluteFilePath("JLinkARM.dll");
        if (QFile::exists(dllPath)) {
            m_jlinkLib = new QLibrary(dllPath, this);

            // Load function pointers
            fpOpen = (JLINK_Open)m_jlinkLib->resolve("JLINK_Open");
            fpClose = (JLINK_Close)m_jlinkLib->resolve("JLINK_Close");
            fpIsOpen = (JLINK_IsOpen)m_jlinkLib->resolve("JLINK_IsOpen");
            fpGetSN = (JLINK_GetSN)m_jlinkLib->resolve("JLINK_GetSN");
            fpGetProductName = (JLINK_GetProductName)m_jlinkLib->resolve("JLINK_GetProductName");
            fpGetNumEmulators = (JLINK_GetNumEmulators)m_jlinkLib->resolve("JLINK_GetNumEmulators");
            fpGetEmuList = (JLINK_GetEmuList)m_jlinkLib->resolve("JLINK_GetEmuList");
            fpTIF_Select = (JLINK_TIF_Select)m_jlinkLib->resolve("JLINK_TIF_Select");
            fpSetSpeed = (JLINK_SetSpeed)m_jlinkLib->resolve("JLINK_SetSpeed");
            fpConnect = (JLINK_Connect)m_jlinkLib->resolve("JLINK_Connect");
            fpReset = (JLINK_Reset)m_jlinkLib->resolve("JLINK_Reset");
            fpRTT_Start = (JLINK_RTT_Start)m_jlinkLib->resolve("JLINK_RTT_Start");
            fpRTT_Stop = (JLINK_RTT_Stop)m_jlinkLib->resolve("JLINK_RTT_Stop");
            fpRTT_Read = (JLINK_RTT_Read)m_jlinkLib->resolve("JLINK_RTT_Read");
            fpRTT_Write = (JLINK_RTT_Write)m_jlinkLib->resolve("JLINK_RTT_Write");
            fpGetDLLVersion = (JLINK_GetDLLVersion)m_jlinkLib->resolve("JLINK_GetDLLVersion");
            fpGetEmuCaps = (JLINK_GetEmuCaps)m_jlinkLib->resolve("JLINK_GetEmuCaps");
            fpGetHardwareVersion = (JLINK_GetHardwareVersion)m_jlinkLib->resolve("JLINK_GetHardwareVersion");
            fpGetFirmwareString = (JLINK_GetFirmwareString)m_jlinkLib->resolve("JLINK_GetFirmwareString");

            // Check if essential functions are loaded
            if (fpOpen && fpClose && fpIsOpen && fpRTT_Start && fpRTT_Stop &&
                fpRTT_Read && fpRTT_Write) {
                m_jlinkAvailable = true;
                qDebug() << "[RTT] J-Link SDK loaded from:" << dllPath;

                // Get DLL version
                if (fpGetDLLVersion) {
                    int version = fpGetDLLVersion();
                    qDebug() << "[RTT] J-Link DLL version:" << version;
                }

                return true;
            } else {
                qDebug() << "[RTT] Failed to load J-Link functions from:" << dllPath;
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

    if (numEmulators <= 0) {
        // Try to open default device
        int handle = fpOpen(-1);
        if (handle >= 0) {
            int sn = fpGetSN ? fpGetSN() : 0;
            char name[256] = {0};
            if (fpGetProductName) {
                fpGetProductName(name, sizeof(name));
            }

            QString displayName = QString("%1 (SN=%2)").arg(name).arg(sn);
            devices.append({QString::number(sn), displayName});
            qDebug() << "[RTT] Found default J-Link: SN=" << sn;

            fpClose();
        }
        return devices;
    }

    // Get emulator list
    struct EmuInfo {
        int serialNo;
        char productName[256];
    };

    QVector<EmuInfo> emuList(numEmulators);
    if (fpGetEmuList) {
        fpGetEmuList(emuList.data(), numEmulators);
    }

    for (int i = 0; i < numEmulators; ++i) {
        int sn = emuList[i].serialNo;
        QString name = QString(emuList[i].productName);

        // Try to open to get more info
        int handle = fpOpen(sn);
        if (handle >= 0) {
            char prodName[256] = {0};
            if (fpGetProductName) {
                fpGetProductName(prodName, sizeof(prodName));
            }
            name = prodName;

            // Get firmware string
            char fwStr[256] = {0};
            if (fpGetFirmwareString) {
                fpGetFirmwareString(fwStr, sizeof(fwStr));
            }

            fpClose();
        }

        QString displayName = QString("%1 (SN=%2)").arg(name).arg(sn);
        devices.append({QString::number(sn), displayName});
        qDebug() << "[RTT] Found J-Link: SN=" << sn << "Name=" << name;
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
    QString startAddressStr = params.value("start_address").toString();
    QString rangeSizeStr = params.value("range_size").toString();

    unsigned int startAddress = 0;
    unsigned int rangeSize = 0;

    if (!startAddressStr.isEmpty()) {
        bool ok;
        startAddress = startAddressStr.toUInt(&ok, 16);
    }
    if (!rangeSizeStr.isEmpty()) {
        bool ok;
        rangeSize = rangeSizeStr.toUInt(&ok, 16);
    }

    // Open J-Link
    int serialNo = params.value("serial_no", -1).toInt();
    int handle = fpOpen(serialNo);
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
    if (fpRTT_Start) {
        int result = fpRTT_Start(startAddress, rangeSize, 0);
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

    if (m_connected && fpRTT_Stop) {
        fpRTT_Stop();
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
