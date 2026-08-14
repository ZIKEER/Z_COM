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
    pub(crate) source_warning: String,
    pub(crate) latest_confirmed: bool,
    pub(crate) source_results: Vec<UpdateSourceResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateSourceResult {
    pub(crate) source: String,
    pub(crate) version: String,
    pub(crate) state: String,
    pub(crate) error: String,
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
        .unwrap_or_else(|_| Err("GitHub 更新检查线程异常退出".to_string()));
    let gitee = gitee
        .join()
        .unwrap_or_else(|_| Err("Gitee 更新检查线程异常退出".to_string()));
    let source_results = vec![
        source_result("GitHub", &github, &current),
        source_result("Gitee", &gitee, &current),
    ];

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
    resolve_candidates(
        current_version,
        &current,
        candidates,
        failures,
        source_results,
    )
}

fn resolve_candidates(
    current_version: &str,
    current: &Version,
    mut candidates: Vec<SourceUpdate>,
    failures: Vec<String>,
    source_results: Vec<UpdateSourceResult>,
) -> Result<(UpdateInfo, Option<ResolvedUpdate>), String> {
    if candidates.is_empty() {
        return Ok((
            unavailable_info(
                current_version,
                format!("两个更新源均不可用：{}", failures.join("；")),
                source_results,
            ),
            None,
        ));
    }

    candidates.sort_by(|left, right| right.version.cmp(&left.version));
    let latest_version = candidates[0].version.clone();
    if latest_version <= *current {
        if failures.is_empty() {
            return Ok((no_update_info(current_version, source_results), None));
        }
        return Ok((
            unavailable_info(
                current_version,
                format!(
                    "无法确认是否为最新版本：{}；可用更新源最高版本为 {}",
                    failures.join("；"),
                    latest_version
                ),
                source_results,
            ),
            None,
        ));
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
        source_warning: if failures.is_empty() {
            String::new()
        } else {
            format!("部分更新源不可用：{}", failures.join("；"))
        },
        latest_confirmed: failures.is_empty(),
        source_results,
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
    let expected_name = asset_name()?;
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

fn source_result(
    source: &str,
    result: &Result<SourceUpdate, String>,
    current: &Version,
) -> UpdateSourceResult {
    match result {
        Ok(candidate) => UpdateSourceResult {
            source: source.into(),
            version: candidate.version.to_string(),
            state: match candidate.version.cmp(current) {
                std::cmp::Ordering::Greater => "newer",
                std::cmp::Ordering::Equal => "current",
                std::cmp::Ordering::Less => "older",
            }
            .into(),
            error: String::new(),
        },
        Err(error) => UpdateSourceResult {
            source: source.into(),
            version: String::new(),
            state: "failed".into(),
            error: error.clone(),
        },
    }
}

fn platform_key() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok("windows-x86_64"),
        ("linux", "x86_64") => Ok("linux-x86_64"),
        (os, arch) => Err(format!("当前平台暂不支持自动更新: {os}-{arch}")),
    }
}

fn asset_name() -> Result<String, String> {
    match platform_key()? {
        "windows-x86_64" => Ok("Z_COM-windows-x86_64.exe".into()),
        "linux-x86_64" => Ok("Z_COM-linux-x86_64".into()),
        platform => Err(format!("没有为 {platform} 配置更新文件名")),
    }
}

fn no_update_info(current_version: &str, source_results: Vec<UpdateSourceResult>) -> UpdateInfo {
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
        source_warning: String::new(),
        latest_confirmed: true,
        source_results,
    }
}

fn unavailable_info(
    current_version: &str,
    warning: String,
    source_results: Vec<UpdateSourceResult>,
) -> UpdateInfo {
    let mut info = no_update_info(current_version, source_results);
    info.title = "无法确认最新版本".into();
    info.source_warning = warning;
    info.latest_confirmed = false;
    info
}

#[cfg(test)]
mod tests {
    use super::{
        ManifestAsset, ReleaseAsset, SourceUpdate, UpdateSourceResult, asset_name, check,
        resolve_candidates, same_asset, validate_manifest_asset,
    };
    use semver::Version;

    #[test]
    fn accepts_matching_manifest_asset() {
        let release = ReleaseAsset {
            name: asset_name().expect("当前测试平台应受支持"),
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
        let candidate = source_update("GitHub", Version::new(0, 2, 0));
        let mut mirror = candidate.clone();
        mirror.source = "Gitee".into();
        assert!(same_asset(&candidate, &mirror));
        mirror.asset_size = 43;
        assert!(!same_asset(&candidate, &mirror));
    }

    #[test]
    fn stable_asset_name_does_not_include_version() {
        let name = asset_name().expect("当前测试平台应受支持");
        assert!(!name.contains("v0."));
        assert!(name.starts_with("Z_COM-"));
    }

    #[test]
    fn does_not_claim_latest_when_a_source_failed() {
        let result = resolve_candidates(
            "0.1.5",
            &Version::new(0, 1, 5),
            vec![source_update("Gitee", Version::new(0, 1, 3))],
            vec!["GitHub Release 查询失败".into()],
            vec![source_status("GitHub", "", "failed")],
        );
        let (info, candidate) = result.expect("部分源失败应返回可展示的检测结果");
        assert!(!info.available);
        assert!(!info.latest_confirmed);
        assert!(info.source_warning.contains("无法确认是否为最新版本"));
        assert!(candidate.is_none());
    }

    #[test]
    fn accepts_newer_version_from_one_available_source() {
        let (info, candidate) = resolve_candidates(
            "0.1.5",
            &Version::new(0, 1, 5),
            vec![source_update("GitHub", Version::new(0, 1, 6))],
            vec!["Gitee Release 查询失败".into()],
            vec![
                source_status("GitHub", "0.1.6", "newer"),
                source_status("Gitee", "", "failed"),
            ],
        )
        .expect("单个可用源发现新版本时应允许更新");
        assert!(info.available);
        assert_eq!(info.version, "0.1.6");
        assert!(info.source_warning.contains("Gitee"));
        assert!(candidate.is_some());
    }

    #[test]
    fn returns_both_results_when_all_sources_fail() {
        let (info, candidate) = resolve_candidates(
            "0.1.6",
            &Version::new(0, 1, 6),
            Vec::new(),
            vec!["GitHub 检测失败".into(), "Gitee 检测失败".into()],
            vec![
                source_status("GitHub", "", "failed"),
                source_status("Gitee", "", "failed"),
            ],
        )
        .expect("双源失败也应返回可展示的检测结果");
        assert!(!info.latest_confirmed);
        assert_eq!(info.source_results.len(), 2);
        assert!(candidate.is_none());
    }

    #[test]
    fn confirms_latest_only_when_all_sources_succeed() {
        let (info, candidate) = resolve_candidates(
            "0.1.6",
            &Version::new(0, 1, 6),
            vec![
                source_update("GitHub", Version::new(0, 1, 5)),
                source_update("Gitee", Version::new(0, 1, 5)),
            ],
            Vec::new(),
            vec![
                source_status("GitHub", "0.1.5", "older"),
                source_status("Gitee", "0.1.5", "older"),
            ],
        )
        .expect("双源成功且均无新版本时应确认当前最新");
        assert!(info.latest_confirmed);
        assert!(!info.available);
        assert!(candidate.is_none());
    }

    fn source_update(source: &str, version: Version) -> SourceUpdate {
        SourceUpdate {
            source: source.into(),
            version,
            title: String::new(),
            notes: String::new(),
            asset_name: asset_name().expect("当前测试平台应受支持"),
            asset_size: 42,
            sha256: "A".repeat(64),
            download_url: String::new(),
        }
    }

    fn source_status(source: &str, version: &str, state: &str) -> UpdateSourceResult {
        UpdateSourceResult {
            source: source.into(),
            version: version.into(),
            state: state.into(),
            error: if state == "failed" {
                format!("{source} 检测失败")
            } else {
                String::new()
            },
        }
    }

    #[test]
    #[ignore = "需要访问 GitHub 和 Gitee Release API"]
    fn checks_live_release_sources() {
        let (info, candidate) = check("0.0.0").expect("公开 Release 应可检查");
        assert!(info.available);
        assert!(candidate.is_some());
    }
}
