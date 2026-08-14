# Z_COM

基于 Rust、SEGGER J-Link SDK、probe-rs、Tauri 2 和 Svelte 5 的跨平台串口、Socket 与调试探针 RTT 通信工具。

本文档同时作为 Rust 分支的功能清单与开发路线图。每次增加、删除或调整功能时，必须同步更新对应状态和验收说明。

## 状态说明

- **已实现**：当前 Rust 版本已经具备，可进入实际测试。
- **待完善**：已有基础实现，但行为、兼容性或交互还未达到目标。
- **计划支持**：需求已经确认，等待实现。
- **暂不支持**：已经确认当前阶段不做，避免无效扩展。

## 已实现功能

### 通信后端

- 串口收发，覆盖跨平台后端全部通用参数：任意正整数波特率、5–8 数据位、1/2 停止位、无/奇/偶校验、无/XON-XOFF/RTS-CTS 流控和帧超时；实际可用组合由设备驱动决定。
- 串口连接期间可直接更新波特率、数据位、停止位、校验、流控和帧超时；驱动拒绝时自动恢复原参数并将原因写入报文区和日志。
- 波特率使用常用值建议与手动输入组合控件，接受 `1..10000000` 的正整数并保存最近 12 个使用值；连接中修改会立即应用到底层串口。
- TCP/UDP Client 与 Server 四种 Socket 模式。
- Socket Server 自动枚举本机 IPv4 地址，并固定提供 `0.0.0.0` 与 `127.0.0.1` 建议；Server/Client 地址均可手动输入且不附加无意义的 `[IP]` 后缀。
- TCP Server 采用“新连接顶替旧连接”的单客户端策略，发送目标始终为当前连接；替换时显示并记录新旧客户端地址，且不会把两条连接的残留数据混为一帧。
- J-Link 使用运行时动态加载的 SEGGER 官方动态库收发 RTT，不经过 probe-rs；未安装 SDK 不影响串口、Socket 和其他 probe-rs 探针功能。
- J-Link SDK 依次从绿色版已保存路径、程序同级目录、`JLINK_PATH` 和系统常见安装目录查找；“更多设置”可选择安装目录或动态库文件，路径保存在 `settings.json`。
- CMSIS-DAP、ST-Link 等其他探针使用 probe-rs 收发 RTT。
- J-Link、probe-rs 双 RTT 后端统一使用上行/下行通道 0。
- 支持指定 MCU 名称连接，不依赖大范围内存扫描。
- 支持从 `config/probe_rs_targets/*.yaml` 动态加载 probe-rs 自定义 MCU。
- 默认隐藏通用 FTDI/JTAG 适配器，可在“更多设置”中选择显示。
- J-Link RTT 暂时失效时自动尝试恢复，连续失败后重新打开会话。
- 串口/探针扫描、自定义目标读取和网卡枚举在后台阻塞任务中执行，不占用 Tauri/WebView 主线程；界面显示扫描状态并合并重复刷新。

### 报文显示

- HEX、ASCII、HEX+ASCII 三种显示模式。
- 收发报文分别使用 `←`、`→` 标识。
- 时间、方向箭头、HEX/ASCII 标签和通信数据使用不同颜色显示。
- 控制字符使用 Unicode 控制图符显示，非 ASCII 字节显示为 `\xNN`。
- ASCII 与 HEX+ASCII 模式支持 ANSI SGR 粗体、下划线、前景/背景 16 色、256 色和 RGB 真彩色；畸形或不完整序列不会中断接收。
- 自动滚动、5000 行显示裁剪、复制、全选、清空和 ANSI 开关右键菜单。
- 发送成功后同步显示发送报文并更新发送计数。
- 连接、断开、错误、警告、客户端变化和 RTT 恢复等后端事件使用独立颜色写入报文区，并由同一事件源同步写入自动日志。
- 接收/发送区域以及接收/扩展发送区域均可拖动调整，比例自动保存到 `settings.json` 并在下次启动恢复。
- 清空报文区时同步将发送和接收字节计数重置为零，不影响已经写入的自动日志。

### 发送功能

- ASCII、HEX 两种普通发送模式。
- 支持添加 CRLF、自动发送和自动发送间隔配置。
- 扩展发送支持单条发送、按序号发送、多条发送、循环发送、延时、排序、删除和移动。
- 扩展发送支持 JSON 导入、导出和自动持久化。
- 扩展发送 ASCII 支持 `\r`、`\n`、`\t`、`\0`、`\\`、`\xNN` 转义。
- 普通、自动和扩展发送均阻止空数据进入后端；HEX 奇数半字节、非法字符以及扩展发送中的未知/不完整转义会在对应入口给出明确错误。

### 配置、日志与打包

- 配置自动保存到 `config/settings.json`。
- 扩展发送自动保存到 `config/extended_send.json`。
- 通信数据和后端事件自动写入 `logs/log_YYYY-MM-DD.txt`，按本机自然日期每天一个文件，跨过午夜后自动切换并在同一天持续追加。
- 每条通信日志包含毫秒级完整日期时间、`←`/`→` 方向、HEX 和 ASCII；控制字符使用 Unicode 控制图符，非 ASCII 字节使用 `\xNN`，事件使用独立单行记录。
- 不提供手动保存日志或当前显示内容入口，避免现场数据依赖人工操作。
- `config/`、`logs/`、`locks/` 和 `instance_N/` 均位于当前执行文件同级目录，不使用系统用户配置目录。
- 基于文件锁分配多实例编号；实例 2+ 使用独立的 `instance_N/config` 和 `instance_N/logs`，索引显示在状态栏、About 及非首实例窗口标题中。
- Windows EXE 已嵌入项目图标。
- 采用免安装绿色版，不生成 MSI、NSIS 等安装包；版本号保留在 `dist/Z_COM_V版本号/` 目录，目录内主程序固定命名为 `Z_COM.exe`（Linux 为 `Z_COM`）。
- About 入口显示软件版本、构建时间、通信后端、实例索引和数据目录，并预留远期检查更新入口。

## 待完善与计划支持

以下需求已经确认，实施前以本表为准。

| 优先级 | 功能 | 当前状态 | 目标与验收标准 |
|---|---|---|---|
| P0 | Windows / Linux 跨平台支持 | **待完善** | 至少正式支持 Windows 10/11 x86_64 与主流 glibc Linux x86_64，并分别使用对应系统原生构建，不用 Windows 专属 API。串口、TCP/UDP、自动日志、绿色配置、多实例、后台任务和 About 必须在两端通过实机验收；界面兼容 Windows WebView2 与 Linux WebKitGTK、X11/Wayland。 |

## 远期开展计划

以下能力短期内不实施，不占用当前基础功能的开发优先级；完成上述 P0/P1 基础功能后再评估。

- **运行时版本检查与升级**：从 GitHub Releases 和 Gitee Releases 检查新版本，显示版本说明并由用户确认后升级。必须处理网络不可用、代理、校验失败、下载中断和回滚，禁止静默强制更新；绿色版的配置、日志和多实例数据不能因升级丢失。具体方案见下文。

### 运行时版本检查与升级方案

#### 范围与原则

- 继续采用免安装绿色版，不引入 MSI、NSIS、AppImage 等安装器。
- 正式分发目录中只保留一个主程序：Windows 为 `Z_COM.exe`，Linux 为 `Z_COM`；不额外分发 updater 可执行文件。
- 更新必须由用户确认，不静默下载、不强制安装；检查失败不得影响串口、Socket、RTT 等正常功能。
- GitHub 和 Gitee 作为同一版本产物的两个镜像源。下载完整性使用 SHA-256 校验即可，不增加发布签名、公钥或证书体系。
- 更新只替换主程序文件，不覆盖或迁移 `config/`、`logs/`、`locks/`、`instance_N/` 和用户自定义探针描述。

#### Release 约定

GitHub 与 Gitee 的相同版本必须上传内容一致的以下资产：

```text
Z_COM-v0.1.3-windows-x86_64.exe
Z_COM-v0.1.3-linux-x86_64
release-manifest.json
```

`release-manifest.json` 至少包含版本、平台、文件名、大小和 SHA-256：

```json
{
  "version": "0.1.3",
  "assets": {
    "windows-x86_64": {
      "name": "Z_COM-v0.1.3-windows-x86_64.exe",
      "size": 12345678,
      "sha256": "..."
    },
    "linux-x86_64": {
      "name": "Z_COM-v0.1.3-linux-x86_64",
      "size": 12345678,
      "sha256": "..."
    }
  }
}
```

Release tag 统一使用 `v主版本.次版本.修订版本`，例如 `v0.1.3`。运行时版本以 `env!("CARGO_PKG_VERSION")` 为准，构建时继续要求 `Cargo.toml`、`tauri.conf.json` 和 `package.json` 三处版本一致。

#### 检查与下载流程

1. 前端在 About 中提供“检查更新”入口；自动检查如后续启用，应延迟到主界面完成初始化后执行。
2. Rust 后端并发请求 GitHub 与 Gitee 的 latest release API，并分别设置连接和总请求超时；网络层至少兼容标准 `HTTP_PROXY`、`HTTPS_PROXY` 和 `NO_PROXY` 环境变量。
3. 两种 API 响应统一转换为内部 `ReleaseInfo`，过滤草稿、预发布、非法版本和缺少当前平台资产的发布。
4. 使用语义化版本比较远端版本与 `CARGO_PKG_VERSION`。两个源都可用时选择版本号较高者；只有一个源可用时继续使用该源；两个源都不可用时只提示检查失败。
5. 用户确认后下载当前平台和架构对应的资产，后端向前端发送进度、完成、取消和错误事件。
6. 下载失败时，只能切换到具有相同版本、相同资产名和相同 SHA-256 的备用源；备用源不满足条件时终止更新。
7. 下载写入程序目录内的 `.update/` 暂存目录，完成后计算 SHA-256。校验失败立即删除临时文件，不进入替换阶段。

建议的内部模块边界：

```text
src-tauri/src/update.rs            Release 查询、响应归一化、版本与资产选择
src-tauri/src/update_download.rs   流式下载、进度通知、取消和 SHA-256 校验
src-tauri/src/update_apply.rs      更新模式、文件替换、回滚和重启
```

#### 单文件 updater 方案

“独立 updater”表示独立运行的更新进程，不是额外分发的程序。`Z_COM` 本身同时包含正常 GUI 模式和更新模式，入口必须在初始化 Tauri 之前解析更新参数：

```text
正常启动：Z_COM.exe
更新模式：临时目录/Z_COM-updater.exe --apply-update <参数>
```

应用更新时执行以下步骤：

1. 校验新版文件后，把当前正在运行的 `Z_COM` 自身复制到操作系统临时目录。
2. 启动该临时副本并传入更新模式参数，包括当前程序路径、新文件路径和重启参数。
3. 主程序停止通信后台任务、刷新日志、释放文件锁并退出。
4. 临时副本等待主程序和其他实例退出，将当前程序改名为 `.bak`，再把 `.update/` 中的新程序移动到原路径。
5. Windows 保持 `.exe` 后缀；Linux 使用无后缀文件名，并在启动前确保新版文件具有可执行权限。路径处理统一使用 `Path`/`PathBuf` 和标准文件 API。
6. 临时副本启动新版程序。替换或启动命令失败时恢复 `.bak`；启动命令成功后由新版程序清理 `.bak` 和 `.update/`。
7. Linux updater 退出后可以删除临时副本；Windows 不能删除正在运行的自身，由下次正常启动时清理遗留的 updater 临时目录。

多实例场景下，更新前必须要求其他实例退出。后续实现可让所有正常实例持有共享更新锁，由 updater 获取独占锁；只有独占锁成功后才允许替换程序，确保 Windows 与 Linux 行为一致。

#### 建议依赖与验收

- HTTP 与流式下载：`reqwest`。
- 版本比较：`semver`。
- 完整性校验：`sha2`。
- 下载取消和进度：异步流配合 Tauri event；不得在 WebView/UI 线程执行阻塞网络或文件操作。
- Windows、Linux 分别验证：单源不可用、双源不可用、下载中断、SHA-256 不一致、目标目录不可写、存在其他实例、替换失败、启动失败回滚以及成功升级后配置和日志保留。

## 跨平台实现与验收

### 实现原则

- 文件、目录、锁、网络和时间处理继续使用 Rust 跨平台库；所有路径通过 `PathBuf` 组合，不拼接平台分隔符，不使用注册表、Named Mutex 等 Windows 专属方案。
- SEGGER SDK 保持可选动态依赖：Windows 加载 `JLink_x64.dll`，Linux 加载 `libjlinkarm.so`；缺失时只禁用 J-Link，不影响串口、Socket 和 probe-rs。
- Linux 串口和 USB 探针权限不足时，在报文区给出设备路径和权限提示，不允许静默显示为空；文档提供 `dialout` 组和厂商 udev 规则说明，但程序不自动修改系统权限。
- Linux 绿色版使用普通可写目录或压缩包分发，不采用运行时挂载为只读目录的形式；`config/`、`logs/`、`locks/` 和 `instance_N/` 仍位于程序同级目录。程序发现目录不可写时必须在启动阶段明确报错。
- Windows 与 Linux 分别原生构建并输出独立产物，不把未经目标系统运行验证的交叉编译结果作为正式版本。

### 平台产物

| 平台 | 目标产物 | 外部运行条件 |
|---|---|---|
| Windows 10/11 x86_64 | `dist/Z_COM_V版本号/Z_COM.exe` | 系统 WebView2；使用 J-Link 时安装 SEGGER Software Pack 或选择 SDK 目录。 |
| Linux x86_64 | `dist/linux-x86_64/Z_COM_V版本号/Z_COM`，可额外生成同目录压缩包 | 系统 WebKitGTK/GTK 运行库；当前用户具备串口和 USB 探针访问权限；使用 J-Link 时安装对应 Linux SDK。 |

### 最低验收矩阵

- 两个平台都验证启动、退出、窗口缩放、区域拖动、设置持久化和多实例索引/目录隔离。
- 两个平台都验证串口枚举、运行中修改参数、ASCII/HEX 收发、自动发送、断开恢复和按日期日志。
- 两个平台都验证 TCP/UDP Client/Server、本机地址枚举和 TCP 新连接替换提示。
- 有 SDK 与无 SDK 两种环境都验证启动；分别验证 J-Link SEGGER 后端和至少一种 probe-rs 探针。
- Linux 分别验证正常权限和权限不足场景；Windows 验证 WebView2、SDK 路径选择及绿色目录迁移。

## 暂不支持

- **手动保存日志/显示内容**：不作为目标功能，必须依赖自动日志避免人员忘记保存。
- **扩展发送高级编辑和批量操作**：Rust 当前功能已满足现阶段需求，暂不增加多行高级编辑、批量删除或批量移动。
- **RTT 大范围内存扫描**：当前只要求指定 MCU 后连接 RTT；不增加通用范围扫描。
- **安装包**：当前仅提供绿色版目录，不生成 MSI、NSIS 等安装程序。

## 功能维护规则

1. 新需求先加入“待完善与计划支持”，写明状态和可验证的验收标准，再开始修改代码。
2. 功能完成并通过检查后，将状态移动到“已实现功能”，不得只修改代码不更新本文档。
3. 跨平台能力优先；平台或底层驱动不支持的参数必须明确提示，不能静默降级。
4. 报文区中的现场事件与自动日志必须来自同一数据源，避免界面显示和日志记录不一致。
5. 与 Python 分支行为不一致时，在本文档中明确说明是待补齐、Rust 改进方案还是有意不支持。

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

## 便携版打包

```powershell
npm run pack
```

打包脚本输出结构为：

```text
dist/
└── Z_COM_V0.1.2/
    ├── Z_COM.exe
    ├── config/
    ├── logs/
    └── locks/
```

双 RTT 后端及兼容性说明见 [docs/probe-rs-rtt-assessment.md](docs/probe-rs-rtt-assessment.md)。
自定义 MCU 描述方法见 [docs/custom-probe-rs-targets.md](docs/custom-probe-rs-targets.md)。
