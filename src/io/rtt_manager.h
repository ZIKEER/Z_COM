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
    // J-Link SDK function types (JLINKARM_ prefix)
    using JLINKARM_Open = int (*)(int);  // int serial_no
    using JLINKARM_Close = void (*)();
    using JLINKARM_IsOpen = int (*)();
    using JLINKARM_GetSN = int (*)();
    using JLINKARM_EMU_GetProductName = int (*)(char*, int);
    using JLINKARM_EMU_GetNumDevices = int (*)();
    using JLINKARM_EMU_GetList = int (*)(void*, int);
    using JLINKARM_TIF_Select = int (*)(int);
    using JLINKARM_SetSpeed = void (*)(int);
    using JLINKARM_Connect = int (*)();
    using JLINKARM_SelDevice = int (*)(const char*);
    using JLINKARM_Reset = int (*)(int, int);
    using JLINKARM_Halt = void (*)();
    using JLINKARM_Go = void (*)();
    using JLINKARM_ReadMem = int (*)(unsigned int, void*, int);
    using JLINKARM_WriteMem = int (*)(unsigned int, const void*, int);
    using JLINKARM_GetDLLVersion = int (*)();
    using JLINKARM_GetEmuCaps = int (*)();
    using JLINKARM_GetHardwareVersion = int (*)();
    using JLINKARM_GetFirmwareString = int (*)(char*, int);

    // RTT functions (JLINK_ prefix)
    using JLINK_RTTERMINAL_Control = int (*)(int, void*);
    using JLINK_RTTERMINAL_Read = int (*)(int, void*, int);
    using JLINK_RTTERMINAL_Write = int (*)(int, const void*, int);

    // Load J-Link SDK
    bool loadJLinkSDK();

    // Perform connection in background thread
    void performConnect();
    QVariantMap m_connectParams;

    // Function pointers
    JLINKARM_Open fpOpen = nullptr;
    JLINKARM_Close fpClose = nullptr;
    JLINKARM_IsOpen fpIsOpen = nullptr;
    JLINKARM_GetSN fpGetSN = nullptr;
    JLINKARM_EMU_GetProductName fpGetProductName = nullptr;
    JLINKARM_EMU_GetNumDevices fpGetNumEmulators = nullptr;
    JLINKARM_EMU_GetList fpGetEmuList = nullptr;
    JLINKARM_TIF_Select fpTIF_Select = nullptr;
    JLINKARM_SetSpeed fpSetSpeed = nullptr;
    JLINKARM_Connect fpConnect = nullptr;
    JLINKARM_SelDevice fpSelDevice = nullptr;
    JLINKARM_Reset fpReset = nullptr;
    JLINKARM_Halt fpHalt = nullptr;
    JLINKARM_Go fpGo = nullptr;
    JLINKARM_ReadMem fpReadMem = nullptr;
    JLINKARM_WriteMem fpWriteMem = nullptr;
    JLINKARM_GetDLLVersion fpGetDLLVersion = nullptr;
    JLINKARM_GetEmuCaps fpGetEmuCaps = nullptr;
    JLINKARM_GetHardwareVersion fpGetHardwareVersion = nullptr;
    JLINKARM_GetFirmwareString fpGetFirmwareString = nullptr;

    // RTT function pointers
    JLINK_RTTERMINAL_Control fpRTT_Control = nullptr;
    JLINK_RTTERMINAL_Read fpRTT_Read = nullptr;
    JLINK_RTTERMINAL_Write fpRTT_Write = nullptr;

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
