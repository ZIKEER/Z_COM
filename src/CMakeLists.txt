set(CMAKE_AUTOUIC_SEARCH_PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../ui)

set(SOURCES
    main.cpp
    core/config_manager.cpp
    core/data_handler.cpp
    core/logger.cpp
    core/ansi_parser.cpp
    core/extended_send_manager.cpp
    io/io_transport.cpp
    io/serial_manager.cpp
    io/serial_reader.cpp
    io/socket_manager.cpp
    io/socket_reader.cpp
    io/rtt_manager.cpp
    io/rtt_reader.cpp
    windows/main_window.cpp
    windows/serial_settings_dialog.cpp
    windows/extended_send_widget.cpp
    windows/extended_send_editor_dialog.cpp
    windows/receive_display_handler.cpp
    windows/status_bar_controller.cpp
)

set(HEADERS
    version.h
    core/config_manager.h
    core/data_handler.h
    core/logger.h
    core/ansi_parser.h
    core/extended_send_manager.h
    io/io_transport.h
    io/serial_manager.h
    io/serial_reader.h
    io/socket_manager.h
    io/socket_reader.h
    io/rtt_manager.h
    io/rtt_reader.h
    windows/main_window.h
    windows/serial_settings_dialog.h
    windows/extended_send_widget.h
    windows/extended_send_editor_dialog.h
    windows/receive_display_handler.h
    windows/status_bar_controller.h
)

set(UI_FILES
    ${CMAKE_CURRENT_SOURCE_DIR}/../ui/main_window.ui
    ${CMAKE_CURRENT_SOURCE_DIR}/../ui/serial_settings_dialog.ui
    ${CMAKE_CURRENT_SOURCE_DIR}/../ui/extended_send_widget.ui
    ${CMAKE_CURRENT_SOURCE_DIR}/../ui/extended_send_editor_dialog.ui
)

# Collect UI headers for AUTOUIC
qt6_wrap_ui(UI_HEADERS ${UI_FILES})

add_executable(${PROJECT_NAME} ${SOURCES} ${HEADERS} ${UI_HEADERS})

target_include_directories(${PROJECT_NAME} PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_BINARY_DIR}  # for generated ui_*.h headers
)

target_link_libraries(${PROJECT_NAME} PRIVATE
    Qt6::Widgets
    Qt6::SerialPort
    Qt6::Network
)

# Copy config and resources to build directory
add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_SOURCE_DIR}/config
        $<TARGET_FILE_DIR:${PROJECT_NAME}>/config
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_SOURCE_DIR}/resources
        $<TARGET_FILE_DIR:${PROJECT_NAME}>/resources
    COMMENT "Copying config and resources to build directory"
)

# Deploy Qt DLLs after build
find_program(WINDEPLOYQT_EXECUTABLE windeployqt HINTS "${Qt6_DIR}/../../../bin")
if(WINDEPLOYQT_EXECUTABLE)
    add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
        COMMAND "${WINDEPLOYQT_EXECUTABLE}"
            --no-translations
            --no-opengl-sw
            --no-system-d3d-compiler
            "$<TARGET_FILE:${PROJECT_NAME}>"
        # Remove unnecessary DirectX DLLs
        COMMAND ${CMAKE_COMMAND} -E rm -f
            "$<TARGET_FILE_DIR:${PROJECT_NAME}>/dxcompiler.dll"
            "$<TARGET_FILE_DIR:${PROJECT_NAME}>/dxil.dll"
        COMMENT "Deploying Qt runtime libraries"
    )
endif()
