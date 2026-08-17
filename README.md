# Z_COM

Z_COM 是基于 Rust、Tauri 2 和 Svelte 5 的跨平台通信工具，支持串口、TCP/UDP 与调试探针 RTT。

## 主要能力

- 串口收发与运行时参数调整
- TCP/UDP Client、Server 通信
- SEGGER J-Link 与 probe-rs 双 RTT 后端
- HEX、ASCII、HEX+ASCII 与 ANSI 颜色显示
- 普通发送、自动发送和扩展发送
- 按日期自动记录通信日志
- 单可执行文件发布，首次运行后在程序同级目录创建配置、日志和多实例数据
- Windows / Linux 绿色版运行，无需随程序分发许可证等外置文件
- GitHub / Gitee 双源检查、下载校验和单文件升级

## 快速开始

开发环境需要 Node.js 20+、Rust stable，以及对应平台的 Tauri 系统依赖。

```powershell
npm install
npm run tauri dev
```

检查与测试：

```powershell
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

打包绿色版：

```powershell
npm run pack
```

## 文档

- [文档索引](docs/README.md)
- [使用与功能说明](docs/user-guide.md)
- [开发、检查与打包](docs/development.md)
- [Windows / Linux 跨平台说明](docs/cross-platform.md)
- [SEGGER / probe-rs 双 RTT 后端](docs/rtt-backends.md)
- [自定义 probe-rs MCU](docs/custom-probe-rs-targets.md)
- [功能状态与开发路线图](docs/roadmap.md)

当前版本以 `package.json`、`src-tauri/Cargo.toml` 和
`src-tauri/tauri.conf.json` 中一致的版本号为准。正式功能状态、验收范围和
暂不支持项以[路线图](docs/roadmap.md)为准。

## 许可证

Z_COM 自有代码由 ZIKEER 以 [Apache License 2.0](LICENSE) 授权。
第三方组件继续适用各自的许可证，详情见 [NOTICE](NOTICE) 和
[第三方声明](THIRD_PARTY_NOTICES.md)。这些文档已内嵌到可执行文件，可在
“About → 开源许可”中离线查看；About 页面也提供
[GitHub](https://github.com/ZIKEER/Z_COM) 和
[Gitee](https://gitee.com/zzk11111111/Z_COM) 仓库入口。
