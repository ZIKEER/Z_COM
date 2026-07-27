# Z_COM

基于 PySide6 开发的串口调试工具。

## 功能

- 串口连接与配置
- ASCII/HEX 数据收发
- 扩展发送（批量、循环）
- 数据日志记录
- 配置持久化

## 安装

```bash
pip install -r requirements.txt
```

## 运行

```bash
python run.py
```

## 打包

Windows 下使用 PyInstaller：

```powershell
python pack.py
```

在 Windows 中通过 WSL 生成 Linux x86_64 可执行文件：

```powershell
.\pack_linux.ps1
```

默认使用 `Ubuntu-20.04`，也可指定其他已安装发行版：

```powershell
.\pack_linux.ps1 -Distro Ubuntu-24.04
```

Linux 或 WSL 内也可直接运行：

```bash
sudo bash scripts/install_linux_build_deps.sh
bash scripts/pack_linux.sh
```

Linux 产物保存在 `dist/linux-<架构>/Z_COM_V<版本>/`。为获得更好的发行版兼容性，建议使用项目支持范围内较旧的 Linux 发行版构建。

## 许可证

MIT License
