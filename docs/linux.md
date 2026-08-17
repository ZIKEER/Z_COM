# Linux 构建与运行

Rust 版至少以 Ubuntu 24.04 x86_64 作为 Linux 构建基线。正式 Linux 产物必须在 Linux 原生环境构建；WSL 可用于编译检查，但不能替代 USB 探针、串口和桌面环境实机验收。

## 构建依赖

Ubuntu 24.04：

```bash
sudo apt update
sudo apt install -y build-essential curl file libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libudev-dev libusb-1.0-0-dev \
  libwebkit2gtk-4.1-dev libxdo-dev pkg-config
```

安装 Node.js 20+、Rust stable 后，在项目目录执行：

```bash
npm ci
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run pack
```

绿色版输出到 `dist/linux-x86_64/Z_COM_V版本号/Z_COM`。打包脚本会设置可执行权限；整个版本目录必须放在当前用户可写的位置，因为配置、日志、实例锁和自定义 MCU 描述都保存在程序同级目录。

## 运行依赖

程序界面依赖系统 WebKitGTK/GTK。若发行版拆分开发包和运行包，只需为最终用户安装对应运行库，不需要 Rust、Node.js 或编译工具链。

Wayland 和 X11 均由 GTK/WebKitGTK 提供。个别桌面环境如遇 WebView 显示问题，可临时使用以下命令判断是否为 Wayland 兼容问题：

```bash
GDK_BACKEND=x11 ./Z_COM
```

WSLg 中如果 Mesa/Zink 无法初始化，可仅在诊断时强制软件渲染：

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 LIBGL_ALWAYS_SOFTWARE=1 ./Z_COM
```

该参数是 WSLg 图形兼容性方案，不应默认用于真实 Linux 桌面。

## 串口权限

常见串口设备为 `/dev/ttyUSB*`、`/dev/ttyACM*`。Ubuntu 通常由 `dialout` 组管理：

```bash
sudo usermod -aG dialout "$USER"
```

修改组后必须注销并重新登录。Z_COM 不会自动提权或修改系统用户组；权限不足时会在报文区和自动日志中给出设备路径与 `dialout` 提示。

## USB 调试探针权限

CMSIS-DAP、ST-Link 等 probe-rs 探针需要匹配设备 VID/PID 的 udev 规则。优先采用探针厂商或 probe-rs 官方提供的规则，并使用受控设备组及 `MODE="0660"`，不要为所有 USB 设备设置全局 `0666` 权限。规则更新后重新加载 udev 并重新插拔设备。

J-Link 继续使用 SEGGER Linux Software Pack 和 `libjlinkarm.so`。可在“更多设置”中选择 SEGGER 安装目录或动态库文件；未安装 J-Link SDK 不影响串口、Socket 和其他 probe-rs 探针。

## 发布验收

Linux 正式发布前必须在非 WSL 桌面系统完成[跨平台最低验收矩阵](cross-platform.md)，至少覆盖一个真实串口、一个 probe-rs 探针，以及有/无 SEGGER SDK 两种启动环境。
