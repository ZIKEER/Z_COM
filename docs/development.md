# 开发、检查与打包

## 技术栈

- Rust stable
- Tauri 2
- Svelte 5 / SvelteKit
- Node.js 20+
- SEGGER J-Link SDK（可选运行时依赖）
- probe-rs

## 目录结构

```text
src/                 Svelte 前端
src-tauri/           Rust 后端与 Tauri 配置
static/              静态资源
scripts/             跨平台打包脚本
docs/                项目文档
LICENSE              Apache-2.0 完整许可文本
NOTICE               项目版权与声明
THIRD_PARTY_NOTICES.md 第三方组件许可说明
dist/                绿色版输出，不纳入源码版本管理
```

## 本地开发

安装依赖并启动 Tauri 开发模式：

```powershell
npm install
npm run tauri dev
```

前端单独开发或预览：

```powershell
npm run dev
npm run preview
```

Linux 系统依赖与权限设置见 [Linux 构建与运行](linux.md)。

## 检查与测试

优先执行与修改范围最接近的检查，再执行完整构建：

```powershell
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo check --manifest-path src-tauri/Cargo.toml --locked
```

## 绿色版打包

```powershell
npm run pack
```

Windows 输出：

```text
dist/
└── Z_COM_V版本号/
    └── Z_COM.exe
```

Linux 输出：

```text
dist/
└── linux-x86_64/
    └── Z_COM_V版本号/
        └── Z_COM
```

打包目录中只有主程序。许可证、NOTICE、第三方声明和前端资源均已编译进主程序，
不需要随可执行文件分发外置文件。`config/`、`logs/`、`locks/`、`.update/`
和多实例目录是程序运行时按需创建的数据，不属于发布包。

版本号保留在目录名和软件内部，主程序文件名固定为 Windows 的 `Z_COM.exe`
或 Linux 的 `Z_COM`。

## 整理 GitHub Release 资产

Windows 和 Linux 绿色版都准备好后执行：

```powershell
npm run release:stage
```

脚本校验 `package.json`、`Cargo.toml` 和 `tauri.conf.json` 版本一致，并将两个平台的主程序复制到同一个目录：

```text
dist/release/v版本号/
├── Z_COM-windows-x86_64.exe
├── Z_COM-linux-x86_64
└── release-manifest.json
```

`release-manifest.json` 包含版本、平台、固定文件名、大小和 SHA-256，可直接与两个
执行文件一起上传到 GitHub/Gitee Release。它是在线升级所需的 Release 元数据，不是
最终用户运行主程序时必须放在 EXE 旁边的文件。版本由 Release tag、版本目录和软件
内部信息表达，执行文件名不再携带版本号。脚本只接受当前版本的 Rust 绿色版路径，
不会收集 `dist/` 中的旧版或其他语言产物。

## 进程与线程模型

- 正常运行时只有一个 `Z_COM` 主进程；一个进程可以同时包含多个操作系统线程。
- Tauri/WebView、异步运行时、系统组件和阻塞任务线程池会创建或复用线程。
- 通信管理器为当前连接维护一个工作线程；设备枚举、下载等阻塞操作在后台执行。
- GitHub 与 Gitee 更新检查会并行查询，相关线程在任务结束后可以退出。
- 空闲线程在 Windows 中显示为 `Wait` 属于正常状态，表示正在等待事件或新任务，
  不等于卡死。是否异常应结合持续 CPU 占用、界面响应和日志判断。

## 版本约定

发布前确保以下版本一致：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

许可证字段也必须保持一致：`package.json` 和 `src-tauri/Cargo.toml` 均使用
`Apache-2.0`。版权标识使用 `ZIKEER`；不要在发布材料中加入个人法定姓名。

## 文档维护

1. 新需求先加入 [功能状态与路线图](roadmap.md)，写明状态和验收标准。
2. 功能完成后更新 [使用与功能说明](user-guide.md)，不能只修改代码。
3. 跨平台能力优先；平台或驱动不支持的参数必须明确提示，不能静默降级。
4. 报文区事件与自动日志必须来自同一数据源，避免现场记录不一致。
5. README 只维护入口信息，详细设计写入 `docs/` 对应专题。
