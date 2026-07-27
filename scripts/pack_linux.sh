#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
PYTHON_BIN="${PYTHON_BIN:-python3}"
VENV_DIR="${Z_COM_LINUX_VENV:-${PROJECT_ROOT}/.venv-linux}"

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "[错误] 此脚本只能在 Linux 或 WSL 中运行。" >&2
    exit 1
fi

cd "${PROJECT_ROOT}"

if [[ ! -x "${VENV_DIR}/bin/python" ]] || ! "${VENV_DIR}/bin/python" -m pip --version >/dev/null 2>&1; then
    echo "[信息] 创建 Linux 虚拟环境: ${VENV_DIR}"
    if ! "${PYTHON_BIN}" -m venv "${VENV_DIR}"; then
        echo "[错误] 无法创建虚拟环境。Ubuntu/Debian 请先运行:" >&2
        echo "       sudo apt-get install python3-venv" >&2
        exit 1
    fi
fi

echo "[信息] 安装/更新 Linux 打包依赖..."
"${VENV_DIR}/bin/python" -m pip install --upgrade pip
"${VENV_DIR}/bin/python" -m pip install -r requirements.txt

echo "[信息] 开始生成 Linux 可执行文件..."
Z_COM_NO_OPEN=1 "${VENV_DIR}/bin/python" pack.py

echo "[完成] Linux 产物位于: ${PROJECT_ROOT}/dist/linux-$(uname -m)"
