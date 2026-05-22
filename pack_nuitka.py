"""Z_COM 打包脚本 - 使用 Nuitka"""
import os
import sys
import shutil
import subprocess
import glob
from datetime import datetime
from pathlib import Path

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
    patterns = ["dist_nuitka/*.dist", "dist_nuitka/*.build", "dist_nuitka/*.onefile-build"]
    for pattern in patterns:
        for path in glob.glob(pattern):
            if os.path.isdir(path):
                shutil.rmtree(path)
                print(f"  已删除临时目录: {path}")


def remove_unnecessary_files(dist_dir):
    """删除打包产物中未使用的大体积文件，减小体积"""
    d = Path(dist_dir)
    if not d.is_dir():
        return

    # ---------------------------------------------------------------
    # 1. 未使用的 Qt C++ DLL
    # ---------------------------------------------------------------
    unneeded_dlls = {
        "qt6pdf.dll",        # 4.4 MB — 未使用 QtPdf
        "qt6network.dll",    # 1.7 MB — Socket 走 Python stdlib，非 QtNetwork
        "qt6svg.dll",        # 0.6 MB — 未使用 QtSvg
        "qt6svgwidgets.dll", # 0.1 MB — 未使用
        "libcrypto-3.dll",   # 5.0 MB — OpenSSL，QtNetwork SSL 专用
        "libssl-3.dll",      # 跟随 libcrypto
    }

    # ---------------------------------------------------------------
    # 2. 未使用的 Qt .pyd 绑定
    # ---------------------------------------------------------------
    unneeded_pyds = {
        "QtNetwork.pyd",     # 1.0 MB
        "QtSvg.pyd",
        "QtSvgWidgets.pyd",
    }

    # ---------------------------------------------------------------
    # 3. 不必要的图片格式插件（只用 .ico，不用 jpeg/webp/tiff 等）
    # ---------------------------------------------------------------
    unneeded_image_plugins = {
        "qjpeg.dll",    # 0.6 MB
        "qwebp.dll",    # 0.5 MB
        "qtiff.dll",    # 0.4 MB
        "qicns.dll",    # 0.1 MB (macOS only)
        "qtga.dll",
        "qwbmp.dll",
        "qsvg.dll",     # SVG image plugin — 不用 QtSvg 则无用
        "qsvgicon.dll",
    }
    # 保留: qico.dll, qgif.dll, qpdf.dll (体积均 < 1 KB)

    deleted_count = 0
    deleted_size = 0

    for file in d.rglob("*"):
        if not file.is_file():
            continue
        name = file.name
        if name in unneeded_dlls or name in unneeded_pyds or name in unneeded_image_plugins:
            sz = file.stat().st_size
            file.unlink()
            deleted_count += 1
            deleted_size += sz
            print(f"  删除: {file.relative_to(d)} ({sz / 1024:.0f} KB)")

    if deleted_count:
        print(f"[信息] 共删除 {deleted_count} 个文件，节省 {deleted_size / 1024 / 1024:.1f} MB")
    else:
        print("[信息] 未找到需要删除的文件")


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
        "--standalone",  # 独立模式（非单文件，输出目录）
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
        "--include-module=pylink",
        "--include-module=psutil",
        "--include-module=src.windows.main_window",
        "--include-module=src.windows.serial_settings_dialog",
        "--include-module=src.windows.extended_send_widget",
        "--include-module=src.core.config_manager",
        "--include-module=src.core.extended_send_manager",
        "--include-module=ui.Ui_main_window",
        "--include-module=ui.Ui_serial_settings_dialog",
        "--include-module=ui.Ui_extended_send_widget",
        "--include-module=src.io.rtt_manager",
        "--include-module=src.version",
        # 排除不需要的模块
        "--noinclude-custom-mode=tkinter:error",
        "--noinclude-custom-mode=matplotlib:error",
        "--noinclude-custom-mode=numpy:error",
        "--noinclude-custom-mode=pandas:error",
        # PySide6 子模块不在此排除（__init__.py 条件式导入会打断编译）
        # 未用的 Qt DLL / .pyd 由构建后的 remove_unnecessary_files() 删除
        # 编译优化 — 仅增加编译时间，运行时无负面影响
        "--lto=yes",
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
            # Nuitka 默认输出为 run.dist，重命名为目标目录
            print("[信息] 查找并重命名输出目录...")
            dist_source = os.path.abspath("dist_nuitka/run.dist")
            dist_target = os.path.abspath(dist_dir)
            if os.path.isdir(dist_source):
                if os.path.isdir(dist_target):
                    shutil.rmtree(dist_target)
                shutil.move(dist_source, dist_target)
                dist_dir = dist_target  # 后续用绝对路径
                print(f"[信息] 已重命名目录为: {dist_dir}")
            
            # 清理 Nuitka 临时目录
            print("[信息] 清理临时文件...")
            clean_nuitka_temps()
            
            # 删除未使用的 Qt DLL / 插件 / PYD
            print("[信息] 删除未使用的 Qt 文件以减小体积...")
            remove_unnecessary_files(dist_dir)
            
            
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