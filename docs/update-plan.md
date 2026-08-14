# 运行时版本检查与升级

Z_COM `0.1.6` 已实现手动检查、双源结果展示、下载进度、取消、SHA-256 校验、单文件替换和失败回滚。自动定时检查暂未启用。

## 固定更新源

程序硬编码以下公开 Release API，不从配置或用户输入读取下载源：

```text
GitHub: https://api.github.com/repos/ZIKEER/Z_COM/releases/latest
Gitee:  https://gitee.com/api/v5/repos/zzk11111111/Z_COM/releases/latest
```

网络层使用 HTTPS、连接/总请求超时和系统代理，兼容常见的 `HTTP_PROXY`、`HTTPS_PROXY`、`NO_PROXY` 及操作系统代理配置。检查失败不影响串口、Socket 和 RTT。

## Release 约定

GitHub 与 Gitee 的相同版本必须提供内容一致的三个文件：

```text
Z_COM-windows-x86_64.exe
Z_COM-linux-x86_64
release-manifest.json
```

`release-manifest.json` 至少包含版本、平台、文件名、大小和 SHA-256：

```json
{
  "version": "0.1.6",
  "assets": {
    "windows-x86_64": {
      "name": "Z_COM-windows-x86_64.exe",
      "size": 12345678,
      "sha256": "..."
    },
    "linux-x86_64": {
      "name": "Z_COM-linux-x86_64",
      "size": 12345678,
      "sha256": "..."
    }
  }
}
```

Release tag 使用 `v主版本.次版本.修订版本`。运行时版本以 `env!("CARGO_PKG_VERSION")` 为准，发布前必须确保 `package.json`、`package-lock.json`、`Cargo.toml` 和 `tauri.conf.json` 一致。

## 检查与选择

1. About 页面点击“检查更新”后，Rust 后端并行查询 GitHub 与 Gitee。
2. 忽略草稿、预发布、非法版本和缺少当前平台文件或清单的 Release。
3. 分别下载并解析两个来源的 `release-manifest.json`。
4. 分别显示两个来源获取到的版本，以及高于、等于或低于当前版本；失败时显示对应来源的错误。
5. 使用语义化版本比较，选择高于当前版本的最高正式版本。
6. 只有两个来源都成功且版本均不高于当前版本时，才确认“当前已是最新版本”。
7. 同版本双源文件名、大小或 SHA-256 不一致时拒绝更新。
8. 同版本完全一致时优先使用 Gitee，GitHub 作为下载失败后的备用镜像。
9. 只有一个来源可用时继续使用该来源；另一个来源的错误同步显示并写入日志。

前端只接收版本、说明、来源、大小等展示信息。下载 URL、SHA-256 和可信候选保存在 Rust 状态中，前端不能传入或修改更新地址。

## 下载与校验

- 更新文件写入主程序同级 `.update/`，因此绿色版目录必须可写。
- 使用 64 KiB 数据块流式写入，并通过 Tauri event 显示百分比。
- 用户可以取消下载；临时 `.part` 文件会删除。
- 下载过程中限制数据不能超过清单大小。
- 完成后校验准确大小和 SHA-256，再原子移动为暂存文件。
- 主来源失败时，仅在备用来源具有相同版本、文件名、大小和 SHA-256 时回退。
- 安装前再次读取文件并校验，防止下载后被修改。

## 单文件替换与回滚

程序不额外分发 updater。当前 `Z_COM` 自身包含 GUI 模式和更新模式：

```text
正常启动：Z_COM.exe
更新模式：临时目录/Z_COM-updater.exe --apply-update <参数>
```

安装过程：

1. 用户点击“安装并重启”并二次确认。
2. 后端拒绝开发模式，并检查是否存在其他 Z_COM 实例。
3. 程序断开通信、写入日志，将当前主程序复制到系统临时目录。
4. 临时副本以更新模式启动，主程序退出并释放文件锁。
5. updater 再次校验暂存文件，把当前程序改名为 `.bak`，再移动新程序到原路径。
6. Linux 新程序设置为 `0755`；Windows 保持原目标路径和 `.exe` 后缀。
7. 新程序启动失败或替换失败时恢复 `.bak`。
8. 新程序成功启动后清理 `.bak`、`.update/` 和临时 updater。

更新进程的错误写入 `.update/update-error.log`。下次启动时 About 和主界面显示错误，旧版本仍可继续使用。

## 模块边界

```text
src-tauri/src/update.rs            Release 查询、清单解析、版本与镜像选择
src-tauri/src/update_download.rs   下载、进度、取消、大小与 SHA-256 校验
src-tauri/src/update_apply.rs      更新模式、备份、替换、回滚、重启和清理
```

## 已验证

- GitHub、Gitee 当前公开 `latest release` API 和 `release-manifest.json` 在线解析。
- 两个来源的版本、文件名、大小和 SHA-256 一致性判断。
- 21 项常规 Rust 测试和 1 项默认忽略的双源在线测试。
- Windows 隔离目录中的 updater 备份、替换、新程序启动和清理流程。
- Svelte 静态检查和生产构建。

## 待验收

- Windows 已完成 `0.1.4` 到 `0.1.5` 的真实跨版本下载、替换和重启；Linux 仍需完成相同验收。
- Windows/Linux 分别验证单源不可用、双源不可用、取消、断网、只读目录、其他实例和文件被占用。
- Linux 验证执行权限、Wayland/X11 重启和真实绿色版目录。
- Windows 验证 Defender/杀毒软件、不同目录权限和快捷方式启动场景。
