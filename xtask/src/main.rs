//! Z_COM 打包与版本管理工具。
//!
//! ```text
//! cargo xtask pack           当前平台便携包
//! cargo xtask pack-win       Windows 便携包：dist/Z_COM_V版本号/Z_COM.exe
//! cargo xtask pack-release   当前平台发布文件，全部平台齐全后生成 release-manifest.json
//! cargo xtask version 0.3.1  统一修改版本号
//! ```

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

type Result<T = ()> = std::result::Result<T, String>;

const USAGE: &str = "\
用法：cargo xtask <命令>

  pack              当前平台便携包（Windows: dist/Z_COM_V版本号/Z_COM.exe，
                    Linux: dist/linux-x86_64/Z_COM_V版本号/Z_COM）
  pack-win          同 pack，但只允许在 Windows 执行
  pack-release      构建当前平台发布文件到 dist/release/v版本号/，
                    Windows 与 Linux 产物齐全后自动生成 release-manifest.json
  version <版本号>  统一修改 Cargo.toml 与 Cargo.lock 中的版本号";

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let result = match arguments.first().map(String::as_str) {
        Some("pack") => pack(false),
        Some("pack-win") => pack(true),
        Some("pack-release") => pack_release(),
        Some("version") => set_version(arguments.get(1).map(String::as_str)),
        Some("help" | "-h" | "--help") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("未知命令：{other}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("错误：{error}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask 位于仓库子目录")
        .to_path_buf()
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| format!("读取 {} 失败：{error}", path.display()))
}

fn write(path: &Path, content: &str) -> Result {
    fs::write(path, content).map_err(|error| format!("写入 {} 失败：{error}", path.display()))
}

/// `Cargo.toml` 中 `[package]` 段的版本行位置
fn package_version_line(cargo_toml: &str) -> Option<(usize, String)> {
    let mut in_package = false;
    for (index, line) in cargo_toml.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
        } else if in_package && trimmed.starts_with("version") {
            let value = trimmed.split_once('=')?.1.trim().trim_matches('"');
            return Some((index, value.to_owned()));
        }
    }
    None
}

fn version() -> Result<String> {
    package_version_line(&read(&root().join("Cargo.toml"))?)
        .map(|(_, version)| version)
        .ok_or_else(|| "无法从 Cargo.toml 读取版本号".into())
}

fn cargo_build_release() -> Result {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .args(["build", "--release", "--locked", "--package", "z-com"])
        .current_dir(root())
        .status()
        .map_err(|error| format!("无法启动 cargo：{error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo build 失败：{status}"))
    }
}

fn executable_name(base: &str) -> String {
    format!("{base}{}", env::consts::EXE_SUFFIX)
}

/// 构建产物目录，遵循 `CARGO_TARGET_DIR`（如 WSL 中把编译缓存放在 Linux 文件系统以免与 Windows 共用）
fn release_executable() -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|directory| if directory.is_absolute() { directory } else { root().join(directory) })
        .unwrap_or_else(|| root().join("target"))
        .join("release")
        .join(executable_name("z-com"))
}

fn copy_executable(source: &Path, target: &Path) -> Result {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建 {} 失败：{error}", parent.display()))?;
    }
    fs::copy(source, target).map_err(|error| {
        if error.kind() == io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(32) {
            format!(
                "无法覆盖 {}：该程序正在运行，请关闭后重新执行打包命令",
                target.display()
            )
        } else {
            format!("复制到 {} 失败：{error}", target.display())
        }
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("设置可执行权限失败：{error}"))?;
    }
    Ok(())
}

fn sha256(path: &Path) -> Result<(u64, String)> {
    let content = fs::read(path).map_err(|error| format!("读取 {} 失败：{error}", path.display()))?;
    Ok((content.len() as u64, hex(&Sha256::digest(&content))))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn pack(windows_only: bool) -> Result {
    if windows_only && !cfg!(windows) {
        return Err("pack-win 只能在 Windows 环境执行".into());
    }
    let version = version()?;
    let app_name = format!("Z_COM_V{version}");
    let dist = if cfg!(windows) {
        root().join("dist")
    } else {
        let platform = if cfg!(target_os = "macos") {
            "macos"
        } else {
            env::consts::OS
        };
        let architecture = match env::consts::ARCH {
            "aarch64" => "arm64",
            other => other,
        };
        root()
            .join("dist")
            .join(format!("{platform}-{architecture}"))
    };
    let output = dist.join(&app_name).join(executable_name("Z_COM"));
    println!("\nZ_COM 便携版打包");
    println!("版本：V{version}");
    println!("输出：{}\n", output.parent().unwrap().display());
    cargo_build_release()?;
    copy_executable(&release_executable(), &output)?;
    let (size, hash) = sha256(&output)?;
    println!("\n打包完成：{}", output.display());
    println!("文件大小：{:.2} MiB", size as f64 / 1024.0 / 1024.0);
    println!("SHA-256：{hash}");
    Ok(())
}

/// 参与发布源码指纹的文件；两个平台必须由相同源码构建才会生成清单
const FINGERPRINT_INPUTS: &[&str] = &[
    "LICENSE",
    "NOTICE",
    "THIRD_PARTY_NOTICES.md",
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "assets",
    "src",
];

fn collect_files(relative: &Path, output: &mut Vec<String>) -> Result {
    let absolute = root().join(relative);
    if absolute.is_file() {
        output.push(relative.to_string_lossy().replace('\\', "/"));
        return Ok(());
    }
    let entries = fs::read_dir(&absolute)
        .map_err(|error| format!("读取目录 {} 失败：{error}", absolute.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        collect_files(&relative.join(entry.file_name()), output)?;
    }
    Ok(())
}

fn source_fingerprint() -> Result<String> {
    let mut files = Vec::new();
    for input in FINGERPRINT_INPUTS {
        collect_files(Path::new(input), &mut files)?;
    }
    files.sort();
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.as_bytes());
        hasher.update([0]);
        // 统一换行符，避免 Windows 与 Linux 检出差异导致指纹不同
        let content = fs::read(root().join(&file)).map_err(|error| error.to_string())?;
        let normalized: Vec<u8> = content
            .windows(2)
            .filter(|pair| pair != b"\r\n")
            .map(|pair| pair[0])
            .chain(content.last().copied())
            .collect();
        hasher.update(&normalized);
        hasher.update([0]);
    }
    Ok(hex(&hasher.finalize()))
}

struct ReleasePlatform {
    key: &'static str,
    name: &'static str,
}

const RELEASE_PLATFORMS: &[ReleasePlatform] = &[
    ReleasePlatform {
        key: "windows-x86_64",
        name: "Z_COM-windows-x86_64.exe",
    },
    ReleasePlatform {
        key: "linux-x86_64",
        name: "Z_COM-linux-x86_64",
    },
];

fn pack_release() -> Result {
    let key = match (env::consts::OS, env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        (os, arch) => {
            return Err(format!(
                "发布打包当前只支持 Windows/Linux x86_64，当前环境为 {os}/{arch}"
            ));
        }
    };
    let platform = RELEASE_PLATFORMS
        .iter()
        .find(|platform| platform.key == key)
        .unwrap();
    let version = version()?;
    let fingerprint = source_fingerprint()?;
    cargo_build_release()?;

    let release = root().join("dist").join("release").join(format!("v{version}"));
    let state = root()
        .join("dist")
        .join(".release-state")
        .join(format!("v{version}"));
    for directory in [&release, &state] {
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    }
    // 清理误运行产生的运行时目录，发布目录只保留可发布文件
    for entry in fs::read_dir(&release).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let runtime = ["config", "logs", "locks", ".update"].contains(&name.as_str())
            || name
                .strip_prefix("instance_")
                .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()));
        if runtime && entry.path().is_dir() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
    let output = release.join(platform.name);
    copy_executable(&release_executable(), &output)?;
    write(
        &state.join(format!("{key}.json")),
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "version": version,
                "platform": key,
                "sourceFingerprint": fingerprint,
            }))
            .unwrap()
        ),
    )?;

    let mut assets = serde_json::Map::new();
    let mut missing = Vec::new();
    for asset in RELEASE_PLATFORMS {
        let file = release.join(asset.name);
        let state_file = state.join(format!("{}.json", asset.key));
        let fresh = read(&state_file)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .is_some_and(|state| {
                state["version"] == version.as_str()
                    && state["sourceFingerprint"] == fingerprint.as_str()
            });
        match (fresh, sha256(&file)) {
            (true, Ok((size, hash))) if size > 0 => {
                assets.insert(
                    asset.key.into(),
                    json!({ "name": asset.name, "size": size, "sha256": hash }),
                );
            }
            _ => {
                missing.push(asset.key);
                let _ = fs::remove_file(&file);
                let _ = fs::remove_file(&state_file);
            }
        }
    }

    let manifest = release.join("release-manifest.json");
    if missing.is_empty() {
        let content = json!({ "version": version, "assets": assets });
        write(
            &manifest,
            &format!("{}\n", serde_json::to_string_pretty(&content).unwrap()),
        )?;
        println!("\nRelease 已完整生成：{}", release.display());
        println!("清单：{}", manifest.display());
    } else {
        let _ = fs::remove_file(&manifest);
        println!("\n已生成 {key} 发布文件：{}", output.display());
        println!("尚缺或需要按当前源码重建：{}。", missing.join("、"));
        println!("请在对应平台的同一工作区再次执行 cargo xtask pack-release。");
        println!("缺少全部平台产物时不会生成 release-manifest.json，避免误发布不完整清单。");
    }
    Ok(())
}

fn valid_version(version: &str) -> bool {
    let (core, _) = version.split_once(['-', '+']).unwrap_or((version, ""));
    let parts: Vec<_> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn set_version(argument: Option<&str>) -> Result {
    let version = argument
        .map(|value| value.trim().trim_start_matches(['v', 'V']))
        .filter(|value| valid_version(value))
        .ok_or("用法：cargo xtask version 0.3.1")?;

    let cargo_toml_path = root().join("Cargo.toml");
    let cargo_toml = read(&cargo_toml_path)?;
    let (index, _) =
        package_version_line(&cargo_toml).ok_or("无法更新 Cargo.toml 的 package.version")?;
    let updated: Vec<String> = cargo_toml
        .lines()
        .enumerate()
        .map(|(line, text)| {
            if line == index {
                format!("version = \"{version}\"")
            } else {
                text.to_owned()
            }
        })
        .collect();
    let newline = if cargo_toml.contains("\r\n") { "\r\n" } else { "\n" };
    write(&cargo_toml_path, &(updated.join(newline) + newline))?;

    let lock_path = root().join("Cargo.lock");
    let lock = read(&lock_path)?;
    let marker = "name = \"z-com\"";
    let start = lock
        .find(&format!("{marker}\nversion = \""))
        .or_else(|| lock.find(&format!("{marker}\r\nversion = \"")))
        .ok_or("无法更新 Cargo.lock 中的 z-com 版本")?;
    let value_start = lock[start..].find("version = \"").unwrap() + start + "version = \"".len();
    let value_end = lock[value_start..].find('"').unwrap() + value_start;
    write(
        &lock_path,
        &format!("{}{version}{}", &lock[..value_start], &lock[value_end..]),
    )?;

    println!("版本已统一更新为 {version}：");
    println!("- Cargo.toml");
    println!("- Cargo.lock");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_package_version_only() {
        let toml = "[package]\nname = \"z-com\"\nversion = \"1.2.3\"\n\n[dependencies]\nversion = \"9\"\n";
        assert_eq!(package_version_line(toml), Some((2, "1.2.3".into())));
    }

    #[test]
    fn validates_semver() {
        assert!(valid_version("0.3.0"));
        assert!(valid_version("1.0.0-rc.1"));
        assert!(!valid_version("0.3"));
        assert!(!valid_version("a.b.c"));
    }
}
