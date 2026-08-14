use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::update::{ResolvedUpdate, build_client};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: u64,
    total: u64,
    percent: u8,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DownloadedUpdate {
    pub(crate) version: String,
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

pub(crate) fn download(
    app: &AppHandle,
    candidate: &ResolvedUpdate,
    cancel: &AtomicBool,
) -> Result<(DownloadedUpdate, PathBuf), String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("无法确定当前程序路径: {error}"))?;
    let executable_root = executable
        .parent()
        .ok_or_else(|| "无法确定当前程序目录".to_string())?;
    let update_directory = executable_root.join(".update");
    fs::create_dir_all(&update_directory).map_err(|error| {
        format!(
            "无法创建更新暂存目录 {}: {error}",
            update_directory.display()
        )
    })?;
    let destination = update_directory.join(&candidate.asset_name);
    let partial = update_directory.join(format!("{}.part", candidate.asset_name));
    let sources = std::iter::once((candidate.source.as_str(), candidate.download_url.as_str()))
        .chain(
            candidate
                .fallback_url
                .as_deref()
                .map(|url| ("备用镜像", url)),
        )
        .collect::<Vec<_>>();
    let mut errors = Vec::new();
    for (source, url) in sources {
        let _ = fs::remove_file(&partial);
        match download_from(app, candidate, source, url, &partial, cancel) {
            Ok(()) => {
                let _ = fs::remove_file(&destination);
                fs::rename(&partial, &destination)
                    .map_err(|error| format!("无法完成更新文件暂存: {error}"))?;
                let result = DownloadedUpdate {
                    version: candidate.version.clone(),
                    path: destination.to_string_lossy().into_owned(),
                    size: candidate.asset_size,
                    sha256: candidate.sha256.clone(),
                };
                return Ok((result, destination));
            }
            Err(error) => {
                errors.push(format!("{source}: {error}"));
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }
    let _ = fs::remove_file(&partial);
    Err(format!("更新下载失败：{}", errors.join("；")))
}

fn download_from(
    app: &AppHandle,
    candidate: &ResolvedUpdate,
    source: &str,
    url: &str,
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    validate_download_url(url)?;
    let client = build_client(Duration::from_secs(180))?;
    let mut response = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("请求失败: {error}"))?;
    let mut file =
        File::create(destination).map_err(|error| format!("无法创建临时更新文件: {error}"))?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("用户已取消下载".into());
        }
        let count = response
            .read(&mut buffer)
            .map_err(|error| format!("读取下载数据失败: {error}"))?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|error| format!("写入更新文件失败: {error}"))?;
        hasher.update(&buffer[..count]);
        downloaded = downloaded.saturating_add(count as u64);
        if downloaded > candidate.asset_size {
            return Err("下载数据超过清单声明大小".into());
        }
        let percent =
            ((downloaded.saturating_mul(100) / candidate.asset_size.max(1)).min(100)) as u8;
        let _ = app.emit(
            "update-progress",
            UpdateProgress {
                downloaded,
                total: candidate.asset_size,
                percent,
                source: source.to_string(),
            },
        );
    }
    file.flush()
        .map_err(|error| format!("刷新更新文件失败: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("同步更新文件失败: {error}"))?;
    if downloaded != candidate.asset_size {
        return Err(format!(
            "文件大小不匹配，期望 {} 字节，实际 {} 字节",
            candidate.asset_size, downloaded
        ));
    }
    let actual_hash = digest_hex(hasher.finalize().as_slice());
    if !actual_hash.eq_ignore_ascii_case(&candidate.sha256) {
        return Err(format!(
            "SHA-256 校验失败，期望 {}，实际 {}",
            candidate.sha256, actual_hash
        ));
    }
    Ok(())
}

pub(crate) fn verify_file(
    path: &Path,
    expected_size: u64,
    expected_hash: &str,
) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("无法读取更新文件 {}: {error}", path.display()))?;
    if metadata.len() != expected_size {
        return Err(format!("更新文件大小已变化: {}", path.display()));
    }
    let mut file = File::open(path)
        .map_err(|error| format!("无法打开更新文件 {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("无法读取更新文件: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual_hash = digest_hex(hasher.finalize().as_slice());
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        return Err("更新文件 SHA-256 已变化".into());
    }
    Ok(())
}

fn validate_download_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|error| format!("下载地址无效: {error}"))?;
    if parsed.scheme() != "https" {
        return Err("更新文件只允许通过 HTTPS 下载".into());
    }
    match parsed.host_str() {
        Some("github.com") | Some("objects.githubusercontent.com") | Some("gitee.com") => Ok(()),
        Some(host) => Err(format!("不允许从非发布站点下载更新: {host}")),
        None => Err("更新下载地址缺少主机名".into()),
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::validate_download_url;

    #[test]
    fn only_accepts_known_https_release_hosts() {
        assert!(validate_download_url("https://github.com/a/b/file").is_ok());
        assert!(validate_download_url("https://gitee.com/a/b/file").is_ok());
        assert!(validate_download_url("http://github.com/a/b/file").is_err());
        assert!(validate_download_url("https://example.com/file").is_err());
    }
}
