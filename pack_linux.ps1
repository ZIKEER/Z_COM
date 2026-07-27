param(
    [string]$Distro = "Ubuntu-20.04"
)

$ErrorActionPreference = "Stop"
$projectRoot = $PSScriptRoot

if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
    throw "未找到 wsl.exe，请先安装并启用 WSL。"
}

$installedDistros = @(wsl.exe --list --quiet) -replace "`0", "" | ForEach-Object { $_.Trim() } | Where-Object { $_ }
if ($Distro -notin $installedDistros) {
    throw "未找到 WSL 发行版 '$Distro'。可用发行版: $($installedDistros -join ', ')"
}

$wslPathInput = $projectRoot -replace "\\", "/"
$linuxProjectRoot = (wsl.exe -d $Distro -- wslpath -a $wslPathInput).Trim()
if (-not $linuxProjectRoot) {
    throw "无法将项目路径转换为 WSL 路径: $projectRoot"
}

Write-Host "[信息] 使用 WSL 发行版: $Distro"
Write-Host "[信息] Linux 项目路径: $linuxProjectRoot"

Write-Host "[信息] 检查 Linux 系统构建依赖..."
wsl.exe -d $Distro --user root --cd $linuxProjectRoot -- bash scripts/install_linux_build_deps.sh
if ($LASTEXITCODE -ne 0) {
    throw "Linux 系统构建依赖安装失败，退出代码: $LASTEXITCODE"
}

wsl.exe -d $Distro --cd $linuxProjectRoot -- bash scripts/pack_linux.sh
if ($LASTEXITCODE -ne 0) {
    throw "Linux 打包失败，退出代码: $LASTEXITCODE"
}
