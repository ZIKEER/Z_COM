#!/usr/bin/env bash
set -Eeuo pipefail

if [[ "$(id -u)" -ne 0 ]]; then
    echo "[错误] 安装系统依赖需要 root 权限，请使用 sudo 运行此脚本。" >&2
    exit 1
fi

packages=(
    python3-venv
    libxcb-xkb1
    libxkbcommon-x11-0
    libxcb-keysyms1
    libxcb-icccm4
    libxcb-cursor0
    libxcb-render-util0
    libxcb-image0
)

missing_packages=()
for package_name in "${packages[@]}"; do
    if ! dpkg-query -W -f='${Status}' "${package_name}" 2>/dev/null | grep -q "ok installed"; then
        missing_packages+=("${package_name}")
    fi
done

if [[ "${#missing_packages[@]}" -eq 0 ]]; then
    echo "[信息] Linux 系统构建依赖已安装。"
    exit 0
fi

echo "[信息] 安装 Linux 系统构建依赖: ${missing_packages[*]}"
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y "${missing_packages[@]}"
