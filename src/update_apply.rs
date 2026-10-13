use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use crate::{update::ResolvedUpdate, update_download::verify_file};

pub(crate) fn launch(candidate: &ResolvedUpdate, downloaded_path: &Path) -> Result<(), String> {
    verify_file(downloaded_path, candidate.asset_size, &candidate.sha256)?;
    let target = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|error| format!("无法确定当前程序路径: {error}"))?;
    let extension = target
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();
    let updater = std::env::temp_dir().join(format!(
        "Z_COM-updater-{}-{}{}",
        candidate.version,
        std::process::id(),
        extension
    ));
    let _ = fs::remove_file(&updater);
    fs::copy(&target, &updater)
        .map_err(|error| format!("无法创建临时更新进程 {}: {error}", updater.display()))?;
    set_executable(&updater)?;
    Command::new(&updater)
        .arg("--apply-update")
        .arg(&target)
        .arg(downloaded_path)
        .arg(&candidate.sha256)
        .arg(candidate.asset_size.to_string())
        .arg(&candidate.version)
        .current_dir(target.parent().unwrap_or_else(|| Path::new(".")))
        .spawn()
        .map_err(|error| format!("无法启动更新进程: {error}"))?;
    Ok(())
}

pub fn run_update_mode() -> Option<Result<(), String>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--apply-update")) {
        return None;
    }
    let result = parse_apply_args(&mut args).and_then(|request| apply_update(&request));
    if let Err(error) = &result
        && let Some(target) = std::env::args_os().nth(2)
    {
        record_error(Path::new(&target), error);
    }
    Some(result)
}

pub(crate) fn schedule_cleanup() {
    let args = std::env::args_os().collect::<Vec<_>>();
    let Some(index) = args
        .iter()
        .position(|argument| argument == "--cleanup-update")
    else {
        return;
    };
    let Some(updater) = args.get(index + 1).map(PathBuf::from) else {
        return;
    };
    let Some(backup) = args.get(index + 2).map(PathBuf::from) else {
        return;
    };
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        let _ = fs::remove_file(&backup);
        for _ in 0..20 {
            match fs::remove_file(&updater) {
                Ok(()) => break,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(_) => thread::sleep(Duration::from_millis(250)),
            }
        }
        if let Some(update_directory) = backup.parent().map(|root| root.join(".update")) {
            let _ = fs::remove_dir_all(update_directory);
        }
    });
}

pub(crate) fn take_startup_error() -> String {
    let Ok(executable) = std::env::current_exe() else {
        return String::new();
    };
    let Some(root) = executable.parent() else {
        return String::new();
    };
    let path = root.join(".update").join("update-error.log");
    let message = fs::read_to_string(&path).unwrap_or_default();
    if !message.is_empty() {
        let _ = fs::remove_file(path);
    }
    message
}

struct ApplyRequest {
    target: PathBuf,
    source: PathBuf,
    sha256: String,
    size: u64,
    version: String,
}

fn parse_apply_args(args: &mut impl Iterator<Item = OsString>) -> Result<ApplyRequest, String> {
    let target = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "更新模式缺少目标程序路径".to_string())?;
    let source = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "更新模式缺少新程序路径".to_string())?;
    let sha256 = args
        .next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| "更新模式缺少 SHA-256".to_string())?;
    let size = args
        .next()
        .ok_or_else(|| "更新模式缺少文件大小".to_string())?
        .to_string_lossy()
        .parse::<u64>()
        .map_err(|error| format!("更新文件大小无效: {error}"))?;
    let version = args
        .next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| "更新模式缺少版本号".to_string())?;
    Ok(ApplyRequest {
        target,
        source,
        sha256,
        size,
        version,
    })
}

fn apply_update(request: &ApplyRequest) -> Result<(), String> {
    verify_file(&request.source, request.size, &request.sha256)?;
    let backup = backup_path(&request.target);
    let _ = fs::remove_file(&backup);
    let mut last_error = None;
    for _ in 0..120 {
        match fs::rename(&request.target, &backup) {
            Ok(()) => {
                last_error = None;
                break;
            }
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(250));
            }
        }
    }
    if let Some(error) = last_error {
        return Err(format!("等待主程序退出超时，无法创建备份: {error}"));
    }
    if let Err(error) = fs::rename(&request.source, &request.target) {
        let _ = fs::rename(&backup, &request.target);
        return Err(format!("替换主程序失败，已恢复旧版本: {error}"));
    }
    if let Err(error) = set_executable(&request.target) {
        rollback(&request.target, &backup);
        return Err(error);
    }
    let updater =
        std::env::current_exe().map_err(|error| format!("无法确定临时更新程序路径: {error}"))?;
    match Command::new(&request.target)
        .arg("--cleanup-update")
        .arg(&updater)
        .arg(&backup)
        .arg("--updated-from")
        .arg(&request.version)
        .current_dir(request.target.parent().unwrap_or_else(|| Path::new(".")))
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(error) => {
            rollback(&request.target, &backup);
            Err(format!("启动新版本失败，已恢复旧版本: {error}"))
        }
    }
}

fn backup_path(target: &Path) -> PathBuf {
    let mut name = target
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("Z_COM"));
    name.push(".bak");
    target.with_file_name(name)
}

fn rollback(target: &Path, backup: &Path) {
    let _ = fs::remove_file(target);
    let _ = fs::rename(backup, target);
}

fn record_error(target: &Path, error: &str) {
    let Some(root) = target.parent() else {
        return;
    };
    let directory = root.join(".update");
    let _ = fs::create_dir_all(&directory);
    let _ = fs::write(directory.join("update-error.log"), error);
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("无法设置新版本执行权限: {error}"))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::backup_path;

    #[test]
    fn appends_backup_suffix_without_changing_extension() {
        assert_eq!(
            backup_path(Path::new("C:/Tools/Z_COM.exe")),
            Path::new("C:/Tools/Z_COM.exe.bak")
        );
        assert_eq!(
            backup_path(Path::new("/opt/zcom/Z_COM")),
            Path::new("/opt/zcom/Z_COM.bak")
        );
    }
}
