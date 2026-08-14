# SEGGER / probe-rs 双 RTT 后端

## 结论

Rust 版本按探针类型自动选择后端：J-Link 使用用户机器已安装的 SEGGER J-Link SDK，保留 SEGGER 驱动以及与 Keil、Ozone 等工具兼容的链路；CMSIS-DAP、ST-Link 和通用调试适配器使用 probe-rs。两种后端均支持指定芯片、SWD 速度、可选复位和 RTT 通道 0 双向收发。

## 后端对应关系

| 探针类型 | 后端与目标数据库 |
|---|---|
| SEGGER J-Link | 动态加载 J-Link SDK；按序列号连接；目标名称来自 SEGGER 数据库 |
| CMSIS-DAP / ST-Link | probe-rs；按 VID/PID/接口/序列号连接；目标来自内置或自定义 Registry |
| 通用 FTDI/JTAG | probe-rs；默认隐藏，在“更多设置”中手动显示 |

Rust 版本不暴露 RTT 起始地址和范围配置。J-Link 交由 SEGGER SDK 自动发现控制块；其他探针由 probe-rs 根据内置或自定义 Target YAML 中的 RAM 映射查找。

## 兼容性说明

1. **J-Link 不走 probe-rs**：程序不会要求把 J-Link 切换为 WinUSB，也不会额外创建 pylink 的跨进程文件锁。实际多客户端能力取决于 J-Link 型号、固件和厂商工具版本。
2. **支持探针不等于支持所有目标**：CMSIS-DAP、ST-Link 等在协议速度、复位线、目标架构和固件能力上存在差异。枚举成功只代表 USB 探针已识别。
3. **两套目标名称相互独立**：J-Link 接受 SEGGER 数据库名称；其他探针使用 probe-rs registry。`probe_rs_targets/*.yaml` 不会扩展 SEGGER 数据库。
4. **RTT 控制块发现策略不同**：SEGGER SDK 使用厂商实现；probe-rs 根据目标 RAM 描述扫描 `SEGGER RTT` 控制块。
5. **复位时序因后端而异**：J-Link 使用 SEGGER SDK reset，其他探针使用 probe-rs target sequence。
6. **J-Link SDK 由用户安装**：安装包不分发 SEGGER DLL。未安装 J-Link Software Pack 时，J-Link 不会出现在列表中；可用 `JLINK_PATH` 指定动态库文件或安装目录。
7. **当前范围仍是通道 0**：与 Python 程序一致，只收发 Up/Down channel 0。
8. **并行调试需要硬件验收**：SEGGER 后端不主动锁住其他进程，并在烧录或复位令 RTT 暂时消失时持续重试；Keil Halt 不会被当作断线。

## 硬件验收建议

- 选择一块主力 J-Link 和一块 CMSIS-DAP/ST-Link 板卡。
- 覆盖冷启动、运行中连接、连接后复位、目标睡眠后恢复、连续大数据上行和下行缓冲区满。
- 验证 J-Link 使用 SEGGER 驱动时可与 Keil/Ozone 配合；验证非 J-Link 探针所需的 WinUSB 驱动。
- 对每个量产目标分别确认 SEGGER 和 probe-rs 芯片名称。
- 对比 50 ms 合帧、二进制数据、UTF-8/控制字符、ANSI 输出及 100 ms RTT 写超时行为。
