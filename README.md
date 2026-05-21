# 串口助手 (SerialComm)

基于 PySide6 开发的串口调试工具，支持多种数据格式显示、自动发送、扩展发送等功能。

## 功能特性

### 串口配置
- 支持串口号和串口名称显示
- 支持常见波特率（9600-921600）
- 支持数据位（5-8位）、停止位（1/1.5/2位）、校验位配置
- 支持参数持久化，配置修改时自动保存

### 数据收发
- 支持 ASCII 和 HEX 两种发送格式
- 支持 HEX、ASCII、ASCII+HEX 三种显示模式
- 支持自动滚动和清空接收
- 支持数据帧自动拼接（50ms超时机制）

### 数据日志
- 启动后自动记录所有收发数据
- 同时存储 ASCII 和 HEX 格式
- 日志文件按日期自动创建，存储在 `logs` 目录下
- 文件格式：`YYYY-MM-DD.log`

### 自动发送
- 支持定时自动发送
- 可设置发送间隔（100ms-60s）

### 扩展发送
- 支持多条数据管理和批量发送
- 支持 HEX 和字符串两种数据格式
- 支持单条发送和多条顺序发送
- 支持循环发送模式
- 每条数据可设置独立延时
- 支持导入导出配置（JSON格式）
- 支持上下移动调整数据顺序

### 参数持久化
- 配置修改时自动保存
- 保存内容：串口端口、波特率、显示模式、发送模式、自动滚动、自动发送间隔
- 配置文件存储在 `config/settings.json`
- 扩展发送配置存储在 `config/extended_send.json`

## 安装依赖

```bash
pip install -r requirements.txt
```

## 运行程序

```bash
python run.py
```

## 项目结构

```
SerialComm/
├── config/                 # 配置目录
│   ├── settings.json      # 配置文件
│   └── extended_send.json # 扩展发送配置
├── doc/                    # 文档目录
│   └── design.md          # 设计方案文档
├── ui/                    # UI 文件目录
│   ├── main_window.ui     # 主窗口界面文件
│   ├── serial_settings_dialog.ui  # 串口设置对话框
│   ├── extended_send_widget.ui    # 扩展发送面板
│   ├── Ui_main_window.py
│   ├── Ui_serial_settings_dialog.py
│   └── Ui_extended_send_widget.py
├── src/                   # 源代码目录
│   ├── main.py           # 程序入口
│   ├── main_window.py    # 主窗口逻辑
│   ├── serial_settings_dialog.py  # 串口设置对话框
│   ├── extended_send_manager.py   # 扩展发送管理类
│   ├── extended_send_widget.py    # 扩展发送面板组件
│   ├── config_manager.py # 配置管理
│   └── __init__.py       # 包初始化
├── resources/            # 资源文件
│   └── icons/           # 图标文件
├── tests/               # 测试代码
├── logs/                # 日志目录
├── run.py               # 启动脚本
├── requirements.txt     # 依赖包
└── README.md           # 项目说明
```

## 界面布局

### 主界面
- **顶部工具栏**：显示模式选择、自动滚动、清空接收、扩展发送按钮
- **中央区域**：数据接收显示区域、扩展发送面板（可折叠）
- **底部左侧**：串口配置（串口选择、波特率、刷新、更多设置、打开/关闭）
- **底部右侧**：数据发送区域（发送格式、自动发送、发送输入）

### 扩展发送面板
- 点击"扩展发送"按钮显示/隐藏
- 显示在接收区域右侧
- 支持表格形式显示多条数据
- 支持批量发送和循环发送

## 使用说明

### 1. 连接串口
1. 从左下角串口下拉框选择串口（显示串口号和名称）
2. 选择波特率或点击"更多设置"配置详细参数
3. 点击"打开串口"按钮连接

### 2. 接收数据
- 顶部工具栏选择显示模式：HEX、ASCII 或 ASCII+HEX
- 接收数据会自动显示在中央接收区域
- 支持自动滚动和清空接收
- 数据帧自动拼接（50ms超时）

### 3. 发送数据
- 底部右侧选择发送格式：ASCII 或 HEX
- 在发送输入框输入数据
- 点击"发送"按钮或使用自动发送功能

### 4. 扩展发送
- 点击顶部"扩展发送"按钮显示扩展发送面板
- 点击"添加"按钮添加数据条目
- 支持 HEX 和字符串两种格式
- 点击"发送"按钮发送单条数据
- 勾选多条数据后点击"发送选中"批量发送
- 支持循环发送模式
- 每条数据可设置独立延时
- 支持导入导出配置（JSON格式）

### 5. 日志管理
- 所有收发数据自动记录到日志文件
- 日志文件按日期自动创建，存储在 `logs` 目录下
- 文件格式：`YYYY-MM-DD.log`
- 日志同时记录 ASCII 和 HEX 格式

### 6. 参数持久化
- 配置修改时自动保存
- 保存内容：串口端口、波特率、显示模式、发送模式、自动滚动、自动发送间隔
- 配置文件存储在 `config/settings.json`
- 扩展发送配置存储在 `config/extended_send.json`

## 开发说明

### UI 工作流程
本项目遵循 PySide6 UI 工作流程：
1. 使用 Qt Designer 编辑 `.ui` 文件
2. 使用 `pyside6-uic` 编译为 Python 模块
3. 在业务逻辑类中使用生成的 UI 类

### 编译 UI 文件
```bash
pyside6-uic ui/main_window.ui -o ui/Ui_main_window.py
pyside6-uic ui/serial_settings_dialog.ui -o ui/Ui_serial_settings_dialog.py
pyside6-uic ui/extended_send_widget.ui -o ui/Ui_extended_send_widget.py
```

## 许可证

MIT License
