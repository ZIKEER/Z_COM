# egui 原生重构验收

自 0.3.0 起使用 egui/eframe + OpenGL 原生窗口，已移除 Tauri、wry、WebView 和原 Svelte 前端。仓库根目录即 Cargo 工程，界面代码位于 `src/ui/`。

## 已迁移

- 串口、TCP/UDP Client/Server、SEGGER 和 probe-rs RTT 复用原通信后端。
- HEX、ASCII、HEX+ASCII、控制字符、ANSI 16/256/RGB 颜色、下划线。
- 普通发送、CRLF、定时发送、扩展发送排序/延时/循环、JSON 导入导出。
- 独立曲线窗口、多模板解析、暂停、点数限制、缩放/拖动、Y 轴范围。
- 配置、多实例锁、自动日志、窗口与分隔区域尺寸记忆、后台设备扫描。
- GitHub/Gitee 更新检查、下载取消、大小与 SHA-256 校验、确认安装。
- 虚拟接收列表，跨行 Unicode 选区、复制、全选、右键菜单。

## 性能设计

字体沿用原版 CSS 字体栈且不随程序分发：界面使用 Segoe UI + 微软雅黑 UI，接收区使用 Cascadia Mono / Consolas，Segoe UI Symbol 提供 ␍␊ 等控制图符；其他平台回退到 PingFang SC、Noto Sans CJK SC、DejaVu 等系统字体。

空闲时等待事件；有数据时按约 30 FPS 刷新，每次处理事件最多占用 8 ms。显示积压最多 2 MiB 或 1024 个事件，超限仅裁剪待显示数据，不阻塞通信；原始通信仍写入日志，字节计数不丢失。接收历史保留 5000 行，只绘制可见行。

## 本次验证

- Windows Cargo check、debug/release 构建成功。
- 单元测试覆盖 HEX/转义、ANSI、曲线跨包解析、队列上限、Unicode 选区及 TCP/UDP 客户端回环收发。
- Ubuntu 24.04 WSL 原生 Cargo check 成功；不等同于真实 Linux 桌面、USB 和串口验收。
- Windows 原版/原生版截图已对比，调整接收背景、顶部双行布局、连接按钮、图标和分隔面板。
- 以原版布局重做分段按钮、图标按钮、设置分组、关于卡片、许可标签页和波特率弹窗；曲线规则使用居中遮罩弹窗。截图保存在本地 `artifacts/ui/`，字体和绘图库的细节仍与 WebView 原版存在差异。

调试构建支持 `--preview=about/settings/legal/baud/extended/serial/socket/probe/plot/rules`（每次选择一个值），用于直接打开界面截图；预览不会保存配置，发布版不启用此入口。独立曲线窗口截图需给 `scripts/capture-window.ps1` 传入 `-Title 数据图表`。

## 仍需实机验收

低配电脑持续高速串口、旧显卡/远程桌面、真实 J-Link/probe-rs RTT、Linux/macOS 桌面以及正式 Release 自更新。当前平台自动更新仅支持原有 Windows/Linux x86_64；macOS 不宣称可自动更新。ANSI 粗体目前使用加宽字距近似；时间戳与标签的粗体以偏移重绘模拟，均不是独立粗体字形。

运行：`cargo run --locked`。
打包：`cargo xtask pack-win` 输出到 `dist/Z_COM_V版本号/Z_COM.exe`，详见[开发、检查与打包](development.md)。
