use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};

use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};

const GITHUB_RELEASE_API: &str = "https://api.github.com/repos/ZIKEER/Z_COM/releases/latest";
const GITEE_RELEASE_API: &str = "https://gitee.com/api/v5/repos/zzk11111111/Z_COM/releases/latest";

#[derive(Debug, Clone)]
pub(crate) struct ResolvedUpdate {
    pub(crate) version: String,
    pub(crate) title: String,
    pub(crate) notes: String,
    pub(crate) source: String,
    pub(crate) asset_name: String,
    pub(crate) asset_size: u64,
    pub(crate) sha256: String,
    pub(crate) download_url: String,
    pub(crate) fallback_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UpdateState {
    pub(crate) candidate: Option<ResolvedUpdate>,
    pub(crate) downloaded_path: Option<std::path::PathBuf>,
    pub(crate) cancel_download: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateInfo {
    pub(crate) available: bool,
    pub(crate) current_version: String,
    pub(crate) version: String,
    pub(crate) title: String,
    pub(crate) notes: String,
    pub(crate) source: String,
    pub(crate) asset_name: String,
    pub(crate) asset_size: u64,
    pub(crate) mirror_available: bool,
}

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    #[serde(default)]
    draft: Option<bool>,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    size: Option<u64>,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseManifest {
    version: String,
    assets: HashMap<String, ManifestAsset>,
}

#[derive(Debug, Deserialize)]
struct ManifestAsset {
    name: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Clone)]
struct SourceUpdate {
    source: String,
    version: Version,
    title: String,
    notes: String,
    asset_name: String,
    asset_size: u64,
    sha256: String,
    download_url: String,
}

pub(crate) fn check(current_version: &str) -> Result<(UpdateInfo, Option<ResolvedUpdate>), String> {
    let current = Version::parse(current_version)
        .map_err(|error| format!("当前版本号 {current_version} 无效: {error}"))?;
    let client = build_client(Duration::from_secs(20))?;
    let github_client = client.clone();
    let gitee_client = client;
    let github = thread::spawn(move || fetch_source(&github_client, "GitHub", GITHUB_RELEASE_API));
    let gitee = thread::spawn(move || fetch_source(&gitee_client, "Gitee", GITEE_RELEASE_API));
    let github = github
        .join()
        .map_err(|_| "GitHub 更新检查线程异常退出".to_string())?;
    let gitee = gitee
        .join()
        .map_err(|_| "Gitee 更新检查线程异常退出".to_string())?;

    let mut failures = Vec::new();
    let mut candidates = Vec::new();
    match github {
        Ok(candidate) => candidates.push(candidate),
        Err(error) => failures.push(error),
    }
    match gitee {
        Ok(candidate) => candidates.push(candidate),
        Err(error) => failures.push(error),
    }
    if candidates.is_empty() {
        return Err(format!("两个更新源均不可用：{}", failures.join("；")));
    }

    candidates.sort_by(|left, right| right.version.cmp(&left.version));
    let latest_version = candidates[0].version.clone();
    if latest_version <= current {
        return Ok((no_update_info(current_version), None));
    }

    let mut latest = candidates
        .into_iter()
        .filter(|candidate| candidate.version == latest_version)
        .collect::<Vec<_>>();
    latest.sort_by_key(|candidate| if candidate.source == "Gitee" { 0 } else { 1 });
    if latest.len() > 1 && !same_asset(&latest[0], &latest[1]) {
        return Err(format!(
            "{} 的 GitHub 与 Gitee 文件校验信息不一致，已拒绝更新",
            latest_version
        ));
    }

    let primary = latest.remove(0);
    let fallback_url = latest
        .first()
        .map(|candidate| candidate.download_url.clone());
    let resolved = ResolvedUpdate {
        version: primary.version.to_string(),
        title: primary.title,
        notes: primary.notes,
        source: primary.source,
        asset_name: primary.asset_name,
        asset_size: primary.asset_size,
        sha256: primary.sha256,
        download_url: primary.download_url,
        fallback_url,
    };
    let info = UpdateInfo {
        available: true,
        current_version: current_version.to_string(),
        version: resolved.version.clone(),
        title: resolved.title.clone(),
        notes: resolved.notes.clone(),
        source: resolved.source.clone(),
        asset_name: resolved.asset_name.clone(),
        asset_size: resolved.asset_size,
        mirror_available: resolved.fallback_url.is_some(),
    };
    Ok((info, Some(resolved)))
}

pub(crate) fn build_client(timeout: Duration) -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(6))
        .timeout(timeout)
        .user_agent(format!("Z_COM-Updater/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("无法初始化更新网络客户端: {error}"))
}

fn fetch_source(client: &Client, source: &str, api_url: &str) -> Result<SourceUpdate, String> {
    let release = client
        .get(api_url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("{source} Release 查询失败: {error}"))?
        .json::<ReleaseResponse>()
        .map_err(|error| format!("{source} Release 响应解析失败: {error}"))?;
    if release.draft.unwrap_or(false) || release.prerelease {
        return Err(format!("{source} 最新 Release 不是正式版本"));
    }
    let version = Version::parse(release.tag_name.trim_start_matches(['v', 'V']))
        .map_err(|error| format!("{source} Release tag {} 无效: {error}", release.tag_name))?;
    let platform = platform_key()?;
    let expected_name = asset_name(&version)?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == expected_name)
        .ok_or_else(|| format!("{source} Release 缺少 {expected_name}"))?;
    let manifest_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "release-manifest.json")
        .ok_or_else(|| format!("{source} Release 缺少 release-manifest.json"))?;
    let manifest = client
        .get(&manifest_asset.browser_download_url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("{source} 更新清单下载失败: {error}"))?
        .json::<ReleaseManifest>()
        .map_err(|error| format!("{source} 更新清单解析失败: {error}"))?;
    if manifest.version != version.to_string() {
        return Err(format!(
            "{source} 更新清单版本 {} 与 Release {} 不一致",
            manifest.version, version
        ));
    }
    let manifest_asset = manifest
        .assets
        .get(platform)
        .ok_or_else(|| format!("{source} 更新清单缺少平台 {platform}"))?;
    validate_manifest_asset(source, &expected_name, asset, manifest_asset)?;
    let title = release
        .name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("Z_COM v{version}"));
    Ok(SourceUpdate {
        source: source.to_string(),
        version,
        title,
        notes: release.body.unwrap_or_default(),
        asset_name: manifest_asset.name.clone(),
        asset_size: manifest_asset.size,
        sha256: manifest_asset.sha256.to_ascii_uppercase(),
        download_url: asset.browser_download_url.clone(),
    })
}

fn validate_manifest_asset(
    source: &str,
    expected_name: &str,
    release_asset: &ReleaseAsset,
    manifest_asset: &ManifestAsset,
) -> Result<(), String> {
    if manifest_asset.name != expected_name {
        return Err(format!("{source} 更新清单文件名与 Release 不一致"));
    }
    if let Some(size) = release_asset.size
        && size != manifest_asset.size
    {
        return Err(format!("{source} 更新文件大小与清单不一致"));
    }
    if manifest_asset.sha256.len() != 64
        || !manifest_asset
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!("{source} 更新清单 SHA-256 无效"));
    }
    Ok(())
}

fn same_asset(left: &SourceUpdate, right: &SourceUpdate) -> bool {
    left.asset_name == right.asset_name
        && left.asset_size == right.asset_size
        && left.sha256.eq_ignore_ascii_case(&right.sha256)
}

fn platform_key() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok("windows-x86_64"),
        ("linux", "x86_64") => Ok("linux-x86_64"),
        (os, arch) => Err(format!("当前平台暂不支持自动更新: {os}-{arch}")),
    }
}

fn asset_name(version: &Version) -> Result<String, String> {
    match platform_key()? {
        "windows-x86_64" => Ok(format!("Z_COM-v{version}-windows-x86_64.exe")),
        "linux-x86_64" => Ok(format!("Z_COM-v{version}-linux-x86_64")),
        platform => Err(format!("没有为 {platform} 配置更新文件名")),
    }
}

fn no_update_info(current_version: &str) -> UpdateInfo {
    UpdateInfo {
        available: false,
        current_version: current_version.to_string(),
        version: current_version.to_string(),
        title: "当前已是最新版本".into(),
        notes: String::new(),
        source: String::new(),
        asset_name: String::new(),
        asset_size: 0,
        mirror_available: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ManifestAsset, ReleaseAsset, SourceUpdate, check, same_asset, validate_manifest_asset,
    };
    use semver::Version;

    #[test]
    fn accepts_matching_manifest_asset() {
        let release = ReleaseAsset {
            name: "Z_COM-v0.2.0-windows-x86_64.exe".into(),
            size: Some(42),
            browser_download_url: "https://example.invalid/file".into(),
        };
        let manifest = ManifestAsset {
            name: release.name.clone(),
            size: 42,
            sha256: "A".repeat(64),
        };
        assert!(validate_manifest_asset("test", &release.name, &release, &manifest).is_ok());
    }

    #[test]
    fn rejects_invalid_manifest_hash() {
        let release = ReleaseAsset {
            name: "file".into(),
            size: None,
            browser_download_url: String::new(),
        };
        let manifest = ManifestAsset {
            name: "file".into(),
            size: 1,
            sha256: "invalid".into(),
        };
        assert!(validate_manifest_asset("test", "file", &release, &manifest).is_err());
    }

    #[test]
    fn mirror_requires_identical_asset_metadata() {
        let candidate = SourceUpdate {
            source: "GitHub".into(),
            version: Version::new(0, 2, 0),
            title: String::new(),
            notes: String::new(),
            asset_name: "file".into(),
            asset_size: 42,
            sha256: "A".repeat(64),
            download_url: String::new(),
        };
        let mut mirror = candidate.clone();
        mirror.source = "Gitee".into();
        assert!(same_asset(&candidate, &mirror));
        mirror.asset_size = 43;
        assert!(!same_asset(&candidate, &mirror));
    }

    #[test]
    #[ignore = "需要访问 GitHub 和 Gitee Release API"]
    fn checks_live_release_sources() {
        let (info, candidate) = check("0.0.0").expect("公开 Release 应可检查");
        assert!(info.available);
        assert!(candidate.is_some());
    }
}
