# SerialComm 集成 J-Link RTT 功能 - 修改计划

## 一、需求概述

在 SerialComm 项目中集成 J-Link RTT 功能，实现：
- 在串口选择下拉框中同时列出普通串口和 J-Link 设备（通过前缀区分）
- 在"更多设置"中新增 RTT 设备配置（芯片型号、速度、复位、地址范围）
- 只使用 RTT 通道 0 进行数据收发，将 RTT 当作串口使用

## 二、方案设计

### 2.1 串口下拉框合并方案

```
串口选择下拉框内容示例：
├── COM3 - USB-SERIAL CH340
├── COM4 - 通信端口
├── JLINK:SN=123456789 - J-Link (nRF52840_xxAA)
└── JLINK:SN=987654321 - J-Link (STM32F103C8)
```

- J-Link 设备以前缀 `JLINK:SN=xxx` 标识
- 选择 J-Link 项时，波特率下拉框自动禁用（RTT 无需波特率）
- 选择普通串口时，行为与现有一致

### 2.2 更多设置对话框扩展

新增"RTT 设备配置"区域：
- **芯片型号**：下拉框（nRF52840_xxAA、STM32F103C8 等）
- **J-Link 速度**：输入框（kHz，默认 4000）
- **连接时复位**：复选框
- **RTT 地址搜索范围**：起始地址 + 范围大小（可选）

### 2.3 连接流程

```
用户选择 JLINK:SN=xxx → 点击"打开串口" → 调用 pylink 连接 J-Link
→ 启动 RTT → 开始读写 RTT 通道 0 数据
```

## 三、文件修改清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `requirements.txt` | 修改 | 添加 `pylink` 依赖 |
| `src/rtt_manager.py` | 新增 | RTT 管理器，封装 J-Link RTT 读写 |
| `src/serial_settings_dialog.py` | 修改 | 新增 RTT 配置区域 |
| `ui/serial_settings_dialog.ui` | 修改 | 添加 RTT 配置 UI 控件 |
| `ui/Ui_serial_settings_dialog.py` | 重新生成 | 编译 UI 文件 |
| `src/main_window.py` | 修改 | 集成 RTT 管理器，修改串口选择逻辑 |
| `src/config_manager.py` | 修改 | 添加 RTT 配置项 |

## 四、详细实现步骤

### 步骤 1：添加依赖

修改 `requirements.txt`，添加：
```
pylink>=1.0.0
```

### 步骤 2：创建 RTT 管理器 `src/rtt_manager.py`

核心功能：
- `RttManager` 类，与 `SerialManager` 接口对齐
- 使用 pylink 库连接 J-Link
- 启动 RTT，读写通道 0
- 提供 `connect/disconnect/send_data` 方法
- 信号：`data_received(bytes)`, `connection_changed(bool)`, `error_occurred(str)`

关键方法：
- `get_available_jlink_devices()` - 扫描已连接的 J-Link 设备
- `connect(serial_no, chip, speed, reset_flag, start_address, range_size)` - 连接 J-Link 并启动 RTT
- `disconnect()` - 停止 RTT 并断开连接
- `send_data(data)` - 向 RTT 通道 0 写入数据
- 内部读取线程持续从 RTT 通道 0 读取数据

### 步骤 3：修改串口设置对话框

在 `serial_settings_dialog.ui` 中新增：
- "RTT 设备配置" GroupBox
- 芯片型号 ComboBox
- J-Link 速度 SpinBox
- 连载时复位 CheckBox
- RTT 地址范围输入框

修改 `serial_settings_dialog.py`：
- 加载/保存 RTT 配置
- 信号传递 RTT 配置变更

### 步骤 4：修改主窗口 `main_window.py`

核心修改点：

1. **`_refresh_ports()` 方法扩展**
   - 调用 `rtt_manager.get_available_jlink_devices()` 获取 J-Link 列表
   - 将 J-Link 设备以 `JLINK:SN=xxx` 格式添加到串口下拉框

2. **`_toggle_serial()` 方法扩展**
   - 判断选择的是普通串口还是 J-Link
   - 如果是 J-Link，调用 `rtt_manager.connect()`
   - 如果是普通串口，调用 `serial_manager.connect()`

3. **`_send_data()` 方法扩展**
   - 根据当前连接类型，调用对应的 `send_data()`

4. **波特率下拉框联动**
   - 选择 J-Link 时禁用波特率下拉框

5. **新增 `_is_jlink_port()` 辅助方法**
   - 判断当前选中项是否为 J-Link 设备

### 步骤 5：扩展配置管理 `config_manager.py`

新增配置项：
```python
'rtt_chip': 'nRF52840_xxAA',
'rtt_speed': 4000,
'rtt_reset': True,
'rtt_start_address': '',
'rtt_range_size': '',
```

## 五、数据流示意

```
┌─────────────┐     ┌──────────────┐
│  主窗口 UI   │────▶│ SerialManager│──▶ COM3 (普通串口)
│             │     └──────────────┘
│ 串口下拉框   │
│ 发送/接收    │     ┌──────────────┐
│             │────▶│ RttManager   │──▶ J-Link RTT 通道 0
└─────────────┘     └──────────────┘
```

## 六、注意事项

1. **pylink 依赖**：需要安装 J-Link 驱动（SEGGER J-Link Software），pylink 依赖其动态库
2. **J-Link 设备扫描**：可能耗时，建议在后台线程执行，避免阻塞 UI
3. **RTT 通道**：只使用通道 0（Terminal 通道），与 rtt_t2 项目一致
4. **错误处理**：J-Link 连接失败、RTT 启动失败等需要友好提示
5. **线程安全**：RTT 读取线程与 UI 线程之间需要通过信号通信

---

**文档版本**：v1.0
**创建日期**：2026年5月20日
