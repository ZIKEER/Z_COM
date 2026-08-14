# 运行时版本检查与升级方案

> 远期计划，短期内不实施。当前优先完成基础通信功能与跨平台实机验收。

## 范围与原则

- 保持免安装绿色版，不引入 MSI、NSIS、AppImage 等安装器。
- 正式目录只保留一个主程序：Windows 为 `Z_COM.exe`，Linux 为 `Z_COM`。
- 更新必须由用户确认，不静默下载、不强制安装。
- 检查失败不得影响串口、Socket 和 RTT 功能。
- GitHub 和 Gitee 作为同一版本产物的两个镜像源。
- 使用 SHA-256 校验下载完整性，不增加发布签名体系。
- 只替换主程序，不覆盖 `config/`、`logs/`、`locks/`、`instance_N/` 和自定义 MCU 描述。

## Release 约定

GitHub 与 Gitee 的相同版本必须上传内容一致的资产：

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

Release tag 统一使用 `v主版本.次版本.修订版本`。运行时版本以 `env!("CARGO_PKG_VERSION")` 为准，构建时要求 `Cargo.toml`、`tauri.conf.json` 和 `package.json` 三处一致。

## 检查与下载流程

1. About 页面提供“检查更新”入口；如启用自动检查，应在主界面初始化完成后执行。
2. Rust 后端并发请求 GitHub 与 Gitee latest release API，分别设置连接和总请求超时。
3. 网络层兼容 `HTTP_PROXY`、`HTTPS_PROXY` 和 `NO_PROXY` 环境变量。
4. 将响应统一转换为内部 `ReleaseInfo`，过滤草稿、预发布、非法版本和缺少当前平台资产的发布。
5. 使用语义化版本比较远端版本与当前版本。两个源都可用时选择较高版本；单源可用时继续使用；全部失败时只提示检查失败。
6. 用户确认后下载当前平台资产，后端向前端发送进度、完成、取消和错误事件。
7. 下载失败时，只切换到版本、资产名和 SHA-256 全部一致的备用源。
8. 下载写入程序目录 `.update/`，完成后校验 SHA-256；校验失败立即删除临时文件。

建议模块边界：

```text
src-tauri/src/update.rs            Release 查询、响应归一化、版本与资产选择
src-tauri/src/update_download.rs   流式下载、进度、取消和 SHA-256 校验
src-tauri/src/update_apply.rs      更新模式、文件替换、回滚和重启
```

## 单文件替换方案

“独立 updater”是独立运行的更新进程，不是额外分发的程序。`Z_COM` 本身同时包含 GUI 模式和更新模式，并在初始化 Tauri 前解析更新参数：

```text
正常启动：Z_COM.exe
更新模式：临时目录/Z_COM-updater.exe --apply-update <参数>
```

更新步骤：

1. 校验新版后，将当前主程序复制到操作系统临时目录。
2. 启动临时副本，传入当前程序路径、新文件路径和重启参数。
3. 主程序停止通信任务、刷新日志、释放文件锁并退出。
4. 临时副本等待所有实例退出，将当前程序改名为 `.bak`，再把新版移动到原路径。
5. Windows 保持 `.exe` 后缀；Linux 使用无后缀文件名并确保可执行权限。
6. 临时副本启动新版。替换或启动失败时恢复 `.bak`。
7. 新版启动后清理 `.bak` 和 `.update/`。Windows 遗留的临时 updater 在下次启动清理。

多实例场景下，更新前必须要求其他实例退出。后续可让正常实例持有共享更新锁，由 updater 获取独占锁后再替换程序。

## 建议依赖

- HTTP 与流式下载：`reqwest`
- 版本比较：`semver`
- 完整性校验：`sha2`
- 进度与取消：异步流配合 Tauri event

网络和文件操作不得阻塞 WebView/UI 线程。

## 验收范围

Windows、Linux 分别验证：

- 单源不可用和双源不可用。
- 下载中断与取消。
- SHA-256 不一致。
- 目标目录不可写。
- 存在其他运行实例。
- 替换失败和启动失败回滚。
- 成功升级后配置、日志和自定义目标保持不变。
