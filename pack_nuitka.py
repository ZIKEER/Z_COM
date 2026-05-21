"""Z_COM 打包脚本 - 使用 Nuitka"""
import os
import sys
import shutil
import subprocess
import glob
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
    
    # 清理 dist_nuitka 目录下的所有内容
    if os.path.isdir("dist_nuitka"):
        shutil.rmtree("dist_nuitka")
        print("  已删除: dist_nuitka")


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


def clean_nuitka_temps():
    """清理 Nuitka 生成的临时目录"""
    temp_patterns = [
        "dist_nuitka/run.dist",
        "dist_nuitka/run.onefile-build", 
        "dist_nuitka/run.build",
        "dist_nuitka/*.build"
    ]
    for pattern in temp_patterns:
        for path in glob.glob(pattern):
            if os.path.isdir(path):
                shutil.rmtree(path)
                print(f"  已删除临时目录: {path}")


def build():
    """执行打包"""
    version = get_version()
    version_name = f"V{version}"
    app_name = f"Z_COM_{version_name}"
    dist_dir = f"dist_nuitka/{app_name}"
    build_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    print()
    print("=" * 50)
    print("  Z_COM Nuitka 打包脚本")
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
    print("[信息] 开始使用 Nuitka 打包...")
    print()
    
    # 构建 Nuitka 参数
    nuitka_args = [
        sys.executable, "-m", "nuitka",
        "--standalone",  # 独立模式
        "--onefile",  # 单文件模式
        "--windows-console-mode=disable",  # 不显示命令行窗口
        "--enable-plugin=pyside6",  # 启用 PySide6 插件
        f"--output-filename={app_name}.exe",
        f"--output-dir=dist_nuitka",
        "--company-name=Z_COM",
        f"--product-name={APP_NAME}",
        f"--file-version={version}",
        f"--product-version={version}",
        # 包含数据文件
        "--include-data-dir=config=config",
        "--include-data-dir=resources=resources",
        "--include-data-dir=ui=ui",
        # 包含隐式导入
        "--include-module=serial",
        "--include-module=serial.tools",
        "--include-module=serial.tools.list_ports",
        "--include-module=PySide6.QtSvg",
        "--include-module=PySide6.QtSvgWidgets",
        "--include-module=pylink",
        "--include-module=psutil",
        "--include-module=src.main_window",
        "--include-module=src.serial_settings_dialog",
        "--include-module=src.config_manager",
        "--include-module=src.extended_send_manager",
        "--include-module=src.extended_send_widget",
        "--include-module=src.rtt_manager",
        "--include-module=src.version",
        # 排除不需要的模块
        "--noinclude-custom-mode=tkinter:error",
        "--noinclude-custom-mode=matplotlib:error",
        "--noinclude-custom-mode=numpy:error",
        "--noinclude-custom-mode=pandas:error",
        # 入口文件
        "run.py"
    ]
    
    # 添加图标参数
    if has_icon and os.path.exists(ICON_PATH):
        nuitka_args.extend([f"--windows-icon-from-ico={ICON_PATH}"])
    
    # 执行打包
    try:
        result = subprocess.run(nuitka_args, check=True)
        
        if result.returncode == 0:
            # 清理 Nuitka 临时目录
            print("[信息] 清理临时文件...")
            clean_nuitka_temps()
            
            # 将 exe 移动到版本目录
            exe_file = f"dist_nuitka/{app_name}.exe"
            if os.path.exists(exe_file):
                os.makedirs(dist_dir, exist_ok=True)
                final_exe = os.path.join(dist_dir, f"{app_name}.exe")
                shutil.move(exe_file, final_exe)
                print(f"[信息] 已移动到: {final_exe}")
            
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
        print("1. Nuitka 未安装，请运行: pip install nuitka")
        print("2. 依赖库缺失，请运行: pip install -r requirements.txt")
    except FileNotFoundError:
        print("[错误] Python 或 Nuitka 未找到")


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
    try:
        if os.path.exists(dist_dir):
            abs_dist_dir = os.path.abspath(dist_dir)
            if sys.platform == "win32":
                os.startfile(abs_dist_dir)
            elif sys.platform == "darwin":
                subprocess.run(["open", abs_dist_dir])
            else:
                subprocess.run(["xdg-open", abs_dist_dir])
    except Exception as e:
        print(f"[警告] 无法自动打开目录: {e}")


if __name__ == "__main__":
    build()