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

## 版本号

版本只需要通过一个命令修改：

```powershell
npm run version:set -- 0.1.8
```

该命令接受带或不带 `v` 的语义化版本号，并同步修改：

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` 中的 `z-com` 根包
- `src-tauri/tauri.conf.json`

不要再逐个手动修改版本文件。修改后应查看 Git diff，确认只有项目自身版本发生变化。

## 三种构建方式

### 原始构建

```powershell
npm run build:raw
```

调用 Tauri `build --no-bundle`，不复制产物。Windows 主程序位于
`src-tauri/target/release/z-com.exe`，Linux 主程序位于
`src-tauri/target/release/z-com`。这主要用于底层构建检查，日常通常不直接取用。

### Windows 日常开发包

```powershell
npm run pack:win
```

该命令只允许在 Windows 执行，先完成 release 构建，再自动复制为：

```text
dist/
└── Z_COM_V版本号/
    └── Z_COM.exe
```

无需再从 `src-tauri/target/release/` 手动复制。原有 `npm run pack` 仍保留为
当前平台便携包命令；Linux 执行时输出：

```text
dist/
└── linux-x86_64/
    └── Z_COM_V版本号/
        └── Z_COM
```

如果正在从目标目录运行 `Z_COM.exe`，Windows 会锁定该文件，脚本无法覆盖。关闭该实例
后重新执行命令即可；脚本不会擅自结束正在使用的程序。

打包目录中只有主程序。许可证、NOTICE、第三方声明和前端资源均已编译进主程序，
不需要随可执行文件分发外置文件。`config/`、`logs/`、`locks/`、`.update/`
和多实例目录是程序运行时按需创建的数据，不属于发布包。

版本号保留在目录名和软件内部，主程序文件名固定为 Windows 的 `Z_COM.exe`
或 Linux 的 `Z_COM`。

### 正式发布包

Windows x86_64 环境执行：

```powershell
npm run pack:release
```

然后在 Linux x86_64 环境进入同一工作区，再执行：

```bash
npm run pack:release
```

每次命令都会原生构建当前平台，并直接写入同一个版本目录。Windows 与 Linux
产物都存在后，脚本自动生成清单：

```text
dist/release/v版本号/
├── Z_COM-windows-x86_64.exe
├── Z_COM-linux-x86_64
└── release-manifest.json
```

脚本会为每个平台记录源码指纹。两个平台没有全部完成，或并非由完全相同的源码构建时，
脚本不会保留 `release-manifest.json`，避免把旧文件或不同代码状态混入同一个发布包。
目标目录中的旧平台产物以及误运行产生的 `config/`、`logs/`、`locks/`、`.update/`
和 `instance_N/` 也会被清理，最终目录只保留可发布文件。
因为正式产物要求平台原生构建，Windows 不能单独生成经过验收的 Linux 程序。可以在
Windows 与 WSL 共同访问的项目路径中依次执行，两边会自然归集到同一个 `dist` 目录。

`release-manifest.json` 包含版本、平台、固定文件名、大小和 SHA-256，可将目录内三个
文件一起上传到 GitHub/Gitee Release。它是在线升级所需的 Release 元数据，不是最终
用户运行主程序时必须放在 EXE 旁边的文件。

## 进程与线程模型

- 正常运行时只有一个 `Z_COM` 主进程；一个进程可以同时包含多个操作系统线程。
- Tauri/WebView、异步运行时、系统组件和阻塞任务线程池会创建或复用线程。
- 通信管理器为当前连接维护一个工作线程；设备枚举、下载等阻塞操作在后台执行。
- GitHub 与 Gitee 更新检查会并行查询，相关线程在任务结束后可以退出。
- 空闲线程在 Windows 中显示为 `Wait` 属于正常状态，表示正在等待事件或新任务，
  不等于卡死。是否异常应结合持续 CPU 占用、界面响应和日志判断。

## 版本一致性

所有构建脚本都会在构建前检查以下版本一致：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

正常情况下使用 `npm run version:set -- 新版本号` 后无需人工同步。

许可证字段也必须保持一致：`package.json` 和 `src-tauri/Cargo.toml` 均使用
`Apache-2.0`。版权标识使用 `ZIKEER`；不要在发布材料中加入个人法定姓名。

## 文档维护

1. 新需求先加入 [功能状态与路线图](roadmap.md)，写明状态和验收标准。
2. 功能完成后更新 [使用与功能说明](user-guide.md)，不能只修改代码。
3. 跨平台能力优先；平台或驱动不支持的参数必须明确提示，不能静默降级。
4. 报文区事件与自动日志必须来自同一数据源，避免现场记录不一致。
5. README 只维护入口信息，详细设计写入 `docs/` 对应专题。
