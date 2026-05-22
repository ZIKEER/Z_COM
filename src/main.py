import sys
import os

# 添加当前目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PySide6.QtWidgets import QApplication
from PySide6.QtGui import QIcon
from src.windows.main_window import MainWindow
from src.version import ICON_PATH, APP_NAME


def get_resource_path(relative_path):
    """获取资源文件的绝对路径，支持打包后的路径"""
    if getattr(sys, 'frozen', False):
        base_path = sys._MEIPASS
    else:
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base_path, relative_path)


def get_instance_id():
    """通过文件锁确定实例号，按 exe 完整路径隔离，跨平台崩溃安全"""
    import hashlib
    import itertools

    exe_path = os.path.realpath(sys.argv[0])
    exe_dir = os.path.dirname(exe_path)
    exe_key = hashlib.sha256(exe_path.encode('utf-8')).hexdigest()[:16]
    lock_dir = os.path.join(exe_dir, 'locks')
    os.makedirs(lock_dir, exist_ok=True)

    is_windows = sys.platform == 'win32'
    if is_windows:
        import msvcrt

    for i in itertools.count(1):
        lock_path = os.path.join(lock_dir, f'{exe_key}_inst_{i}.lock')
        try:
            fd = os.open(lock_path, os.O_CREAT | os.O_RDWR)
            if is_windows:
                msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            # fd 保持打开，进程退出时内核自动释放锁
            return i
        except (IOError, OSError):
            os.close(fd)


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
