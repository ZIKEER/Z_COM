#include <QApplication>
#include <QMessageBox>
#include <QFile>
#include <QDir>
#include <QStandardPaths>
#include <QSharedMemory>
#include <QLockFile>
#include <QFileInfo>

#ifdef Q_OS_WIN
#include <windows.h>
#endif

#include "version.h"
#include "windows/main_window.h"

static int getInstanceId() {
    // Use a lock file based on executable path to detect multiple instances
    QString appPath = QCoreApplication::applicationFilePath();
    QString lockDir = QCoreApplication::applicationDirPath() + "/locks";
    QDir().mkpath(lockDir);

    // Create a hash of the app path for the lock file name
    QString hash = QString::number(qHash(appPath), 16);
    QString lockPath = lockDir + "/" + hash + ".lock";

    // Try to lock incrementally (instance 1, 2, 3...)
    for (int i = 1; i <= 10; ++i) {
        QString instanceLockPath = lockPath + "." + QString::number(i);
        QLockFile *lockFile = new QLockFile(instanceLockPath);
        lockFile->setStaleLockTime(0);
        if (lockFile->tryLock()) {
            // Keep the lock file alive - store it statically
            static QLockFile *persistentLocks[10] = {};
            persistentLocks[i - 1] = lockFile;
            return i;
        }
        delete lockFile;
    }
    return 1; // fallback
}

static void setWindowsAppId(int instanceId) {
#ifdef Q_OS_WIN
    QString appId = QString("Z_COM.Instance.%1").arg(instanceId);
    // Set AppUserModelID so each instance gets a distinct taskbar icon
    typedef HRESULT (WINAPI *SetCurrentProcessExplicitAppUserModelIDFunc)(PCWSTR);
    auto func = reinterpret_cast<SetCurrentProcessExplicitAppUserModelIDFunc>(
        GetProcAddress(GetModuleHandleW(L"shell32.dll"),
                       "SetCurrentProcessExplicitAppUserModelID"));
    if (func) {
        func(reinterpret_cast<PCWSTR>(appId.utf16()));
    }
#endif
}

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);
    app.setApplicationName(Version::appName());
    app.setApplicationVersion(Version::versionString());

    // Set application icon
    QString iconPath = QCoreApplication::applicationDirPath() + "/resources/icons/serial_comm.ico";
    if (QFile::exists(iconPath)) {
        app.setWindowIcon(QIcon(iconPath));
    }

    int instanceId = getInstanceId();
    setWindowsAppId(instanceId);

    MainWindow mainWindow(instanceId);
    mainWindow.show();

    return app.exec();
}
