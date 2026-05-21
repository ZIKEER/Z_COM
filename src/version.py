"""版本信息配置"""

# 软件版本号
VERSION = "0.0.7"
VERSION_NAME = f"V{VERSION}"

# 软件信息
APP_NAME = "Z_COM"
APP_NAME_EN = "Z_COM"
APP_DESCRIPTION = "基于 PySide6 开发的串口调试工具"

# 编译时间（由打包脚本自动更新，存储在 build_info.py）
from src.build_info import BUILD_TIME

# 文件信息
FILE_NAME = f"Z_COM_{VERSION_NAME}.exe"
DIST_DIR = f"dist/Z_COM_{VERSION_NAME}"

# 图标路径
ICON_PATH = "resources/icons/serial_comm.ico"
ICON_PATH_SVG = "resources/icons/serial_comm.svg"
