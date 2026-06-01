# Z_COM 工具设计方案

## 1. 项目概述

### 1.1 项目背景
Z_COM 是嵌入式开发、硬件调试中常用的工具软件，用于与设备进行通信。随着调试场景的多样化，单一串口通道已不能满足需求，需支持多种物理通道，并提供灵活的协议扩展能力。本项目旨在开发一个功能完善、界面友好的多通道通信调试工具，满足日常开发调试需求。

### 1.2 项目目标
- 提供多种通信通道（串口、J-Link RTT、TCP/UDP Socket）
- 支持多种数据格式显示（HEX、ASCII、HEX+ASCII 混合、原始数据）
- 提供数据日志和扩展发送功能
- 提供协议解析插件系统，支持自定义协议
- 界面紧凑，适合工具软件使用场景

### 1.3 设计范围
本方案覆盖核心框架设计和功能规划。具体模块的详细设计（如各协议解析器实现、通道适配器细节）在各自模块的设计文档中描述。

---

## 2. 功能需求

### 2.1 多通道连接
支持以下通信方式：

#### 2.1.1 串口通信（Serial）
- **串口选择**：显示可用串口列表，包括串口号和串口名称
- **波特率**：支持常见波特率（9600-921600）
- **数据位**：5, 6, 7, 8 位
- **停止位**：1, 1.5, 2 位
- **校验位**：None, Even, Odd, Mark, Space
- **流控制**：None, RTS/CTS, DTR/DSR, XON/XOFF
- **串口保护锁**：串口打开时禁用刷新按钮和串口选择，防止误操作
- **配置热更新**：修改波特率等参数后立即生效，无需关闭/重新打开串口

#### 2.1.2 J-Link RTT
- 通过 J-Link 调试器进行 RTT 通信
- 支持指定 Target Device 和 RTT Channel

#### 2.1.3 TCP/UDP Socket
- 支持 TCP 和 UDP 协议
- 支持 Server 和 Client 两种模式
- 支持自定义 IP 地址和端口
- 支持多客户端连接管理

### 2.2 数据收发
- **发送格式**：ASCII 和 HEX
- **显示模式**：
  - HEX 模式：以十六进制格式显示
  - ASCII 模式：以 ASCII 字符格式显示
  - HEX+ASCII 混显模式：同时显示两种格式，便于对照分析
  - 原始数据模式（Raw）：显示未处理的原始字节流（规划中）
- **时间戳显示**：可选择是否在接收数据前添加时间戳（规划中）
- **自动滚动**：可控制接收区是否自动滚动
- **回车换行**：可选择发送时是否添加回车换行
- **自动发送**：支持定时自动发送，可设置间隔
- **数据屏蔽**：支持过滤空帧数据，避免无效数据干扰（规划中）

### 2.3 数据日志
- **自动存储**：启动后自动记录所有收发数据
- **存储格式**：同时存储 ASCII 和 HEX 格式
- **日志文件**：按日期自动创建，存储在 `logs` 目录下
- **文件格式**：`YYYY-MM-DD_HHMMSS.log`
- **日志轮转**：单个文件超过 50MB 自动创建新文件
- **日志转换**：内置日志格式转换工具（工具菜单）

### 2.4 扩展发送（预设管理）
- **数据管理**：支持添加、删除、清空、上移、下移数据条目
- **数据格式**：支持 HEX 和字符串两种格式
- **发送模式**：支持单条发送、多条顺序发送、循环发送
- **延时控制**：每条数据可设置独立延时（0-60000ms）
- **配置管理**：支持导入导出配置（JSON格式）
- **预设管理**：支持启用/禁用预设条目，支持批量操作（规划中）

### 2.5 参数持久化
- **自动保存**：配置修改时自动保存（500ms 防抖）
- **保存内容**：通信模式、通道参数、显示模式、发送模式、自动滚动、自动发送间隔、窗口布局
- **配置文件**：存储在 `config/settings.json`
- **扩展发送配置**：存储在 `config/extended_send.json`
- **多实例隔离**：多实例运行时，各实例使用独立的配置和日志目录

### 2.6 协议解析插件系统（规划中）

#### 2.6.1 概述
设计一个协议解析插件系统，允许用户通过配置文件定义自定义协议的解析规则，无需编写代码即可支持新的协议格式。插件定义发送数据的解析逻辑和接收数据的解析逻辑，实现数据的自动化分析。

#### 2.6.2 设计目标
- **易用性**：通过 JSON 配置文件定义协议，无需编写代码
- **灵活性**：支持多种数据类型、字节序、校验方式
- **可扩展性**：支持动态加载/卸载协议插件
- **双向解析**：发送和接收数据使用独立的解析逻辑
- **错误容错**：解析失败时显示原始数据，不影响正常通信

#### 2.6.3 功能需求

**协议定义（JSON 配置）：**
- 协议名称、版本、描述
- 接收数据解析：正则表达式匹配 + 字段提取
- 发送数据构建：字段组合 + 格式化输出
- 字段类型：整数、浮点、枚举、位域、字节数组、字符串
- 字节序支持：Big-Endian、Little-Endian
- 校验和：SUM、CRC16、CRC32、XOR、自定义

**UI 集成：**
- 协议选择下拉框
- 发送时显示协议字段输入表单
- 接收时显示解析后的字段结果
- 解析失败时回退显示原始数据

#### 2.6.4 插件生命周期
1. 加载：程序启动时扫描 `protocols/` 目录，加载所有 JSON 配置文件
2. 卸载：程序退出时释放插件资源
3. 切换：用户通过协议选择器切换当前协议
4. 错误处理：协议加载失败时在日志中记录错误，不影响其他协议

---

## 3. 界面设计

### 3.1 窗口布局
```
┌───────────────────────────────────────────────────────────────────┐
│ 菜单栏：文件 | 编辑 | 工具 | 帮助                                  │
├───────────────────────────────────────────────────────────────────┤
│ 顶部工具栏                                                        │
│ [刷新端口] 端口:[COMx ▼] 波特率:[115200 ▼]  ║ [打开端口][更多设置][扩展发送] │
├───────────────────────────────────────────────────────────────────┤
│ 接收区域                                          │ 扩展发送区域   │
│ [HEX] [ASCII] [HEX+ASCII] [自动滚动] [清空]        │ （默认隐藏）   │
│ ┌─────────────────────────────────────────────┐   │              │
│ │                                             │   │              │
│ │              接收数据显示区域                  │   │              │
│ │                                             │   │              │
│ └─────────────────────────────────────────────┘   │              │
├───────────────────────────────────────────────────┴──────────────┤
│ 发送区域                                                          │
│ [ASCII] [HEX] [添加回车换行]         [自动发送] [1000]ms [发送]     │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ 输入要发送的数据...                                            │ │
│ └──────────────────────────────────────────────────────────────┘ │
├───────────────────────────────────────────────────────────────────┤
│                                            已连接 | 发送: N 字节 | 接收: N 字节 │
└───────────────────────────────────────────────────────────────────┘
```

### 3.2 组件说明

#### 3.2.1 接收区域（receiveGroup）
- **标题**：接收区域
- **工具栏**：HEX、ASCII、HEX+ASCII 单选按钮，自动滚动复选框，清空按钮
- **数据显示区**：QTextEdit 组件，只读，Consolas 10pt 等宽字体
- **颜色区分**（常量定义在 `status_bar_controller.py`）：
  - 时间戳：淡青色 `#00CED1`
  - 箭头和标识：黑色 `#000000`，加粗
  - 数据内容：黑色 `#000000`
- **显示裁剪**：超过 5000 行时裁剪前 2500 行，每 50 次追加检查一次
- **批量刷新**：接收数据 50ms 窗口批量拼包后统一显示，减少 UI 刷新频率

#### 3.2.2 扩展发送区域（预设面板）
- **位置**：显示区域右侧
- **默认隐藏**：点击扩展发送按钮显示/隐藏
- **功能**：表格形式显示多条数据，支持批量发送
- **分割比例**：接收:扩展 = 7:0（隐藏时），700:320（显示时）

#### 3.2.3 发送区域（sendGroup）
- **结构**：单列上下布局（sendCenterLayout）
  - 第一行（sendConfigLayout）：ASCII、HEX、添加回车换行、弹性间距、自动发送、间隔时间、ms 标签、发送按钮
  - 第二行：发送输入框（QTextEdit，placeholder: "输入要发送的数据..."）
- **高度限制**：最小 96px，最大 156px

#### 3.2.4 顶部工具栏（statusBarToolbar）
- **左侧**：刷新端口按钮、端口标签+下拉框、波特率/IP+端口（QStackedWidget 根据通道模式切换）
- **右侧**：打开端口按钮、更多设置按钮、扩展发送按钮
- **高度**：固定最大 36px
- **打开端口按钮**：断开时红底白字，连接后绿底白字

#### 3.2.5 状态栏（QStatusBar）
- **右侧永久组件**：连接状态标签、分隔符、已发送字节、分隔符、已接收字节
- **状态标签**：固定宽度150px，避免文本长度变化影响布局

### 3.3 窗口尺寸
- **默认大小**：890 x 685
- **最小大小**：可调整，各组件有最小尺寸限制

### 3.4 分割器布局
- **主分割器**（垂直）：上部分显示区域 + 下部分发送区域，比例 7:1
- **顶部分割器**（水平）：左边显示区域 + 右边扩展发送区域
- **布局持久化**：分割器位置自动保存和恢复

### 3.5 菜单功能
- **文件**：退出
- **编辑**：清空接收、清空发送
- **工具**：更多设置、扩展发送（切换显示）、日志转换
- **帮助**：关于

---

## 4. 技术架构

### 4.1 技术选型
- **开发语言**：Python 3.13+
- **GUI 框架**：PySide6
- **串口通信**：pyserial
- **J-Link RTT**：pylink-square
- **Socket 通信**：Python socket 模块
- **配置存储**：JSON 文件
- **日志存储**：文本日志文件

### 4.2 项目结构
```
Z_COM/
├── config/                     # 配置目录
│   ├── settings.json          # 主配置文件
│   ├── extended_send.json     # 扩展发送配置
│   ├── presets.json           # 预设配置
│   └── dap_devices.json       # DAP 设备配置
├── doc/                        # 文档目录
│   ├── design.md              # 设计方案文档
│   ├── changelog.txt          # 变更日志
│   └── rtt_integration_plan.md  # RTT 集成方案
├── ui/                        # UI 文件目录（.ui 为源，Ui_*.py 为生成代码）
│   ├── main_window.ui / Ui_main_window.py
│   ├── serial_settings_dialog.ui / Ui_serial_settings_dialog.py
│   ├── extended_send_widget.ui / Ui_extended_send_widget.py
│   └── extended_send_editor_dialog.ui / Ui_extended_send_editor_dialog.py
├── src/                       # 源代码目录
│   ├── main.py               # 程序入口，多实例检测
│   ├── version.py            # 版本信息
│   ├── build_info.py         # 编译时间（自动生成）
│   ├── core/                 # 核心业务逻辑
│   │   ├── config_manager.py    # 配置管理
│   │   ├── data_handler.py      # 数据处理
│   │   ├── logger.py            # 日志管理
│   │   ├── ansi_parser.py       # ANSI 转义解析
│   │   └── extended_send_manager.py  # 扩展发送管理
│   ├── io/                   # 通信通道层
│   │   ├── io_transport.py      # 抽象基类
│   │   ├── serial_manager.py    # 串口管理器
│   │   ├── serial_reader.py     # 串口读取线程
│   │   ├── rtt_manager.py       # RTT 管理器
│   │   ├── rtt_reader.py        # RTT 读取线程
│   │   ├── socket_manager.py    # Socket 管理器
│   │   └── socket_reader.py     # Socket 读取线程
│   └── windows/              # UI 逻辑层
│       ├── main_window.py       # 主窗口
│       ├── receive_display_handler.py  # 接收区显示管理
│       ├── status_bar_controller.py    # 状态栏管理
│       ├── serial_settings_dialog.py  # 串口设置对话框
│       ├── extended_send_widget.py    # 扩展发送面板
│       └── extended_send_editor_dialog.py  # 扩展发送编辑对话框
├── scripts/                   # 工具脚本
│   └── log_converter.py      # 日志格式转换
├── resources/                 # 资源文件
│   └── icons/               # 图标（serial_comm.ico, serial_comm.svg）
├── tests/                    # 测试代码
├── logs/                     # 日志目录
├── locks/                    # 多实例锁文件目录
├── run.py                    # 启动脚本
├── pack.py                   # PyInstaller 打包脚本
├── pack_nuitka.py            # Nuitka 打包脚本
├── requirements.txt          # 依赖包
└── README.md                 # 项目说明
```

### 4.3 核心类设计

#### 4.3.1 通道层

**IOTransport**（抽象基类）
- 定义统一的通信接口，所有通道管理器继承此类
- 信号：`data_received(bytes)`、`connection_changed(bool)`、`error_occurred(str)`
- 抽象方法：`get_available_devices()`、`_connect_impl()`、`_close_resource()`、`_send_bytes()`
- 生命周期方法：`open_connection()`、`close_connection()`、`send_data()`、`update_settings()`

**SerialManager**
- 管理串口通信，继承 IOTransport
- 支持数据帧自动拼接（50ms超时）
- 支持配置热更新（reconfigure 方法）

**RttManager**
- 管理 J-Link RTT 通信，继承 IOTransport
- 通过 pylink-square 库与 J-Link 交互

**SocketManager**
- 管理 TCP/UDP Socket 通信，继承 IOTransport
- 支持 Server/Client 模式
- 支持多客户端连接管理
- 信号：`client_event(str)` 通知客户端连接/断开事件

#### 4.3.2 核心业务

**DataHandler**
- 数据格式转换：HEX/ASCII 互转
- 格式化显示文本，支持 HEX、ASCII、MIXED 三种模式
- 控制字符映射：0x00-0x1F、0x7F 映射到 Unicode Control Pictures（U+2400-U+2421）

**Logger**
- 线程安全的缓冲日志写入
- 按日期自动创建日志文件
- 同时记录 ASCII 和 HEX 格式
- 日志轮转：单文件超过 50MB 自动创建新文件
- 刷新间隔：1000ms

**ConfigManager**
- JSON 配置文件的加载和保存
- 支持默认配置合并
- 配置保存通过主窗口 500ms 防抖定时器触发

**ExtendedSendManager**
- 扩展发送数据管理
- 支持单条/多条/循环发送
- 信号：`send_started`、`send_finished`、`send_progress(int,int)`、`data_sent(bytes)`、`error_occurred(str)`、`items_changed()`
- 配置导入导出（JSON 格式）

**AnsiParser**
- ANSI CSI SGR 转义序列解析
- 转换为 HTML `<span>` 标签用于富文本显示

#### 4.3.3 协议插件层（规划中）

**ProtocolBase**（协议插件基类）
- 定义协议插件的标准接口
- 信号：`protocol_data_ready(dict)` 发送解析结果
- 方法：`parse_data(bytes)` 解析接收数据、`build_frame(dict)` 构建发送帧

**ProtocolPanelWidget**（协议面板组件）
- 协议字段输入表单
- 解析结果显示区域
- 支持动态 UI 生成

### 4.4 数据流架构
```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ SerialReader │    │  RttReader   │    │ SocketReader │
│   (QThread)  │    │  (QThread)   │    │  (QThread)   │
└──────┬───────┘    └──────┬───────┘    └──────┬───────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │ data_received(bytes)
                           ▼
              ┌─────────────────────────────────┐
              │ IOTransport (抽象基类)           │
              │ open_connection / close_connection│
              │ send_data / update_settings      │
              └────────────┬────────────────────┘
                           │
                           ▼
              ┌─────────────────────────────────┐
              │ ReceiveDisplayHandler            │
              │ on_data_received(bytes)          │
              │         │                        │
              │         ▼                        │
              │ _pending_data (bytearray)        │  ◄── 批量缓冲
              │         │                        │
              │  flush timer (50ms)              │
              │         │                        │
              │         ▼                        │
              │ _flush_pending()                 │
              │ append_data()                    │
              │    ┌────┴────┐                   │
              │    ▼         ▼                   │
              │ Display   Logger                 │
              │ (QTextEdit) (文件)               │
              └─────────────────────────────────┘
```

### 4.5 多实例机制
- 基于文件锁的多实例检测
- 通过 SHA256 哈希生成唯一的锁文件名
- Windows 使用 `msvcrt.locking`，Linux 使用 `fcntl.flock`
- 实例 1 使用 `config/` 和 `logs/`
- 实例 N>1 使用 `instance_N/config/` 和 `instance_N/logs/`
- 锁文件句柄保持打开，进程退出时内核自动释放（崩溃安全）

---

## 5. 使用说明

### 5.1 连接设备
1. 从顶部工具栏端口下拉框选择串口
2. 选择波特率或点击更多设置配置详细参数
3. 点击顶部工具栏的打开端口按钮

### 5.2 串口保护锁
- 串口打开后，刷新按钮和串口选择下拉框自动禁用
- 防止在通信过程中误操作导致连接中断
- 串口关闭后自动恢复可用状态

### 5.3 配置热更新
- 串口打开时可随时修改波特率、数据位、停止位、校验位等参数
- 修改后立即生效，无需手动关闭/重新打开串口
- 点击"更多设置"修改参数后自动应用到当前连接

### 5.4 接收数据
- 顶部选择显示模式：HEX、ASCII 或 HEX+ASCII
- 接收数据会自动显示在显示区域
- 支持自动滚动和清空
- 快捷键：Ctrl+D 清空接收区

### 5.5 发送数据
- 发送区域选择格式：ASCII 或 HEX
- 在发送输入框输入数据
- 点击发送按钮或按回车发送
- 可选择添加回车换行
- 快捷键：Ctrl+Back 清空发送输入框

### 5.6 扩展发送
- 点击顶部工具栏扩展发送按钮或菜单"工具 → 扩展发送"显示面板
- 点击添加按钮添加数据条目
- 直接在表格中编辑数据内容和注释
- 支持批量发送和循环发送
- 支持导入导出配置

### 5.7 日志管理
- 所有收发数据自动记录到日志文件
- 日志文件按日期自动创建
- 存储在 `logs` 目录下
- 菜单"工具 → 日志转换"可进行日志格式转换

### 5.8 协议解析（规划中）
- 从状态栏协议下拉框选择协议
- 接收数据自动按协议规则解析显示
- 发送时显示协议字段输入表单
- 解析失败时回退显示原始数据

---

## 6. 编译说明

### 编译 UI 文件
```bash
pyside6-uic ui/main_window.ui -o ui/Ui_main_window.py
pyside6-uic ui/serial_settings_dialog.ui -o ui/Ui_serial_settings_dialog.py
pyside6-uic ui/extended_send_widget.ui -o ui/Ui_extended_send_widget.py
pyside6-uic ui/extended_send_editor_dialog.ui -o ui/Ui_extended_send_editor_dialog.py
```

### 运行程序
```bash
python run.py
```

### 运行测试
```bash
pytest
pytest tests/test_logger.py           # 单文件
pytest tests/test_logger.py::test_fn  # 单用例
```

### 打包
```bash
python pack.py            # PyInstaller -> dist/
python pack_nuitka.py     # Nuitka -> dist_nuitka/
```

---

**文档版本**：v3.0
**最后更新**：2026年6月1日
