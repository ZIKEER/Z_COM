"""Z_COM 打包脚本 - 使用 PyInstaller"""
import os
import sys
import shutil
import subprocess
from datetime import datetime

# 导入版本信息
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from src.version import VERSION, APP_NAME, ICON_PATH


def get_version():
    """获取版本号"""
    return VERSION


def clean_build():
    """清理构建文件"""
    # 清理 build 目录
    if os.path.isdir("build"):
        shutil.rmtree("build")
        print("  已删除: build")
    
    # 只清理当前版本的 dist 目录，保留其他版本
    version_name = f"V{VERSION}"
    current_dist = f"dist/Z_COM_{version_name}"
    if os.path.isdir(current_dist):
        shutil.rmtree(current_dist)
        print(f"  已删除: {current_dist}")


def create_icon():
    """检查/生成图标"""
    if os.path.exists(ICON_PATH):
        print(f"[信息] 图标文件已存在: {ICON_PATH}")
        return True
    
    print("[信息] 图标文件不存在，尝试生成...")
    result = subprocess.run([sys.executable, "scripts/create_icon.py"], capture_output=True, text=True)
    if result.returncode == 0 and os.path.exists(ICON_PATH):
        print(f"[信息] 图标已生成: {ICON_PATH}")
        return True
    
    print("[警告] 图标生成失败，将不使用图标")
    return False


def update_build_time(build_time):
    """更新 build_info.py 中的编译时间"""
    build_info_file = os.path.join("src", "build_info.py")
    try:
        with open(build_info_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 替换 BUILD_TIME 的值
        import re
        pattern = r'BUILD_TIME\s*=\s*"[^"]*"'
        replacement = f'BUILD_TIME = "{build_time}"'
        new_content = re.sub(pattern, replacement, content)
        
        with open(build_info_file, 'w', encoding='utf-8') as f:
            f.write(new_content)
        
        print(f"[信息] 编译时间已写入: {build_time}")
    except Exception as e:
        print(f"[警告] 写入编译时间失败: {e}")


def build():
    """执行打包"""
    version = get_version()
    version_name = f"V{version}"
    app_name = f"Z_COM_{version_name}"
    dist_dir = f"dist/{app_name}"
    build_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    print()
    print("=" * 50)
    print("  Z_COM PyInstaller 打包脚本")
    print("=" * 50)
    print()
    print(f"[信息] 版本号: {version_name}")
    print(f"[信息] 输出目录: {dist_dir}")
    print(f"[信息] 编译时间: {build_time}")
    print()
    
    # 写入编译时间到 version.py
    print("[信息] 写入编译时间...")
    update_build_time(build_time)
    
    # 清理旧文件
    print("[信息] 清理旧的构建文件...")
    clean_build()
    
    # 检查/生成图标
    has_icon = create_icon()
    
    print()
    print("[信息] 开始使用 PyInstaller 打包...")
    print()
    
    # 构建 PyInstaller 参数
    pyinstaller_args = [
        sys.executable, "-m", "PyInstaller",
        "--noconfirm",
        "--clean",
        "--noconsole",  # 不显示命令行窗口
        "--name", app_name,
        "--distpath", "dist",
        "--workpath", "build",
        "--specpath", ".",
        # 添加数据文件
        "--add-data", "config;config",
        "--add-data", "resources;resources",
        "--add-data", "ui;ui",
        # 添加隐式导入
        "--hidden-import", "serial",
        "--hidden-import", "serial.tools",
        "--hidden-import", "serial.tools.list_ports",
        "--hidden-import", "PySide6.QtSvg",
        "--hidden-import", "PySide6.QtSvgWidgets",
        "--hidden-import", "pylink",
        "--hidden-import", "psutil",
        "--hidden-import", "src.windows.main_window",
        "--hidden-import", "src.windows.serial_settings_dialog",
        "--hidden-import", "src.core.config_manager",
        "--hidden-import", "src.core.extended_send_manager",
        "--hidden-import", "src.windows.extended_send_widget",
        "--hidden-import", "src.io.rtt_manager",
        "--hidden-import", "src.version",
        # 排除不需要的模块
        "--exclude-module", "tkinter",
        "--exclude-module", "matplotlib",
        "--exclude-module", "numpy",
        "--exclude-module", "pandas",
        # 入口文件
        "run.py"
    ]
    
    # 添加图标参数
    if has_icon and os.path.exists(ICON_PATH):
        pyinstaller_args.extend(["--icon", ICON_PATH])
    
    # 执行打包
    try:
        result = subprocess.run(pyinstaller_args, check=True)
        
        if result.returncode == 0:
            # 重命名输出目录
            if os.path.exists(f"dist/{app_name}") and dist_dir != f"dist/{app_name}":
                if os.path.exists(dist_dir):
                    shutil.rmtree(dist_dir)
                os.rename(f"dist/{app_name}", dist_dir)
            
            # 显示结果
            show_result(dist_dir, app_name)
        else:
            print()
            print("=" * 50)
            print("  打包失败！")
            print("=" * 50)
            
    except subprocess.CalledProcessError as e:
        print()
        print("=" * 50)
        print("  打包失败！")
        print("=" * 50)
        print(f"错误代码: {e.returncode}")
        print()
        print("可能的原因:")
        print("1. PyInstaller 未安装，请运行: pip install pyinstaller")
        print("2. 依赖库缺失，请运行: pip install -r requirements.txt")
    except FileNotFoundError:
        print("[错误] Python 或 PyInstaller 未找到")


def show_result(dist_dir, app_name):
    """显示打包结果"""
    exe_path = os.path.join(dist_dir, f"{app_name}.exe")
    
    print()
    print("=" * 50)
    print("  打包完成！")
    print("=" * 50)
    print()
    print(f"[信息] 输出目录: {dist_dir}")
    print(f"[信息] 可执行文件: {exe_path}")
    
    # 计算总大小
    total_size = 0
    if os.path.exists(dist_dir):
        for dirpath, dirnames, filenames in os.walk(dist_dir):
            for f in filenames:
                fp = os.path.join(dirpath, f)
                total_size += os.path.getsize(fp)
    
    total_size_mb = total_size / (1024 * 1024)
    print(f"[信息] 总大小: {total_size_mb:.2f} MB")
    
    if os.path.exists(exe_path):
        size = os.path.getsize(exe_path)
        size_mb = size / (1024 * 1024)
        print(f"[信息] EXE大小: {size_mb:.2f} MB")
    
    print()
    
    # 打开输出目录
    if os.path.exists(dist_dir):
        if sys.platform == "win32":
            os.startfile(dist_dir)
        elif sys.platform == "darwin":
            subprocess.run(["open", dist_dir])
        else:
            subprocess.run(["xdg-open", dist_dir])


if __name__ == "__main__":
    build()
