import os
import sys


def get_resource_path(relative_path):
    """获取资源文件的绝对路径，兼容源码运行与打包环境。"""
    if getattr(sys, 'frozen', False):
        base_path = getattr(sys, '_MEIPASS', os.path.dirname(os.path.abspath(sys.executable)))
    else:
        base_path = os.path.dirname(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        )
    return os.path.join(base_path, relative_path)
