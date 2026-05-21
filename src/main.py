import sys
import os
import psutil

# 添加当前目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PySide6.QtWidgets import QApplication
from PySide6.QtGui import QIcon
from main_window import MainWindow
from version import ICON_PATH, APP_NAME


def get_resource_path(relative_path):
    """获取资源文件的绝对路径，支持打包后的路径"""
    if getattr(sys, 'frozen', False):
        base_path = sys._MEIPASS
    else:
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base_path, relative_path)


def get_instance_id():
    """通过检测同名进程数量确定实例号"""
    current_pid = os.getpid()
    current_process = psutil.Process(current_pid)
    current_name = current_process.name()
    
    # 获取所有同名进程，按创建时间排序
    instances = []
    for proc in psutil.process_iter(['pid', 'name', 'create_time']):
        try:
            if proc.info['name'] == current_name:
                instances.append(proc.info)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    
    # 按创建时间排序
    instances.sort(key=lambda x: x['create_time'])
    
    # 找到当前进程的序号
    for i, proc_info in enumerate(instances):
        if proc_info['pid'] == current_pid:
            return i + 1
    
    return 1


def set_windows_app_id(instance_id):
    """设置Windows应用程序ID"""
    if sys.platform == 'win32':
        try:
            import ctypes
            app_id = f'Z_COM.{APP_NAME}.Instance{instance_id}'
            ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(app_id)
        except Exception:
            pass


def main():
    # 自动检测实例号
    instance_id = get_instance_id()
    print(f"[启动] 实例号: {instance_id}")
    
    # 设置Windows应用程序ID
    set_windows_app_id(instance_id)
    
    app = QApplication(sys.argv)
    
    # 设置应用程序图标
    icon_path = get_resource_path(ICON_PATH)
    if os.path.exists(icon_path):
        app.setWindowIcon(QIcon(icon_path))
    
    # 创建主窗口
    window = MainWindow(instance_id=instance_id)
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
