import os
import sys


def get_app_base_path():
    """返回配置、日志等可写数据的根目录。"""
    if getattr(sys, 'frozen', False):
        return os.path.dirname(os.path.abspath(sys.executable))
    return os.path.dirname(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    )


def get_resource_path(relative_path):
    """获取资源文件的绝对路径，兼容源码运行与打包环境。"""
    if getattr(sys, 'frozen', False):
        base_path = getattr(sys, '_MEIPASS', os.path.dirname(os.path.abspath(sys.executable)))
    else:
        base_path = get_app_base_path()
    return os.path.join(base_path, relative_path)
