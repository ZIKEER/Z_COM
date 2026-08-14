# Z_COM

基于 Rust、SEGGER J-Link SDK、probe-rs、Tauri 2 和 Svelte 5 的跨平台串口、Socket 与调试探针 RTT 通信工具。

## 当前功能

- 串口收发及数据位、停止位、校验、流控、帧超时配置
- TCP/UDP 客户端与服务端
- J-Link 通过 SEGGER 官方 SDK 收发 RTT，CMSIS-DAP、ST-Link 等通过 probe-rs 收发 RTT
- 默认隐藏 probe-rs 识别到的通用 FTDI/JTAG 适配器，可在“更多设置”中显示
- 按 MCU 名称连接，并从应用配置目录的 `probe_rs_targets/*.yaml` 动态加载自定义目标
- HEX、ASCII、HEX+ASCII 显示，ANSI SGR 颜色与控制字符可视化
- 自动发送、CRLF、扩展发送排序/循环、JSON 导入导出
- 5000 行接收裁剪、收发计数、按实例隔离的配置与日志
- 基于文件锁的跨平台多实例编号

界面中的通用功能统一使用“调试探针 / RTT”。只有设备列表、驱动提示与兼容配置会显示 J-Link、CMSIS-DAP、ST-Link 等具体类型。

## 开发

```powershell
npm install
npm run tauri dev
```

静态检查与测试：

```powershell
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

双 RTT 后端及兼容性说明见 [docs/probe-rs-rtt-assessment.md](docs/probe-rs-rtt-assessment.md)。
自定义 MCU 描述方法见 [docs/custom-probe-rs-targets.md](docs/custom-probe-rs-targets.md)。
