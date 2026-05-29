# CLAUDE.md — Z_COM Qt C++ Edition

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

Z_COM is a Qt C++ serial communication debugging tool supporting three transport modes: Serial (QSerialPort), J-Link RTT (J-Link SDK), and TCP/UDP Socket (QTcpSocket/QUdpSocket). C++17, Qt 6, CMake build system.

## Commands

```bash
# Configure and build
cmake -B build -G "Visual Studio 17 2022"
cmake --build build --config Release

# Or with Ninja
cmake -B build -G Ninja
cmake --build build

# Run
./build/src/Z_COM.exe

# Clean
rmdir /s /q build
```

No CI, lint, format, or typecheck configuration exists.

## Architecture

```
src/
  main.cpp                -- Entry point, multi-instance detection via QLockFile
  version.h               -- VERSION, APP_NAME constants
  core/                   -- Business logic
    config_manager.h/cpp  -- JSON config load/save with QJsonDocument + debounce
    data_handler.h/cpp    -- QByteArray <-> HEX/ASCII, control char display (U+2400~U+2421)
    logger.h/cpp          -- Thread-safe buffered daily log writer (QMutex)
    ansi_parser.h/cpp     -- ANSI CSI SGR -> HTML <span>
    extended_send_manager.h/cpp -- Multi-item send orchestration (single/multi/loop)
  io/                     -- Transport layer
    io_transport.h/cpp    -- Abstract base: IOTransport(QObject) with shared signals
    serial_manager.h/cpp  -- QSerialPort wrapper
    serial_reader.h/cpp   -- QThread serial reader with 50ms frame-batching
    rtt_manager.h/cpp     -- J-Link SDK wrapper (TODO: dynamic loading)
    rtt_reader.h/cpp      -- QThread RTT reader
    socket_manager.h/cpp  -- QTcpServer/QTcpSocket/QUdpSocket wrapper
    socket_reader.h/cpp   -- QThread socket reader (event-driven, no select())
  windows/                -- UI classes (import generated ui_*.h, add logic only)
    main_window.h/cpp     -- MainWindow, data flow hub
    serial_settings_dialog.h/cpp
    extended_send_widget.h/cpp
    extended_send_editor_dialog.h/cpp
    receive_display_handler.h/cpp
    status_bar_controller.h/cpp
ui/                       -- .ui source files (Qt Designer, language-agnostic)
config/                   -- Runtime JSON config (settings, extended_send, presets, dap_devices)
```

**Data flow**: Reader QThread emits `dataReceived(QByteArray)` -> `ReceiveDisplayHandler::onDataReceived()` -> batched in `m_pendingData` with flush timer -> `appendData()` -> display + log.

**IO switching**: `MainWindow::currentIO()` returns the active manager based on `m_ioMode` (Serial/Rtt/Socket). All three managers share the same `IOTransport` signal interface.

## Key Conventions

- **Build system**: CMake with `CMAKE_AUTOMOC`, `CMAKE_AUTOUIC`, `CMAKE_AUTORCC` enabled. Qt 6 `find_package` for Widgets, SerialPort, Network.
- **UI workflow**: `.ui` files are the single source of truth for layout. Never hand-edit `ui_*.h`. Signal connections and business logic go in `src/windows/` runtime classes only.
- **Multi-instance**: QLockFile-based detection. Instance 1 uses `config/` and `logs/`; instance N>1 uses `instance_N/config/` and `instance_N/logs/`.
- **Frame timeout**: `SerialReaderThread` batches incoming bytes with a 50ms timeout, configurable in settings.
- **Config debounce**: `ConfigManager` saves with 500ms debounce via QTimer.
- **Display pruning**: `ReceiveDisplayHandler` limits display to 5000 lines, checks every 50 appends.
- **Cross-platform**: Use Qt APIs. Platform-specific code only in `#ifdef Q_OS_WIN` blocks.
- **Socket mode**: Event-driven via QTcpServer/QTcpSocket/QUdpSocket (no select() polling).
- **J-Link RTT**: Currently stubbed. TODO: dynamic load JLinkARM.dll.
- **Extended send**: `sortOrder=0` means excluded from sending. Duplicate order values trigger a warning.
