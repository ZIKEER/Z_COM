# 跨平台设计与验收

## 实现原则

- 文件、目录、锁、网络和时间处理使用 Rust 跨平台库。
- 路径通过 `Path` / `PathBuf` 组合，不手工拼接平台分隔符。
- 不使用注册表、Named Mutex 等 Windows 专属机制作为核心能力。
- SEGGER SDK 保持可选动态依赖：Windows 加载 `JLink_x64.dll`，Linux 加载 `libjlinkarm.so`。
- Linux 串口或 USB 权限不足时显示具体设备路径和权限提示，程序不自动提权或修改系统配置。
- 绿色版必须放在普通可写目录；配置、日志、锁和多实例目录都位于程序同级目录。
- 程序目录不可写时必须在启动阶段明确报错，不能继续运行后静默丢失配置或日志。
- Windows 与 Linux 分别原生构建，未经目标系统运行验证的交叉编译产物不作为正式版本。

## 平台产物

| 平台 | 目标产物 | 外部运行条件 |
|---|---|---|
| Windows 10/11 x86_64 | `dist/Z_COM_V版本号/Z_COM.exe` | WebView2；使用 J-Link 时安装 SEGGER Software Pack 或选择 SDK 目录 |
| Linux x86_64 | `dist/linux-x86_64/Z_COM_V版本号/Z_COM` | WebKitGTK/GTK 运行库；串口和 USB 权限；使用 J-Link 时安装 Linux SDK |

## 当前验证状态

- Windows 开发、检查、测试和绿色版打包已通过。
- Ubuntu 24.04 WSL2 原生依赖安装、前端检查、前端构建、11 项 Rust 测试和 release 打包已通过。
- WSLg 软件渲染模式下完成启动存活测试，产物依赖无缺失。
- WSL 不能代替真实 Linux 桌面、串口、USB 探针和 SEGGER SDK 硬件验收。

## 最低验收矩阵

### 两个平台通用

- 启动、退出、窗口缩放、区域拖动、设置持久化。
- 多实例编号、窗口标题和数据目录隔离。
- 串口枚举、运行中修改参数、ASCII/HEX 收发、自动发送、断开恢复和按日期日志。
- TCP/UDP Client/Server、本机地址枚举和 TCP 新连接替换提示。
- 有 J-Link SDK 与无 SDK 两种环境启动。
- SEGGER J-Link RTT 和至少一种 probe-rs 探针 RTT。

### Linux 专项

- 正常权限与权限不足场景。
- X11 与 Wayland 桌面启动。
- 真实串口和真实 USB 探针。
- SEGGER Linux Software Pack 的动态库发现与手动选择。

### Windows 专项

- WebView2 可用与缺失场景。
- J-Link SDK 自动发现与手动选择。
- 绿色版目录迁移后配置、日志和多实例数据保持正确。

Linux 依赖、权限和发布步骤见 [Linux 构建与运行](linux.md)。
