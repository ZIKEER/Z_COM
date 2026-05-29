#ifndef RTT_MANAGER_H
#define RTT_MANAGER_H

#include "io_transport.h"

#include <QLibrary>
#include <QMutex>

#ifdef Q_OS_WIN
#include <windows.h>
#endif

/**
 * @brief J-Link RTT 管理器 - 动态加载 JLinkARM.dll
 */
class RttManager : public IOTransport {
    Q_OBJECT

public:
    explicit RttManager(QObject *parent = nullptr);
    ~RttManager() override;

    QList<QPair<QString, QString>> getAvailableDevices() override;

    // Get J-Link serial number if connected
    QString getSerialNumber() const;

    // Check if J-Link SDK is available
    bool isJLinkAvailable() const { return m_jlinkAvailable; }

    // Read from RTT buffer (called by reader thread)
    int readRTT(int bufferIdx, void *data, int size);

protected:
    bool connectImpl(const QVariantMap &params) override;
    void closeResource() override;
    bool sendBytes(const QByteArray &data) override;

private:
    // J-Link SDK function types
    using JLINK_Open = int (*)(int);
    using JLINK_Close = void (*)();
    using JLINK_IsOpen = int (*)();
    using JLINK_GetSN = int (*)();
    using JLINK_GetProductName = int (*)(char*, int);
    using JLINK_GetNumEmulators = int (*)();
    using JLINK_GetEmuList = int (*)(void*, int);
    using JLINK_TIF_Select = int (*)(int);
    using JLINK_SetSpeed = int (*)(int);
    using JLINK_Connect = int (*)();
    using JLINK_Reset = int (*)(int, int);
    using JLINK_RTT_Start = int (*)(unsigned int, unsigned int, unsigned int);
    using JLINK_RTT_Stop = int (*)();
    using JLINK_RTT_Read = int (*)(unsigned int, void*, unsigned int);
    using JLINK_RTT_Write = int (*)(unsigned int, const void*, unsigned int);
    using JLINK_GetDLLVersion = int (*)();
    using JLINK_GetEmuCaps = int (*)();
    using JLINK_GetHardwareVersion = int (*)();
    using JLINK_GetFirmwareString = int (*)(char*, int);

    // Load J-Link SDK
    bool loadJLinkSDK();

    // Function pointers
    JLINK_Open fpOpen = nullptr;
    JLINK_Close fpClose = nullptr;
    JLINK_IsOpen fpIsOpen = nullptr;
    JLINK_GetSN fpGetSN = nullptr;
    JLINK_GetProductName fpGetProductName = nullptr;
    JLINK_GetNumEmulators fpGetNumEmulators = nullptr;
    JLINK_GetEmuList fpGetEmuList = nullptr;
    JLINK_TIF_Select fpTIF_Select = nullptr;
    JLINK_SetSpeed fpSetSpeed = nullptr;
    JLINK_Connect fpConnect = nullptr;
    JLINK_Reset fpReset = nullptr;
    JLINK_RTT_Start fpRTT_Start = nullptr;
    JLINK_RTT_Stop fpRTT_Stop = nullptr;
    JLINK_RTT_Read fpRTT_Read = nullptr;
    JLINK_RTT_Write fpRTT_Write = nullptr;
    JLINK_GetDLLVersion fpGetDLLVersion = nullptr;
    JLINK_GetEmuCaps fpGetEmuCaps = nullptr;
    JLINK_GetHardwareVersion fpGetHardwareVersion = nullptr;
    JLINK_GetFirmwareString fpGetFirmwareString = nullptr;

    QLibrary *m_jlinkLib = nullptr;
    bool m_jlinkAvailable = false;
    bool m_connected = false;
    int m_serialNo = 0;

    // RTT buffer
    static const int RTT_BUFFER_SIZE = 8192;
    char m_rttBuffer[RTT_BUFFER_SIZE];

    mutable QMutex m_mutex;
};

#endif // RTT_MANAGER_H
