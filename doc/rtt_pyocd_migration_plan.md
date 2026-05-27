# RTT 通信层迁移设计：`pylink-square` -> `pyocd`

## 1. 文档目的

本文档基于当前仓库实现，重新整理 RTT 通信层从 `pylink-square` 迁移到 `pyocd` 的设计方案。

目标不是直接给出“理想化改造草案”，而是先对齐当前项目事实，再给出可落地的迁移边界、改动点、风险和验证方式，避免后续实现阶段继续依赖已经过期的假设。

---

## 2. 当前项目现状（与仓库对齐）

截至 2026-05-27，当前 RTT 相关实现状态如下：

### 2.1 运行时架构

- `src/io/rtt_manager.py` 当前基于 `pylink-square`，只支持 J-Link RTT。
- `src/io/rtt_reader.py` 读取线程依赖 `jlink.rtt_read()`，并沿用与串口一致的帧拼包策略。
- `src/windows/main_window.py` 在端口下拉框中将 RTT 设备标记为 `JLINK:SN=<sn>`。
- `src/windows/serial_settings_dialog.py` 已包含 RTT 设置区域，当前控件和配置流已经存在，不是从零开始新增。
- `src/core/config_manager.py` 已持久化 RTT 配置项：`rtt_chip`、`rtt_speed`、`rtt_reset`、`rtt_start_address`、`rtt_range_size`、`rtt_chip_history`、`rtt_frame_timeout`。

### 2.2 依赖与打包现状

- `requirements.txt` 当前依赖包含：
  - `pylink-square>=1.0.0`
  - `hidapi>=0.9.0`
- `pack.py` 当前显式包含 `--hidden-import pylink`。
- `pack_nuitka.py` 当前显式包含 `--include-module=pylink`。
- 当前打包裁剪逻辑只覆盖 Qt 相关 DLL / pyd / 插件，没有针对 `pyocd` 的额外裁剪。

### 2.3 测试现状

当前仓库已经有 RTT 相关测试，不应再按“无测试”假设设计迁移文档：

- `tests/test_rtt_manager.py`
- `tests/test_reader_threads.py`
- `tests/test_main_window.py`
- 以及多组集成/端到端测试

迁移设计必须把这些现有测试一起纳入改造范围。

### 2.4 当前实现中的关键行为

- RTT 扫描通过 `RttManager.get_available_devices()` 完成，本质上扫描的是 J-Link。
- `MainWindow._refresh_ports()` 会先加载串口，再后台扫描 J-Link，最后追加 4 个固定 Socket 模式条目。
- 选择 `JLINK:` 条目时，波特率下拉框被禁用，但仍保留 RTT 速度配置入口在“更多设置”中。
- 当前 `RttManager.connect()` 在用户未填写芯片型号时，会回退到 `nRF52840_xxAA`，这是现有行为，不是通用设计。
- 当前 `rtt_reset` 默认值为 `False`，文档不能再写成默认 `True`。

---

## 3. 原设计文档中需要修正的点

相对旧版迁移文档，以下内容已不再准确，必须修正：

| 主题 | 旧文档问题 | 当前应修正为 |
|---|---|---|
| 测试现状 | 按“无测试”或仅少量 mock 设计 | 迁移时要同步更新现有 RTT/主窗体/线程测试 |
| RTT 设备标识 | 假设可直接改为全新前缀而不提兼容 | 当前持久化的端口值是 `JLINK:SN=...`，迁移需考虑旧配置兼容 |
| RTT 设置 UI | 按“新增 RTT 区域”描述 | RTT UI 已存在，迁移主要是文案和语义调整 |
| 默认芯片行为 | 假设留空即通用行为 | 当前代码默认回退 `nRF52840_xxAA`，迁移后建议改为显式通用目标 |
| 打包依赖 | 只提 `pylink` / `pyocd` 替换 | 还要覆盖 `hidapi`、`pyusb`、可选排除模块和裁剪策略 |
| 配置迁移 | 默认直接改配置键 | 更合理做法是保留现有配置键，降低迁移成本 |
| 读写 API | 混用旧 `pyocd.rtt.RTT` 写法 | 应统一以目标版本可验证的 API 方案为准，并在实现前做探针验证 |

---

## 4. 迁移目标

将 RTT 后端从仅支持 J-Link 的 `pylink-square` 迁移到 `pyocd`，并在不破坏现有 UI 主流程的前提下，实现以下目标：

- 支持多种调试探针的 RTT 通信：
  - J-Link
  - DAPLink / CMSIS-DAP
  - ST-Link
- 保持主窗口“单下拉框统一入口”的交互方式不变。
- 保持 RTT 配置项和已有用户配置文件尽量兼容。
- 保持 `IOTransport` 层接口形状尽量稳定，减少对主窗口发送/接收流程的影响。
- 保持跨平台设计，不引入仅 Windows 可用的机制。

非目标：

- 本次迁移不扩展 SWO、Flash 烧录、断点调试等 pyOCD 高级功能。
- 本次迁移不重做主界面结构。
- 本次迁移不修改串口或 Socket 通道设计。

---

## 5. 迁移后的总体设计

### 5.1 保持的接口边界

以下边界建议保持不变：

- `RttManager` 继续实现 `IOTransport` 风格接口：
  - `connect()`
  - `disconnect()`
  - `send_data()`
  - `update_settings()`
  - `get_available_devices()`
- `RttReaderThread` 继续负责 RTT 持续读取和帧拼包。
- `MainWindow` 继续通过 `self._io` 统一走 `serial / rtt / socket` 三类通道。
- RTT 仍只使用通道 0。

### 5.2 需要调整的核心内部对象

当前：

- `self.jlink`

迁移后建议：

- `self.session`
- `self.rtt_cb` 或等价 RTT 句柄
- `self.probe_info`

建议 `RttManager` 内部状态改为：

```python
self.session = None
self.rtt_cb = None
self.probe_info = None
self.is_connected = False
self.reader_thread = None
```

说明：

- 不再暴露或依赖 `jlink.opened()` 这类 J-Link 专属状态判断。
- 连接状态统一由 `self.session is not None` 和 `self.is_connected` 管理。

---

## 6. 关键设计决策

### 6.1 探针统一标识

旧实现使用：

```text
JLINK:SN=<sn>
```

迁移后建议内部统一为：

```text
PROBE:<unique_id>
```

显示文案建议为：

```text
JLink  SN=<id>
DAPLink  SN=<id>
STLink  SN=<id>
```

这样做的原因：

- 避免内部 key 继续带有 J-Link 偏置。
- 与多探针目标一致。
- 后续如果增加更多 probe 类型，不需要再改 key 语义。

兼容要求：

- 配置加载时应兼容历史值 `JLINK:SN=...`。
- 刷新端口列表后，如发现旧值，允许自动映射为新的 `PROBE:<id>`。

### 6.2 芯片型号留空时的行为

当前实现留空时默认：

```text
nRF52840_xxAA
```

这不适合作为迁移后的默认策略。迁移后建议：

- UI 允许用户留空。
- 运行时留空时使用通用 Cortex-M 目标，例如 `cortex_m`。

理由：

- 更符合“探针无关”设计。
- 不把 Nordic 芯片型号硬编码成全局默认值。
- 对 RTT 只读写场景足够合理。

约束：

- 该方案是否能在目标 `pyocd` 版本下稳定支持 RTT，需要在实现前做一次独立探针验证。
- 若验证结果表明 `cortex_m` 不稳定，则退回“用户显式填写芯片型号”的策略，而不是继续默认 `nRF52840_xxAA`。

### 6.3 RTT 速度配置

建议保持 UI 中的 RTT 速度单位为 `kHz`，内部换算为 `Hz` 传给 `pyocd`。

原因：

- 兼容现有配置值。
- 用户认知成本最低。

### 6.4 配置键兼容

建议保留现有配置键，不做重命名：

- `rtt_chip`
- `rtt_speed`
- `rtt_reset`
- `rtt_start_address`
- `rtt_range_size`
- `rtt_chip_history`
- `rtt_frame_timeout`

原因：

- 当前 UI 和配置管理已经稳定接入这些键。
- 迁移的核心是后端实现变化，不值得为“语义纯洁”付出配置迁移成本。

### 6.5 RTT UI 调整策略

建议优先做“文案调整”，而不是修改控件对象名或大改 `.ui` 结构。

例如：

- `J-Link 速度` -> `SWD 速度`
- RTT 芯片输入框增加 placeholder / tooltip

原因：

- 现有控件已经完整接入运行时逻辑。
- 迁移不需要重新设计 RTT 面板结构。

---

## 7. 推荐的 pyOCD 集成方式

### 7.1 设备扫描

建议通过 `pyocd.core.helpers.ConnectHelper` 获取已连接 probe 列表。

设计目标：

- `get_available_devices()` 返回统一结构：

```python
[(unique_id, display_text), ...]
```

- `display_text` 与 UI 展示解耦，不把内部 key 混入显示字符串。

### 7.2 RTT 连接对象

文档层面建议不要过早把实现绑死到某一个未经验证的 pyOCD RTT API 路径上。

原因：

- 旧版草案混用了 `pyocd.rtt.RTT` 和 `pyocd.debug.rtt.RTTControlBlock` 两套写法。
- 当前仓库尚未引入 `pyocd`，文档应先定义“验证优先”的策略，而不是把某个 API 当成既定事实。

因此建议分两层描述：

1. 设计层约束
   - 必须支持手动指定 RTT 搜索地址和范围
   - 必须支持通道 0 读写
   - 必须返回 `bytes`
   - 必须允许主线程发送、读线程接收

2. 实现前验证项
   - 确认目标 `pyocd` 版本下稳定可用的 RTT API 入口
   - 确认 J-Link / CMSIS-DAP / ST-Link 三类 probe 至少各有一种可跑通的 RTT 样例

### 7.3 读写数据类型

迁移后设计上统一使用 `bytes`：

- 读取返回 `bytes`
- 写入接受 `bytes`

好处：

- 与 `MainWindow._send_data()` 当前流程自然衔接
- 比当前 `pylink` 的 `list[int]` 适配更干净

---

## 8. 文件级改动建议

以下为实现阶段建议修改的文件，但本次仅更新设计文档，不改代码。

| 文件 | 建议动作 | 说明 |
|---|---|---|
| `src/io/rtt_manager.py` | 重写核心实现 | 从 `pylink` 切到 `pyocd`，保留外部接口 |
| `src/io/rtt_reader.py` | 调整读取源对象 | 从 `jlink.rtt_read()` 切为 `pyocd` RTT 读接口 |
| `src/windows/main_window.py` | 适配 probe 标识与连接分支 | `JLINK:` 迁移为 `PROBE:`，保留历史值兼容 |
| `src/windows/serial_settings_dialog.py` | 微调 RTT 文案 | 主要是显示语义，不改整体结构 |
| `src/core/config_manager.py` | 尽量少改 | 保持原配置键，必要时只补兼容逻辑 |
| `requirements.txt` | 替换 / 增补依赖 | 引入 `pyocd`，评估是否保留 `hidapi` |
| `pack.py` | 调整 hidden imports / excludes | 覆盖 `pyocd` 相关模块 |
| `pack_nuitka.py` | 调整 include modules / excludes | 覆盖 `pyocd` 相关模块 |
| `tests/test_rtt_manager.py` | 更新 mock 目标 | 从 `pylink` 改到 `pyocd` |
| `tests/test_main_window.py` | 更新端口标识预期 | 从 `JLINK:` 改到 `PROBE:` |
| `tests/test_reader_threads.py` | 校准 RTT reader mock | 对齐新的读取 API |

---

## 9. 与当前代码逐项对比后的优化建议

### 9.1 `src/io/rtt_manager.py`

当前问题：

- 内部状态强绑定 `self.jlink`
- 扫描逻辑依赖解析 `connected_emulators()` 返回字符串
- `connect()` 默认芯片回退为 `nRF52840_xxAA`
- `send_data()` 需要把 `bytes` 转成 `list[int]`

迁移后建议：

- 扫描直接基于 probe 对象元数据，不再解析字符串
- 默认芯片改为通用目标或显式校验策略
- `send_data()` 直接写 `bytes`
- 增加 `probe_info` 供 UI 或日志使用

### 9.2 `src/io/rtt_reader.py`

当前优点：

- 帧拼包逻辑已经稳定
- 与串口 / Socket 模式的数据上抛方式一致

迁移建议：

- 保留现有拼包算法
- 只替换底层读取调用和连接状态判定
- `set_frame_timeout()` 逻辑保留

### 9.3 `src/windows/main_window.py`

当前问题不是流程设计，而是 RTT 设备类型被写死为 J-Link。

迁移建议：

- 保留“串口 + RTT + Socket 共用一个下拉框”的主流程
- 把扫描线程命名从 `JLinkScanThread` 改成中性名字，例如 `ProbeScanThread`
- `_on_port_changed()`、`_toggle_serial()` 从 `JLINK:` 逻辑切到 `PROBE:` 逻辑
- 加入旧配置兼容映射，避免用户升级后丢失之前保存的 RTT 端口项

### 9.4 `src/windows/serial_settings_dialog.py`

当前 RTT 设置区已经足够承载迁移，不建议做结构性重构。

建议只做：

- 文案去 J-Link 化
- 芯片留空说明补充
- 如果 pyOCD 目标验证结果要求用户必须填 chip，则在 UI 中把该限制明确写出来

### 9.5 测试层

旧文档对测试考虑不足，这是需要补强的地方。

建议迁移时至少覆盖：

- probe 扫描结果转换
- RTT 连接成功 / 失败路径
- `MainWindow` 中 `PROBE:` 端口分支
- RTT reader 的 bytes 读取和帧拼包
- 旧 `JLINK:` 配置值兼容

---

## 10. 依赖与打包策略

### 10.1 `requirements.txt`

建议目标形态：

```diff
- pylink-square>=1.0.0
+ pyocd>=<待验证版本>
```

同时需要明确：

- `hidapi` 当前已显式声明，迁移后是否继续显式保留，需要以实际 probe 后端依赖为准。
- 如果 `pyocd` 通过传递依赖带入 `pylink`，也不应在业务依赖层继续显式保留 `pylink-square`，除非验证后确认某些运行环境必须如此。

### 10.2 PyInstaller

当前只包含 `pylink`。迁移后需要：

- 添加 `pyocd` 相关 hidden imports
- 评估是否排除以下大模块：
  - `capstone`
  - `cmsis_pack_manager`
- 保持跨平台意识，不写死仅 Windows 才有意义的清理逻辑

### 10.3 Nuitka

当前只显式包含 `pylink`。

迁移后需要：

- 改为 `pyocd` 相关 include-module
- 评估 `pyocd` 自带 target 描述文件是否可裁剪
- 先以“可运行”优先，再做体积优化

设计原则：

1. 先验证功能可用
2. 再做可选裁剪
3. 最后再对体积做保守优化

---

## 11. 风险与应对

| 风险 | 说明 | 应对 |
|---|---|---|
| RTT API 路径不稳定 | 不同 `pyocd` 版本 RTT API 可能有差异 | 先锁定候选版本并做最小实验 |
| 通用 `cortex_m` 目标不一定总能工作 | 某些芯片上 RTT 搜索/接入可能失败 | 提供“留空走通用目标，失败时要求显式 chip”策略 |
| probe 类型兼容不一致 | J-Link、DAPLink、ST-Link 的 RTT 可用性可能不同 | 至少按 3 类 probe 各做一次样机验证 |
| 打包体积明显增加 | `pyocd` 及其依赖可能显著增大产物 | 将功能验证与体积裁剪拆成两阶段 |
| 老配置失效 | 历史 `JLINK:SN=...` 无法匹配新 key | 实现兼容映射逻辑并加入测试 |
| 线程模型问题 | 新 RTT API 在线程中读取可能有副作用 | 保持“主线程发送，读线程只读”模式，避免额外并发复杂度 |

---

## 12. 推荐实施顺序

### 阶段 1：独立技术验证

先在隔离环境完成：

1. 安装目标 `pyocd` 版本
2. 扫描 probe
3. 连接 probe
4. 启动 RTT
5. 通道 0 读写
6. 验证可选地址范围

产出：

- 确认最终使用的 `pyocd` 版本范围
- 确认最终使用的 RTT API 路径

### 阶段 2：运行时迁移

1. 重写 `src/io/rtt_manager.py`
2. 调整 `src/io/rtt_reader.py`
3. 更新 `src/windows/main_window.py`
4. 微调 `src/windows/serial_settings_dialog.py`
5. 处理旧配置兼容

### 阶段 3：依赖与打包迁移

1. 修改 `requirements.txt`
2. 修改 `pack.py`
3. 修改 `pack_nuitka.py`
4. 做一次完整打包验证

### 阶段 4：测试补齐

1. 更新 RTT 单测
2. 更新主窗口相关测试
3. 跑通现有核心测试集
4. 追加配置兼容测试

---

## 13. 验证清单

实现阶段建议至少验证以下场景：

### 13.1 功能验证

- 串口模式正常
- RTT 模式正常
- Socket 模式正常
- RTT 下收发计数正常
- RTT 下日志记录正常
- RTT 下 ASCII / HEX / MIXED 显示正常

### 13.2 兼容验证

- 旧配置中的 `JLINK:SN=...` 能被识别
- 旧 RTT 配置键不需要迁移脚本
- 多实例模式下配置和日志隔离不受影响

### 13.3 打包验证

- `python pack.py`
- `python pack_nuitka.py`
- 打包产物在目标平台可启动
- 目标平台上可完成至少一种 probe 的 RTT 连接

---

## 14. 回滚策略

如果迁移后 RTT 稳定性或打包兼容性不达标，回滚应聚焦于以下文件：

- `src/io/rtt_manager.py`
- `src/io/rtt_reader.py`
- `src/windows/main_window.py`
- `src/windows/serial_settings_dialog.py`
- `requirements.txt`
- `pack.py`
- `pack_nuitka.py`
- RTT 相关测试文件

回滚原则：

- 优先回滚 RTT 后端与打包配置
- 不回滚与本次迁移无关的 UI 或其他协议逻辑

---

## 15. 结论

相较旧版文档，当前更合理的迁移设计不是“把 J-Link 方案直接替换成 pyOCD 版本”，而是：

- 先承认现有仓库已经有一套稳定的 RTT/UI/配置/测试骨架
- 再把 J-Link 偏置收敛为通用 probe 抽象
- 同时保留用户配置兼容和主流程稳定性

这样做的好处是：

- 实现改动集中
- UI 影响最小
- 测试和回滚边界清晰
- 更符合后续跨探针、跨平台演进方向

---

**文档版本**：v2.0  
**最后更新**：2026-05-27
