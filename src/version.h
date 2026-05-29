#ifndef VERSION_H
#define VERSION_H

#include <QString>

namespace Version {
    constexpr int MAJOR = 0;
    constexpr int MINOR = 1;
    constexpr int PATCH = 0;

    inline QString versionString() {
        return QStringLiteral("%1.%2.%3").arg(MAJOR).arg(MINOR).arg(PATCH);
    }

    inline QString appName() {
        return QStringLiteral("Z_COM");
    }

    inline QString appDescription() {
        return QStringLiteral("Qt C++ Serial Communication Tool");
    }
}

#endif // VERSION_H
