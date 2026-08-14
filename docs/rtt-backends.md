# SEGGER / probe-rs 双 RTT 后端

## 后端选择

Z_COM 根据探针类型自动选择 RTT 后端：

| 探针类型 | 后端与目标数据库 |
|---|---|
| SEGGER J-Link | 动态加载 J-Link SDK；按序列号连接；目标名称来自 SEGGER 数据库 |
| CMSIS-DAP / ST-Link | probe-rs；按 VID/PID、接口和序列号连接；目标来自内置或自定义 Registry |
| 通用 FTDI/JTAG | probe-rs；默认隐藏，在“更多设置”中手动显示 |

J-Link 保留 SEGGER 驱动以及与 Keil、Ozone 等工具配合的链路；其他探针使用 probe-rs。两种后端均支持指定 MCU、SWD 速度、可选复位和 RTT 通道 0 双向收发。

Z_COM 不提供 RTT 起始地址或扫描范围设置。J-Link 由 SEGGER SDK 自动发现控制块；probe-rs 根据内置或自定义 Target YAML 的 RAM 映射查找。

## 兼容性说明

1. **J-Link 不走 probe-rs**：不要求将 J-Link 切换为 WinUSB，也不创建 pylink 跨进程锁。多客户端能力取决于 J-Link 型号、固件和 SEGGER 工具版本。
2. **识别探针不等于支持目标**：CMSIS-DAP、ST-Link 等在协议速度、复位线、目标架构和固件能力上存在差异。
3. **目标名称相互独立**：J-Link 接受 SEGGER 数据库名称；其他探针使用 probe-rs Registry。自定义 YAML 不会扩展 SEGGER 数据库。
4. **控制块发现策略不同**：SEGGER SDK 使用厂商实现；probe-rs 在目标 RAM 描述范围内查找 `SEGGER RTT` 控制块。
5. **复位时序不同**：J-Link 使用 SEGGER SDK reset；其他探针使用 probe-rs target sequence。
6. **J-Link SDK 由用户安装**：绿色版不分发 SEGGER 动态库。未安装 SDK 不影响其他通信后端，可在“更多设置”选择安装目录或动态库，也支持 `JLINK_PATH`。
7. **当前只使用通道 0**：仅收发 Up/Down channel 0。
8. **并行调试需要实机验收**：SEGGER 后端不主动锁住其他进程；烧录或复位导致 RTT 暂时消失时会持续恢复，Keil Halt 不视为断线。

## 自定义目标

CMSIS-DAP、ST-Link 等 probe-rs 后端可以从 `config/probe_rs_targets/*.yaml` 加载额外目标，详见[自定义 probe-rs MCU](custom-probe-rs-targets.md)。

## 硬件验收

- 选择一块主力 J-Link 和一块 CMSIS-DAP 或 ST-Link 板卡。
- 覆盖冷启动、运行中连接、连接后复位、目标睡眠恢复、连续大数据上行和下行缓冲区满。
- 验证 J-Link 使用 SEGGER 驱动时与 Keil/Ozone 配合，验证非 J-Link 探针所需的 USB 驱动或 udev 权限。
- 对每个量产目标分别确认 SEGGER 和 probe-rs 芯片名称。
- 对比 50 ms 合帧、二进制数据、UTF-8/控制字符、ANSI 输出和 RTT 写超时行为。
