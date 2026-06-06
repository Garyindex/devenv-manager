use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc, Arc, OnceLock,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};
use tauri::{AppHandle, Emitter};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const APP_GITHUB_URL: &str = "https://github.com/GaryIndex/devenv-manager";
const APP_RELEASES_URL: &str = "https://github.com/GaryIndex/devenv-manager/releases";
const APP_RELEASE_API_URL: &str =
    "https://api.github.com/repos/GaryIndex/devenv-manager/releases/latest";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvEntry {
    id: String,
    name_key: String,
    path: String,
    category: Category,
    risk: Risk,
    size_bytes: u64,
    exists: bool,
    description_key: String,
    modified: Option<String>,
    file_count: u64,
    dir_count: u64,
    size_approximate: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanReport {
    home_dir: String,
    platform: String,
    total_size_bytes: u64,
    entries: Vec<EnvEntry>,
    path_entries: Vec<PathEntry>,
    scan_mode: String,
    elapsed_ms: u128,
    estimated_memory_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PathEntry {
    path: String,
    exists: bool,
    source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SizeEstimate {
    bytes: u64,
    files: u64,
    dirs: u64,
    truncated: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeepScanReport {
    estimate: SizeEstimate,
    children: Vec<EnvEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteResult {
    deleted: bool,
    path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupPreview {
    path: String,
    allowed: bool,
    risk: Risk,
    estimate: SizeEstimate,
    reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuarantineItem {
    id: String,
    original_path: String,
    quarantine_path: String,
    created: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuarantineResult {
    item: QuarantineItem,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditLogEntry {
    id: String,
    action: String,
    target_path: String,
    status: String,
    detail: String,
    created: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DevToolCheck {
    id: String,
    name: String,
    summary: Option<String>,
    description: Option<String>,
    command: String,
    installed: bool,
    version: Option<String>,
    source: String,
    status: String,
    required: bool,
    category: String,
    install_plan: Option<InstallPlan>,
    uninstall_plan: Option<InstallPlan>,
    version_options: Vec<InstallVersionOption>,
    detail: Option<ToolDetailMetadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDetailMetadata {
    aliases: Vec<String>,
    tags: Vec<String>,
    platforms: Vec<String>,
    lifecycle: ToolLifecycleMetadata,
    descriptions: ToolDescriptions,
    usage: ToolUsageMetadata,
    notes: ToolNotesMetadata,
    links: ToolLinks,
    dependencies: ToolDependenciesMetadata,
    risk: ToolRiskMetadata,
    quality: ToolQualityMetadata,
    verify_commands: Vec<ToolVerifyCommand>,
    sources: Vec<ToolSourceDetail>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolLifecycleMetadata {
    status: Option<String>,
    deprecated: bool,
    replaced_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolDescriptions {
    short: Option<String>,
    long: Option<String>,
    source: Option<String>,
    homepage: Option<String>,
    last_updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolUsageMetadata {
    primary_use_cases: Vec<String>,
    keywords: Vec<String>,
    related_tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolNotesMetadata {
    install: Vec<String>,
    upgrade: Vec<String>,
    known_issues: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolLinks {
    homepage: Option<String>,
    download: Option<String>,
    releases: Option<String>,
    docs: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolDependenciesMetadata {
    required: bool,
    required_by_suites: Vec<String>,
    depends_on: Vec<String>,
    optional_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolRiskMetadata {
    requires_admin: bool,
    portable: bool,
    system_critical: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolQualityMetadata {
    confidence: String,
    score: i32,
    official: bool,
    last_successful_scan_at: Option<String>,
    failure_count: u32,
    stale_after_days: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolVerifyCommand {
    command: String,
    args: Vec<String>,
    expected_pattern: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolSourceDetail {
    id: String,
    manager: String,
    package_id: Option<String>,
    official: bool,
    priority: i32,
    platforms: Vec<String>,
    scan: ToolScanMetadata,
    package: ToolPackageMetadata,
    descriptions: ToolDescriptions,
    usage: ToolUsageMetadata,
    notes: ToolNotesMetadata,
    links: ToolLinks,
    quality: ToolQualityMetadata,
    versions: Vec<ToolVersionDetail>,
    downloads: Vec<ToolDownloadDetail>,
    commands: Vec<ToolCommandDetail>,
    verify_commands: Vec<ToolVerifyCommand>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolScanMetadata {
    status: String,
    scanned_at: Option<String>,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToolPackageMetadata {
    name: Option<String>,
    publisher: Option<String>,
    author: Option<String>,
    description: Option<String>,
    license: Option<String>,
    license_url: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolVersionDetail {
    version: String,
    channel: String,
    latest: bool,
    prerelease: bool,
    lts: Option<bool>,
    release_date: Option<String>,
    eol_date: Option<String>,
    changelog_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDownloadDetail {
    id: String,
    version: Option<String>,
    platform: String,
    architecture: Option<String>,
    kind: String,
    url: Option<String>,
    url_type: String,
    direct: bool,
    sha256: Option<String>,
    size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolCommandDetail {
    action: String,
    manager: String,
    platform: String,
    version: Option<String>,
    requires_admin: bool,
    supports_version: bool,
    shell: String,
    command: Vec<String>,
    template: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentSuite {
    id: String,
    name: String,
    description: String,
    tool_ids: Vec<String>,
    required_count: usize,
    installed_count: usize,
    missing_count: usize,
    admin_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallPlan {
    package_manager: String,
    package_id: String,
    command: Vec<String>,
    needs_admin: bool,
    notes: String,
    source_id: String,
    source_quality: i32,
    download_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallVersionOption {
    id: String,
    label: String,
    package_id: String,
    command: Vec<String>,
    needs_admin: bool,
    source_id: String,
    manager: String,
    quality_score: i32,
    download_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentCheckReport {
    platform: String,
    package_manager_available: bool,
    package_manager_version: Option<String>,
    package_manager_checked: bool,
    required_missing: usize,
    optional_missing: usize,
    checked_tools: usize,
    pending_tools: usize,
    total_tools: usize,
    completed: bool,
    tools: Vec<DevToolCheck>,
    suites: Vec<EnvironmentSuite>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentCheckProgress {
    report: EnvironmentCheckReport,
    checked: usize,
    total: usize,
    completed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentToolDetailRequest {
    tool_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallRequest {
    tool_id: String,
    package_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    current_version: String,
    github_url: String,
    releases_url: String,
    official_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppUpdateInfo {
    current_version: String,
    latest_version: Option<String>,
    update_available: bool,
    release_url: String,
    download_url: Option<String>,
    asset_name: Option<String>,
    published_at: Option<String>,
    body: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    body: Option<String>,
    #[serde(default)]
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallResult {
    tool_id: String,
    status: String,
    exit_code: Option<i32>,
    output: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteInstallRequest {
    suite_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteInstallResult {
    suite_id: String,
    results: Vec<InstallResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanRequest {
    mode: ScanMode,
    drive: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum ScanMode {
    Developer,
    Drive,
    Full,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    entry: EnvEntry,
    scanned: usize,
    total: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolInsights {
    cleanable_count: usize,
    toolchain_count: usize,
    ai_tool_count: usize,
    broken_path_count: usize,
    config_rule_count: usize,
    dependency_edge_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Category {
    Runtime,
    Cache,
    Config,
    Temp,
    Toolchain,
    PackageManager,
    AiTool,
    Ide,
    Container,
    Mobile,
    BuildTool,
    Drive,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Risk {
    Keep,
    Caution,
    Cleanable,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleFile {
    risk_model: RiskModel,
    rules: Vec<RuleSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RiskModel {
    default: Risk,
    factors: std::collections::HashMap<String, i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleSpec {
    id: String,
    name_key: String,
    category: Category,
    path_by_platform: std::collections::HashMap<String, String>,
    risk_factors: Vec<String>,
    depends_on: Vec<String>,
}

#[tauri::command]
fn list_drives() -> Result<Vec<String>, String> {
    Ok(available_roots()
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
fn scan_environment(app: AppHandle, request: Option<ScanRequest>) -> Result<ScanReport, String> {
    let started = Instant::now();
    let home = home_dir().ok_or_else(|| "Unable to resolve home directory".to_string())?;
    let platform = env::consts::OS.to_string();
    let request = request.unwrap_or(ScanRequest {
        mode: ScanMode::Developer,
        drive: None,
    });
    let specs = scan_specs(&home, &request)?;
    let total = specs.len();
    let mut entries = Vec::with_capacity(total);

    for (index, spec) in specs.into_iter().enumerate() {
        let entry = scan_spec_fast(spec);
        let _ = app.emit(
            "scan-progress",
            ScanProgress {
                entry: entry.clone(),
                scanned: index + 1,
                total,
            },
        );
        entries.push(entry);
    }

    let total_size_bytes = entries.iter().map(|entry| entry.size_bytes).sum();
    let estimated_memory_bytes = estimate_report_memory(&entries);
    Ok(ScanReport {
        home_dir: home.to_string_lossy().to_string(),
        platform,
        total_size_bytes,
        entries,
        path_entries: path_entries(),
        scan_mode: match request.mode {
            ScanMode::Developer => "developer",
            ScanMode::Drive => "drive",
            ScanMode::Full => "full",
        }
        .to_string(),
        elapsed_ms: started.elapsed().as_millis(),
        estimated_memory_bytes,
    })
}

#[tauri::command]
fn scan_entry_size(path: String) -> Result<SizeEstimate, String> {
    let target = PathBuf::from(path);
    if !target.exists() {
        return Err("Path does not exist".to_string());
    }
    Ok(dir_size_limited(&target, deep_limits()))
}

#[tauri::command]
fn tool_insights(report: ScanReport) -> Result<ToolInsights, String> {
    Ok(ToolInsights {
        cleanable_count: report
            .entries
            .iter()
            .filter(|entry| entry.exists && matches!(entry.risk, Risk::Cleanable))
            .count(),
        toolchain_count: report
            .entries
            .iter()
            .filter(|entry| entry.exists && matches!(entry.category, Category::Toolchain))
            .count(),
        ai_tool_count: report
            .entries
            .iter()
            .filter(|entry| entry.exists && matches!(entry.category, Category::AiTool))
            .count(),
        broken_path_count: report
            .path_entries
            .iter()
            .filter(|entry| !entry.exists)
            .count(),
        config_rule_count: load_rule_file()?.rules.len(),
        dependency_edge_count: load_rule_file()?
            .rules
            .iter()
            .map(|rule| rule.depends_on.len())
            .sum(),
    })
}

#[tauri::command]
fn deep_scan_entry(path: String) -> Result<DeepScanReport, String> {
    let target = PathBuf::from(path);
    if !target.exists() {
        return Err("Path does not exist".to_string());
    }

    let estimate = dir_size_limited(&target, deep_limits());
    let mut children = Vec::new();
    if target.is_dir() {
        let read_dir = fs::read_dir(&target).map_err(|error| error.to_string())?;
        for (index, item) in read_dir.flatten().take(80).enumerate() {
            let child_path = item.path();
            let child_name = item.file_name().to_string_lossy().to_string();
            let child_estimate = if child_path.is_dir() {
                dir_size_limited(&child_path, child_limits())
            } else {
                file_estimate(&child_path)
            };
            children.push(EnvEntry {
                id: format!("deep_child_{index}_{}", sanitize_id(&child_name)),
                name_key: format!("literal:{child_name}"),
                path: child_path.to_string_lossy().to_string(),
                category: Category::Cache,
                risk: Risk::Caution,
                size_bytes: child_estimate.bytes,
                exists: child_path.exists(),
                description_key: "desc.deepChild".to_string(),
                modified: modified_time(&child_path),
                file_count: child_estimate.files,
                dir_count: child_estimate.dirs,
                size_approximate: child_estimate.truncated,
            });
        }
        children.sort_by_key(|child| std::cmp::Reverse(child.size_bytes));
    }

    Ok(DeepScanReport { estimate, children })
}

#[tauri::command]
fn delete_entry(path: String, risk: Risk) -> Result<DeleteResult, String> {
    if !matches!(risk, Risk::Cleanable) {
        return Err("Only cleanable entries can be deleted from this build".to_string());
    }
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Ok(DeleteResult {
            deleted: false,
            path,
        });
    }
    if target.is_dir() {
        fs::remove_dir_all(&target).map_err(|error| error.to_string())?;
    } else {
        fs::remove_file(&target).map_err(|error| error.to_string())?;
    }
    append_audit_log(
        "delete-entry",
        &path,
        "success",
        "Permanently deleted a cleanable entry",
    )?;
    Ok(DeleteResult {
        deleted: true,
        path,
    })
}

#[tauri::command]
fn preview_cleanup(path: String, risk: Risk) -> Result<CleanupPreview, String> {
    let target = PathBuf::from(&path);
    let allowed = matches!(risk, Risk::Cleanable) && target.exists();
    let estimate = if target.exists() {
        dir_size_limited(&target, child_limits())
    } else {
        SizeEstimate {
            bytes: 0,
            files: 0,
            dirs: 0,
            truncated: false,
        }
    };
    Ok(CleanupPreview {
        path,
        allowed,
        risk,
        estimate,
        reason: if allowed {
            "cleanable rule".to_string()
        } else {
            "not cleanable or missing".to_string()
        },
    })
}

#[tauri::command]
fn quarantine_entry(path: String, risk: Risk) -> Result<QuarantineResult, String> {
    if !matches!(risk, Risk::Cleanable) {
        return Err("Only cleanable entries can be quarantined".to_string());
    }
    let source = PathBuf::from(&path);
    if !source.exists() {
        return Err("Path does not exist".to_string());
    }
    let quarantine_root = quarantine_dir()?;
    fs::create_dir_all(&quarantine_root).map_err(|error| error.to_string())?;
    let id = format!(
        "{}_{}",
        current_epoch_secs(),
        sanitize_id(
            source
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("item")
        )
    );
    let target = quarantine_root.join(&id);
    fs::rename(&source, &target).map_err(|error| error.to_string())?;
    let item = QuarantineItem {
        id,
        original_path: path,
        quarantine_path: target.to_string_lossy().to_string(),
        created: current_epoch_secs().to_string(),
    };
    write_quarantine_manifest(&item)?;
    append_audit_log(
        "quarantine",
        &item.original_path,
        "success",
        &format!("Moved to {}", item.quarantine_path),
    )?;
    Ok(QuarantineResult { item })
}

#[tauri::command]
fn list_quarantine() -> Result<Vec<QuarantineItem>, String> {
    let manifest = quarantine_manifest();
    if !manifest.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(manifest).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

#[tauri::command]
fn restore_quarantine(id: String) -> Result<QuarantineItem, String> {
    let mut items = list_quarantine()?;
    let index = items
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| "Quarantine item not found".to_string())?;
    let item = items.remove(index);
    let source = PathBuf::from(&item.quarantine_path);
    let target = PathBuf::from(&item.original_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::rename(&source, &target).map_err(|error| error.to_string())?;
    fs::write(
        quarantine_manifest(),
        serde_json::to_string_pretty(&items).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    append_audit_log(
        "restore-quarantine",
        &item.original_path,
        "success",
        &format!("Restored from {}", item.quarantine_path),
    )?;
    Ok(item)
}

#[tauri::command]
fn delete_quarantine(id: String) -> Result<QuarantineItem, String> {
    let mut items = list_quarantine()?;
    let index = items
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| "Quarantine item not found".to_string())?;
    let item = items.remove(index);
    let quarantine_root = quarantine_dir()?;
    let target = PathBuf::from(&item.quarantine_path);
    let canonical_root = quarantine_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let canonical_target = target.canonicalize().map_err(|error| error.to_string())?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err("Refusing to delete a file outside quarantine".to_string());
    }
    if canonical_target.is_dir() {
        fs::remove_dir_all(&canonical_target).map_err(|error| error.to_string())?;
    } else {
        fs::remove_file(&canonical_target).map_err(|error| error.to_string())?;
    }
    fs::write(
        quarantine_manifest(),
        serde_json::to_string_pretty(&items).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    append_audit_log(
        "delete-quarantine",
        &item.original_path,
        "success",
        "Permanently deleted from quarantine",
    )?;
    Ok(item)
}

#[tauri::command]
fn list_audit_log() -> Result<Vec<AuditLogEntry>, String> {
    read_audit_log()
}

#[tauri::command]
fn app_metadata() -> AppMetadata {
    AppMetadata {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        github_url: APP_GITHUB_URL.to_string(),
        releases_url: APP_RELEASES_URL.to_string(),
        official_url: APP_GITHUB_URL.to_string(),
    }
}

#[tauri::command]
fn check_app_update() -> Result<AppUpdateInfo, String> {
    Ok(fetch_app_update_info())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    open_url(&url)
}

#[tauri::command]
fn check_development_environment() -> Result<EnvironmentCheckReport, String> {
    let data = environment_data()?;
    build_environment_report(data)
}

#[tauri::command]
fn get_environment_tool_detail(
    request: EnvironmentToolDetailRequest,
) -> Result<DevToolCheck, String> {
    build_environment_tool_detail(&request.tool_id)
}

#[tauri::command]
fn start_development_environment_check(app: AppHandle) -> Result<EnvironmentCheckReport, String> {
    let data = environment_data()?;
    let initial = build_pending_environment_report(data, None, false);
    let handle = app.clone();
    thread::spawn(move || {
        let mut report = build_pending_environment_report(data, None, false);
        let resolver = CommandResolver::new();
        let manager_versions = detect_install_managers(&resolver, data);
        report.package_manager_available = !manager_versions.is_empty();
        report.package_manager_version = package_manager_status_text(&manager_versions);
        report.package_manager_checked = true;
        report = rebuild_environment_report_totals(report, data);
        emit_environment_progress(&handle, report.clone(), false);

        let platform_tools = platform_catalog_tools(data);
        let total = platform_tools.len();
        let (tx, rx) = mpsc::channel();
        let cursor = Arc::new(AtomicUsize::new(0));
        let worker_count = environment_scan_worker_count(total);
        for _ in 0..worker_count {
            let tx = tx.clone();
            let cursor = Arc::clone(&cursor);
            let manager_versions = manager_versions.clone();
            let specs = platform_tools.clone();
            thread::spawn(move || {
                let resolver = CommandResolver::new();
                loop {
                    let index = cursor.fetch_add(1, Ordering::SeqCst);
                    let Some(spec) = specs.get(index) else {
                        break;
                    };
                    let checked =
                        build_tool_check_with_resolver(spec, data, &manager_versions, &resolver);
                    if tx.send(checked).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);

        for (index, checked) in rx.iter().enumerate() {
            if let Some(tool) = report.tools.iter_mut().find(|tool| tool.id == checked.id) {
                *tool = checked;
            }
            report.checked_tools = index + 1;
            report.pending_tools = total.saturating_sub(index + 1);
            report.completed = report.pending_tools == 0;
            report = rebuild_environment_report_totals(report, data);
            emit_environment_progress(&handle, report.clone(), report.completed);
        }
    });
    Ok(initial)
}

#[tauri::command]
async fn open_environment_window(app: AppHandle) -> Result<(), String> {
    open_environment_window_impl(&app)
}

#[tauri::command]
async fn open_environment_tool_detail_window(
    app: AppHandle,
    request: EnvironmentToolDetailRequest,
) -> Result<(), String> {
    open_environment_tool_detail_window_impl(&app, &request.tool_id)
}

fn open_environment_window_impl(app: &AppHandle) -> Result<(), String> {
    const LABEL: &str = "environment-check";
    if let Some(window) = app.get_webview_window(LABEL) {
        window.unminimize().map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .title("DevEnv Manager - Environment Check")
        .inner_size(980.0, 760.0)
        .min_inner_size(720.0, 560.0)
        .focused(true)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn open_environment_tool_detail_window_impl(app: &AppHandle, tool_id: &str) -> Result<(), String> {
    let label = format!("environment-tool-{}", sanitize_window_label(tool_id));
    if let Some(window) = app.get_webview_window(&label) {
        window.unminimize().map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, &label, WebviewUrl::default())
        .title("DevEnv Manager - Tool Detail")
        .inner_size(760.0, 680.0)
        .min_inner_size(520.0, 520.0)
        .focused(true)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn install_missing_tool(request: InstallRequest) -> Result<InstallResult, String> {
    let mut tool = build_environment_tool_detail(&request.tool_id)?;
    if let Some(package_id) = request.package_id {
        tool.install_plan = install_plan_for_package(&tool, &package_id)?;
    }
    install_tool_check(tool)
}

#[tauri::command]
fn uninstall_installed_tool(request: InstallRequest) -> Result<InstallResult, String> {
    let tool = build_environment_tool_detail(&request.tool_id)?;
    uninstall_tool_check(tool)
}

#[tauri::command]
fn open_install_terminal(request: InstallRequest) -> Result<(), String> {
    let tool = build_environment_tool_detail(&request.tool_id)?;
    let plan = match request.package_id {
        Some(package_id) => install_plan_for_package(&tool, &package_id)?,
        None => tool.install_plan.clone(),
    }
    .ok_or_else(|| "No install plan is available for this tool".to_string())?;
    open_terminal_with_command(&plan.command)
}

#[tauri::command]
fn open_uninstall_terminal(request: InstallRequest) -> Result<(), String> {
    let tool = build_environment_tool_detail(&request.tool_id)?;
    let plan = tool
        .uninstall_plan
        .ok_or_else(|| "No uninstall plan is available for this tool".to_string())?;
    open_terminal_with_command(&plan.command)
}

#[tauri::command]
fn install_environment_suite(request: SuiteInstallRequest) -> Result<SuiteInstallResult, String> {
    let data = environment_data()?;
    let suite = data
        .catalog
        .suites
        .iter()
        .find(|suite| suite.id == request.suite_id)
        .ok_or_else(|| "Unknown suite".to_string())?;
    let report = build_environment_report(data)?;
    let mut results = Vec::new();
    for tool_id in suite_tool_ids(suite, data.identities) {
        if let Some(tool) = report
            .tools
            .iter()
            .find(|tool| {
                tool.id == tool_id && tool.status == "missing" && tool.install_plan.is_some()
            })
            .cloned()
        {
            results.push(install_tool_check(tool)?);
        }
    }
    Ok(SuiteInstallResult {
        suite_id: request.suite_id,
        results,
    })
}

#[tauri::command]
fn export_html_report(report: ScanReport, locale: String) -> Result<String, String> {
    let desktop = desktop_dir().ok_or_else(|| "Unable to resolve desktop directory".to_string())?;
    let file_name = if locale == "zh-CN" {
        "开发环境管家报告.html"
    } else {
        "devenv-manager-report.html"
    };
    let target = desktop.join(file_name);
    let html = render_html_report(&report, &locale);
    fs::write(&target, html).map_err(|error| error.to_string())?;
    append_audit_log(
        "export-report",
        &target.to_string_lossy(),
        "success",
        "Exported HTML scan report",
    )?;
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(path);
    if !target.exists() {
        return Err("Path does not exist".to_string());
    }
    let open_target = if target.is_dir() {
        target
    } else {
        target
            .parent()
            .ok_or_else(|| "Unable to resolve parent directory".to_string())?
            .to_path_buf()
    };

    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer.exe");
        command.arg(&open_target);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(&open_target);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(&open_target);
        command
    };

    command.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            app.add_capability(
                r#"{
                    "identifier": "environment-check-core",
                    "description": "Allow the environment check window to use shared Tauri core APIs.",
                    "windows": ["environment-check", "environment-tool-*"],
                    "permissions": ["core:default"]
                }"#,
            )?;
            if env::var_os("DEVENV_MANAGER_OPEN_ENVIRONMENT").is_some() {
                let handle = app.handle().clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(800));
                    let _ = open_environment_window_impl(&handle);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_drives,
            scan_environment,
            scan_entry_size,
            tool_insights,
            deep_scan_entry,
            delete_entry,
            preview_cleanup,
            quarantine_entry,
            list_quarantine,
            restore_quarantine,
            delete_quarantine,
            list_audit_log,
            app_metadata,
            check_app_update,
            open_external_url,
            check_development_environment,
            get_environment_tool_detail,
            start_development_environment_check,
            open_environment_window,
            open_environment_tool_detail_window,
            install_missing_tool,
            uninstall_installed_tool,
            open_install_terminal,
            open_uninstall_terminal,
            install_environment_suite,
            export_html_report,
            open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct EnvSpec {
    id: String,
    name_key: String,
    path: PathBuf,
    category: Category,
    risk: Risk,
    description_key: String,
}

#[derive(Clone, Copy)]
struct ScanLimits {
    max_entries: u64,
    max_duration: Duration,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolCommandSpec {
    command: String,
    version_arg: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogToolFile {
    tools: Vec<CatalogToolSpec>,
    suites: Vec<CatalogSuiteSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogToolSpec {
    id: String,
    name: String,
    category_id: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    platforms: Vec<String>,
    detection: ToolDetectionSpec,
    requirements: ToolRequirementSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolDetectionSpec {
    #[serde(default)]
    commands: Vec<ToolCommandSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolRequirementSpec {
    #[serde(default)]
    required: bool,
    #[serde(default)]
    requires_admin: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogSuiteSpec {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tool_ids: Vec<String>,
    #[serde(default)]
    required_tool_ids: Vec<String>,
    #[serde(default)]
    optional_tool_ids: Vec<String>,
}

#[derive(Clone, Copy)]
struct EnvironmentData {
    catalog: &'static CatalogToolFile,
    metadata: &'static OnlineMetadataFile,
    policy: &'static SourcePolicyFile,
    identities: &'static IdentityFile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineMetadataFile {
    tools: Vec<OnlineToolSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineToolSpec {
    id: String,
    name: String,
    category_id: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    platforms: Vec<String>,
    #[serde(default)]
    lifecycle: ToolLifecycleMetadata,
    #[serde(default)]
    links: ToolLinks,
    #[serde(default)]
    descriptions: ToolDescriptions,
    #[serde(default)]
    usage: ToolUsageMetadata,
    #[serde(default)]
    notes: ToolNotesMetadata,
    detection: ToolDetectionSpec,
    #[serde(default)]
    verify: OnlineVerifySpec,
    #[serde(default)]
    dependencies: ToolDependenciesMetadata,
    #[serde(default)]
    risk: ToolRiskMetadata,
    quality: OnlineQualitySpec,
    #[serde(default)]
    sources: Vec<OnlineSourceSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineSourceSpec {
    id: String,
    manager: String,
    package_id: Option<String>,
    #[serde(default)]
    platforms: Vec<String>,
    #[serde(default)]
    official: bool,
    #[serde(default)]
    priority: i32,
    scan: OnlineScanSpec,
    #[serde(default)]
    package: ToolPackageMetadata,
    #[serde(default)]
    links: ToolLinks,
    #[serde(default)]
    descriptions: ToolDescriptions,
    #[serde(default)]
    usage: ToolUsageMetadata,
    #[serde(default)]
    notes: ToolNotesMetadata,
    #[serde(default)]
    versions: Vec<OnlineVersionSpec>,
    #[serde(default)]
    downloads: Vec<OnlineDownloadSpec>,
    #[serde(default)]
    commands: Vec<OnlineCommandSpec>,
    quality: OnlineQualitySpec,
    #[serde(default)]
    verify: OnlineVerifySpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineScanSpec {
    status: String,
    #[serde(default)]
    scanned_at: Option<String>,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineQualitySpec {
    confidence: String,
    score: i32,
    #[serde(default)]
    official: bool,
    #[serde(default)]
    last_successful_scan_at: Option<String>,
    #[serde(default)]
    failure_count: u32,
    #[serde(default)]
    stale_after_days: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineVersionSpec {
    version: String,
    #[serde(default)]
    channel: String,
    #[serde(default)]
    latest: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    lts: Option<bool>,
    #[serde(default)]
    release_date: Option<String>,
    #[serde(default)]
    eol_date: Option<String>,
    #[serde(default)]
    changelog_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineDownloadSpec {
    #[serde(default)]
    id: String,
    #[serde(default)]
    version: Option<String>,
    platform: String,
    #[serde(default)]
    architecture: Option<String>,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    url_type: String,
    #[serde(default)]
    direct: bool,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineCommandSpec {
    action: String,
    manager: String,
    platform: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    requires_admin: bool,
    #[serde(default)]
    supports_version: bool,
    shell: String,
    command: Vec<String>,
    template: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct OnlineVerifySpec {
    #[serde(default)]
    commands: Vec<ToolVerifyCommand>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourcePolicyFile {
    platforms: HashMap<String, PlatformSourcePolicy>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformSourcePolicy {
    #[serde(default)]
    manager_priority: Vec<String>,
    #[serde(default = "default_minimum_quality_score")]
    minimum_quality_score: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityFile {
    identities: Vec<ToolIdentitySpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolIdentitySpec {
    stable_id: String,
    current_id: String,
    current_name: String,
}

fn deep_limits() -> ScanLimits {
    ScanLimits {
        max_entries: 600_000,
        max_duration: Duration::from_secs(10),
    }
}

fn child_limits() -> ScanLimits {
    ScanLimits {
        max_entries: 12_000,
        max_duration: Duration::from_millis(420),
    }
}

fn startup_size_limits() -> ScanLimits {
    ScanLimits {
        max_entries: 900,
        max_duration: Duration::from_millis(18),
    }
}

fn scan_specs(home: &Path, request: &ScanRequest) -> Result<Vec<EnvSpec>, String> {
    let specs = environment_specs(home)?;
    match request.mode {
        ScanMode::Developer => Ok(specs),
        ScanMode::Drive => {
            let drive = request
                .drive
                .as_ref()
                .ok_or_else(|| "Drive is required".to_string())?;
            let root = normalize_root(drive)?;
            Ok(filter_specs_by_roots(specs, &[root]))
        }
        ScanMode::Full => Ok(filter_specs_by_roots(specs, &available_roots())),
    }
}

fn filter_specs_by_roots(specs: Vec<EnvSpec>, roots: &[PathBuf]) -> Vec<EnvSpec> {
    specs
        .into_iter()
        .filter(|spec| {
            roots
                .iter()
                .any(|root| path_is_under_root(&spec.path, root))
        })
        .collect()
}

fn path_is_under_root(path: &Path, root: &Path) -> bool {
    if cfg!(target_os = "windows") {
        let path = path.to_string_lossy().to_ascii_lowercase();
        let mut root = root.to_string_lossy().to_ascii_lowercase();
        if !root.ends_with('\\') && !root.ends_with('/') {
            root.push('\\');
        }
        path == root.trim_end_matches(['\\', '/']) || path.starts_with(&root)
    } else {
        path.starts_with(root)
    }
}

fn load_rule_file() -> Result<RuleFile, String> {
    serde_json::from_str(include_str!("../scan-rules.json")).map_err(|error| error.to_string())
}

fn rule_path_for_current_os(rule: &RuleSpec) -> Option<&str> {
    rule.path_by_platform
        .get(env::consts::OS)
        .or_else(|| rule.path_by_platform.get("all"))
        .map(String::as_str)
}

fn calculate_risk(rule: &RuleSpec, model: &RiskModel) -> Risk {
    let score = rule
        .risk_factors
        .iter()
        .filter_map(|factor| model.factors.get(factor))
        .fold(50_i32, |score, value| score + value);
    if score >= 75 {
        Risk::Cleanable
    } else if score <= 20 {
        Risk::Keep
    } else {
        model.default.clone()
    }
}

fn expand_rule_path(template: &str, home: &Path) -> PathBuf {
    let local = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Local"));
    let roaming = env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Roaming"));
    let program_data = env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let mut value = template
        .replace("{home}", &home.to_string_lossy())
        .replace("{local}", &local.to_string_lossy())
        .replace("{roaming}", &roaming.to_string_lossy())
        .replace("{programData}", &program_data.to_string_lossy())
        .replace("{temp}", &env::temp_dir().to_string_lossy());
    if cfg!(target_os = "windows") {
        value = value.replace('/', "\\");
    }
    PathBuf::from(value)
}

fn scan_spec_fast(spec: EnvSpec) -> EnvEntry {
    let exists = spec.path.exists();
    let (estimate, modified) = if exists {
        if should_estimate_on_startup(&spec) {
            (
                dir_size_limited(&spec.path, startup_size_limits()),
                modified_time(&spec.path),
            )
        } else {
            (file_estimate(&spec.path), modified_time(&spec.path))
        }
    } else {
        (
            SizeEstimate {
                bytes: 0,
                files: 0,
                dirs: 0,
                truncated: false,
            },
            None,
        )
    };

    EnvEntry {
        id: spec.id,
        name_key: spec.name_key,
        path: spec.path.to_string_lossy().to_string(),
        category: spec.category,
        risk: spec.risk,
        size_bytes: estimate.bytes,
        exists,
        description_key: spec.description_key,
        modified,
        file_count: estimate.files,
        dir_count: estimate.dirs,
        size_approximate: estimate.truncated,
    }
}

fn should_estimate_on_startup(spec: &EnvSpec) -> bool {
    matches!(spec.risk, Risk::Cleanable)
        || matches!(
            spec.category,
            Category::Cache | Category::Temp | Category::PackageManager
        )
}

fn file_estimate(path: &Path) -> SizeEstimate {
    let metadata = fs::symlink_metadata(path).ok();
    if metadata.as_ref().is_some_and(|item| item.is_file()) {
        return SizeEstimate {
            bytes: metadata.map(|item| item.len()).unwrap_or(0),
            files: 1,
            dirs: 0,
            truncated: false,
        };
    }
    SizeEstimate {
        bytes: 0,
        files: 0,
        dirs: u64::from(path.is_dir()),
        truncated: path.is_dir(),
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn sanitize_window_label(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn estimate_report_memory(entries: &[EnvEntry]) -> u64 {
    entries
        .iter()
        .map(|entry| {
            128_u64
                + entry.id.len() as u64
                + entry.name_key.len() as u64
                + entry.path.len() as u64
                + entry.description_key.len() as u64
                + entry
                    .modified
                    .as_ref()
                    .map(|value| value.len() as u64)
                    .unwrap_or(0)
        })
        .sum()
}

fn environment_specs(home: &Path) -> Result<Vec<EnvSpec>, String> {
    let rule_file = load_rule_file()?;
    Ok(rule_file
        .rules
        .into_iter()
        .filter_map(|rule| {
            let path = rule_path_for_current_os(&rule)?.to_string();
            let risk = calculate_risk(&rule, &rule_file.risk_model);
            let description_key = format!("desc.{}", rule.id);
            spec_owned(
                rule.id,
                rule.name_key,
                expand_rule_path(&path, home),
                rule.category,
                risk,
                description_key,
            )
            .into()
        })
        .collect())
}

fn load_catalog_tool_file() -> Result<CatalogToolFile, String> {
    serde_json::from_str(include_str!("../catalog-tools.json")).map_err(|error| error.to_string())
}

fn load_online_metadata_file() -> Result<OnlineMetadataFile, String> {
    serde_json::from_str(include_str!("../online-split-metadata.json"))
        .map_err(|error| error.to_string())
}

fn load_source_policy_file() -> Result<SourcePolicyFile, String> {
    serde_json::from_str(include_str!("../source-policy.json")).map_err(|error| error.to_string())
}

fn load_identity_file() -> Result<IdentityFile, String> {
    serde_json::from_str(include_str!("../identities.json")).map_err(|error| error.to_string())
}

fn environment_data() -> Result<EnvironmentData, String> {
    static CATALOG: OnceLock<CatalogToolFile> = OnceLock::new();
    static METADATA: OnceLock<OnlineMetadataFile> = OnceLock::new();
    static POLICY: OnceLock<SourcePolicyFile> = OnceLock::new();
    static IDENTITIES: OnceLock<IdentityFile> = OnceLock::new();
    let catalog = CATALOG
        .get_or_init(|| load_catalog_tool_file().expect("embedded catalog tool data should parse"));
    let metadata = METADATA.get_or_init(|| {
        load_online_metadata_file().expect("embedded online metadata should parse")
    });
    let policy = POLICY
        .get_or_init(|| load_source_policy_file().expect("embedded source policy should parse"));
    let identities =
        IDENTITIES.get_or_init(|| load_identity_file().expect("embedded identities should parse"));
    Ok(EnvironmentData {
        catalog,
        metadata,
        policy,
        identities,
    })
}

fn build_pending_environment_report(
    data: EnvironmentData,
    package_manager_version: Option<String>,
    package_manager_checked: bool,
) -> EnvironmentCheckReport {
    let suite_required = suite_required_tool_ids(&data.catalog.suites, data.identities);
    let tools = data
        .catalog
        .tools
        .iter()
        .filter(|tool| platform_matches(&tool.platforms, env::consts::OS))
        .map(|tool| pending_tool_check(tool, data, &suite_required))
        .collect::<Vec<_>>();
    let total_tools = tools.len();
    rebuild_environment_report_totals(
        EnvironmentCheckReport {
            platform: env::consts::OS.to_string(),
            package_manager_available: package_manager_version.is_some(),
            package_manager_version,
            package_manager_checked,
            required_missing: 0,
            optional_missing: 0,
            checked_tools: 0,
            pending_tools: total_tools,
            total_tools,
            completed: false,
            suites: Vec::new(),
            tools,
        },
        data,
    )
}

fn pending_tool_check(
    spec: &CatalogToolSpec,
    data: EnvironmentData,
    suite_required: &HashSet<String>,
) -> DevToolCheck {
    DevToolCheck {
        id: stable_id_for(data.identities, &spec.id),
        name: identity_name_for(data.identities, &spec.id).unwrap_or_else(|| spec.name.clone()),
        summary: spec.summary.clone(),
        description: spec.description.clone(),
        command: spec
            .detection
            .commands
            .first()
            .map(|command| command.command.clone())
            .unwrap_or_else(|| "PATH".to_string()),
        installed: false,
        version: None,
        source: "pending".to_string(),
        status: "pending".to_string(),
        required: tool_is_required(spec, data.identities, suite_required),
        category: spec.category_id.clone(),
        install_plan: None,
        uninstall_plan: None,
        version_options: Vec::new(),
        detail: None,
    }
}

fn build_environment_report(data: EnvironmentData) -> Result<EnvironmentCheckReport, String> {
    let resolver = CommandResolver::new();
    let manager_versions = detect_install_managers(&resolver, data);
    let package_manager_available = !manager_versions.is_empty();
    let mut tools = platform_catalog_tools(data)
        .into_iter()
        .map(|spec| build_tool_check_with_resolver(spec, data, &manager_versions, &resolver))
        .collect::<Vec<_>>();
    tools.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(rebuild_environment_report_totals(
        EnvironmentCheckReport {
            platform: env::consts::OS.to_string(),
            package_manager_available,
            package_manager_version: package_manager_status_text(&manager_versions),
            package_manager_checked: true,
            required_missing: 0,
            optional_missing: 0,
            checked_tools: tools.len(),
            pending_tools: 0,
            total_tools: tools.len(),
            completed: true,
            suites: Vec::new(),
            tools,
        },
        data,
    ))
}

fn build_environment_tool_detail(tool_id: &str) -> Result<DevToolCheck, String> {
    let data = environment_data()?;
    let spec = platform_catalog_tools(data)
        .into_iter()
        .find(|spec| stable_id_for(data.identities, &spec.id) == tool_id || spec.id == tool_id)
        .ok_or_else(|| "Unknown tool".to_string())?;
    let resolver = CommandResolver::new();
    let manager_versions = detect_install_managers_for_tool(&resolver, data, spec);
    let mut check = build_tool_check_with_resolver(spec, data, &manager_versions, &resolver);
    check.detail = data
        .metadata
        .tools
        .iter()
        .find(|tool| tool.id == spec.id && platform_matches(&tool.platforms, env::consts::OS))
        .map(build_tool_detail_metadata);
    Ok(check)
}

fn platform_catalog_tools(data: EnvironmentData) -> Vec<&'static CatalogToolSpec> {
    data.catalog
        .tools
        .iter()
        .filter(|spec| platform_matches(&spec.platforms, env::consts::OS))
        .collect()
}

fn environment_scan_worker_count(total: usize) -> usize {
    total.clamp(1, 8)
}

fn rebuild_environment_report_totals(
    mut report: EnvironmentCheckReport,
    data: EnvironmentData,
) -> EnvironmentCheckReport {
    report.required_missing = report
        .tools
        .iter()
        .filter(|tool| tool.required && tool.status == "missing")
        .count();
    report.optional_missing = report
        .tools
        .iter()
        .filter(|tool| !tool.required && tool.status == "missing")
        .count();
    report.checked_tools = report
        .tools
        .iter()
        .filter(|tool| tool.status != "pending")
        .count();
    report.pending_tools = report.tools.len().saturating_sub(report.checked_tools);
    report.total_tools = report.tools.len();
    report.completed = report.pending_tools == 0 && report.package_manager_checked;
    report.suites = build_environment_suites(&data.catalog.suites, &report.tools, data.identities);
    report
}

fn emit_environment_progress(app: &AppHandle, report: EnvironmentCheckReport, completed: bool) {
    let _ = app.emit(
        "environment-check-progress",
        EnvironmentCheckProgress {
            checked: report.checked_tools,
            total: report.total_tools,
            completed,
            report,
        },
    );
}

fn build_tool_check_with_resolver(
    spec: &CatalogToolSpec,
    data: EnvironmentData,
    manager_versions: &HashMap<String, String>,
    resolver: &CommandResolver,
) -> DevToolCheck {
    let mut source = "PATH".to_string();
    let mut version = None;
    for candidate in &spec.detection.commands {
        if let Some(found) = probe_command_with_resolver(
            resolver,
            &candidate.command,
            &candidate.version_arg,
            detection_timeout(),
        ) {
            source = candidate.command.clone();
            version = Some(found);
            break;
        }
    }
    let suite_required = suite_required_tool_ids(&data.catalog.suites, data.identities);
    let metadata = data
        .metadata
        .tools
        .iter()
        .find(|tool| tool.id == spec.id && platform_matches(&tool.platforms, env::consts::OS));
    if version.is_none() {
        if let Some(tool) = metadata {
            for candidate in &tool.detection.commands {
                if let Some(found) = probe_command_with_resolver(
                    resolver,
                    &candidate.command,
                    &candidate.version_arg,
                    detection_timeout(),
                ) {
                    source = candidate.command.clone();
                    version = Some(found);
                    break;
                }
            }
        }
    }
    let installed = version.is_some();
    let status = if installed { "installed" } else { "missing" }.to_string();
    let install_plan = metadata
        .and_then(|tool| build_install_plan_from_metadata(tool, spec, data, manager_versions));
    let uninstall_plan = if installed {
        metadata
            .and_then(|tool| build_uninstall_plan_from_metadata(tool, spec, data, manager_versions))
    } else {
        None
    };
    let version_options = install_plan
        .as_ref()
        .map(|plan| {
            metadata
                .and_then(|tool| selected_source(tool, data, manager_versions))
                .map(|source| build_version_options_from_source(source, plan))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    DevToolCheck {
        id: stable_id_for(data.identities, &spec.id),
        name: metadata
            .map(|tool| tool.name.clone())
            .or_else(|| identity_name_for(data.identities, &spec.id))
            .unwrap_or_else(|| spec.name.clone()),
        summary: spec.summary.clone(),
        description: spec.description.clone(),
        command: source.clone(),
        installed,
        version,
        source,
        status,
        required: tool_is_required(spec, data.identities, &suite_required),
        category: metadata
            .map(|tool| tool.category_id.clone())
            .unwrap_or_else(|| spec.category_id.clone()),
        install_plan,
        uninstall_plan,
        version_options,
        detail: None,
    }
}

fn build_environment_suites(
    suites: &[CatalogSuiteSpec],
    tools: &[DevToolCheck],
    identities: &IdentityFile,
) -> Vec<EnvironmentSuite> {
    suites
        .iter()
        .map(|suite| {
            let tool_ids = suite_tool_ids(suite, identities);
            let suite_tools = tool_ids
                .iter()
                .filter_map(|id| tools.iter().find(|tool| &tool.id == id))
                .collect::<Vec<_>>();
            EnvironmentSuite {
                id: suite.id.clone(),
                name: suite.name.clone(),
                description: suite.description.clone(),
                tool_ids: tool_ids.clone(),
                required_count: suite_required_ids(suite, identities).len(),
                installed_count: suite_tools.iter().filter(|tool| tool.installed).count(),
                missing_count: suite_tools
                    .iter()
                    .filter(|tool| tool.status == "missing")
                    .count(),
                admin_required: suite_tools
                    .iter()
                    .filter_map(|tool| tool.install_plan.as_ref())
                    .any(|plan| plan.needs_admin),
            }
        })
        .collect()
}

fn default_minimum_quality_score() -> i32 {
    45
}

fn detect_install_managers(
    resolver: &CommandResolver,
    data: EnvironmentData,
) -> HashMap<String, String> {
    let mut managers = HashSet::new();
    for policy in data.policy.platforms.values() {
        managers.extend(policy.manager_priority.iter().cloned());
    }
    for tool in &data.metadata.tools {
        for source in &tool.sources {
            managers.insert(source.manager.clone());
        }
    }

    managers
        .into_iter()
        .filter_map(|manager| {
            let (command, arg) = manager_probe_command(&manager)?;
            let version =
                probe_command_with_resolver(resolver, command, arg, manager_detection_timeout())?;
            Some((manager, version))
        })
        .collect()
}

fn detect_install_managers_for_tool(
    resolver: &CommandResolver,
    data: EnvironmentData,
    spec: &CatalogToolSpec,
) -> HashMap<String, String> {
    let Some(tool) = data.metadata.tools.iter().find(|tool| tool.id == spec.id) else {
        return HashMap::new();
    };
    tool.sources
        .iter()
        .filter(|source| platform_matches(&source.platforms, env::consts::OS))
        .filter(|source| {
            install_command_for_source(source).is_some()
                || uninstall_command_for_source(source).is_some()
        })
        .filter_map(|source| {
            let (command, arg) = manager_probe_command(&source.manager)?;
            let version =
                probe_command_with_resolver(resolver, command, arg, manager_detection_timeout())?;
            Some((source.manager.clone(), version))
        })
        .collect()
}

fn manager_probe_command(manager: &str) -> Option<(&'static str, &'static str)> {
    match manager {
        "winget" => Some(("winget", "--version")),
        "scoop" => Some(("scoop", "--version")),
        "choco" => Some(("choco", "--version")),
        "homebrew" => Some(("brew", "--version")),
        _ => None,
    }
}

fn package_manager_status_text(managers: &HashMap<String, String>) -> Option<String> {
    if managers.is_empty() {
        return None;
    }
    let mut pairs = managers
        .iter()
        .map(|(manager, version)| format!("{manager} {version}"))
        .collect::<Vec<_>>();
    pairs.sort();
    Some(pairs.join(" / "))
}

fn build_install_plan_from_metadata(
    tool: &OnlineToolSpec,
    catalog_tool: &CatalogToolSpec,
    data: EnvironmentData,
    manager_versions: &HashMap<String, String>,
) -> Option<InstallPlan> {
    let source = selected_source(tool, data, manager_versions)?;
    let command = install_command_for_source(source)?;
    build_command_plan(catalog_tool, source, command, None)
}

fn build_uninstall_plan_from_metadata(
    tool: &OnlineToolSpec,
    catalog_tool: &CatalogToolSpec,
    data: EnvironmentData,
    manager_versions: &HashMap<String, String>,
) -> Option<InstallPlan> {
    let source = selected_source_for_action(tool, data, manager_versions, "uninstall")?;
    let command = uninstall_command_for_source(source)?;
    build_command_plan(catalog_tool, source, command, None)
}

fn build_command_plan(
    catalog_tool: &CatalogToolSpec,
    source: &OnlineSourceSpec,
    command: &OnlineCommandSpec,
    download_version: Option<&str>,
) -> Option<InstallPlan> {
    let package_id = source
        .package_id
        .clone()
        .unwrap_or_else(|| source.id.clone());
    Some(InstallPlan {
        package_manager: source.manager.clone(),
        package_id,
        command: command.command.clone(),
        needs_admin: command.requires_admin || catalog_tool.requirements.requires_admin,
        notes: tool_notes(catalog_tool, source),
        source_id: source.id.clone(),
        source_quality: source.quality.score,
        download_url: download_url_for_source(source, download_version),
    })
}

fn build_tool_detail_metadata(tool: &OnlineToolSpec) -> ToolDetailMetadata {
    let mut sources = tool
        .sources
        .iter()
        .filter(|source| platform_matches(&source.platforms, env::consts::OS))
        .map(source_detail_metadata)
        .collect::<Vec<_>>();
    sources.sort_by_key(|source| {
        (
            -source.quality.score,
            i32::from(!source.official),
            source.priority,
            source.manager.clone(),
        )
    });
    ToolDetailMetadata {
        aliases: tool.aliases.clone(),
        tags: tool.tags.clone(),
        platforms: tool.platforms.clone(),
        lifecycle: tool.lifecycle.clone(),
        descriptions: normalize_descriptions(tool.descriptions.clone()),
        usage: tool.usage.clone(),
        notes: tool.notes.clone(),
        links: normalize_links(tool.links.clone()),
        dependencies: tool.dependencies.clone(),
        risk: tool.risk.clone(),
        quality: quality_metadata(&tool.quality),
        verify_commands: tool.verify.commands.clone(),
        sources,
    }
}

fn source_detail_metadata(source: &OnlineSourceSpec) -> ToolSourceDetail {
    ToolSourceDetail {
        id: source.id.clone(),
        manager: source.manager.clone(),
        package_id: source.package_id.clone(),
        official: source.official,
        priority: source.priority,
        platforms: source.platforms.clone(),
        scan: ToolScanMetadata {
            status: source.scan.status.clone(),
            scanned_at: source.scan.scanned_at.clone(),
            errors: source.scan.errors.clone(),
        },
        package: normalize_package(source.package.clone()),
        descriptions: normalize_descriptions(source.descriptions.clone()),
        usage: source.usage.clone(),
        notes: source.notes.clone(),
        links: normalize_links(source.links.clone()),
        quality: quality_metadata(&source.quality),
        versions: source
            .versions
            .iter()
            .take(60)
            .map(version_detail_metadata)
            .collect(),
        downloads: source
            .downloads
            .iter()
            .filter(|download| {
                platform_matches(std::slice::from_ref(&download.platform), env::consts::OS)
            })
            .take(12)
            .map(download_detail_metadata)
            .collect(),
        commands: source
            .commands
            .iter()
            .filter(|command| command.platform == env::consts::OS)
            .map(command_detail_metadata)
            .collect(),
        verify_commands: source.verify.commands.clone(),
    }
}

fn quality_metadata(quality: &OnlineQualitySpec) -> ToolQualityMetadata {
    ToolQualityMetadata {
        confidence: quality.confidence.clone(),
        score: quality.score,
        official: quality.official,
        last_successful_scan_at: quality.last_successful_scan_at.clone(),
        failure_count: quality.failure_count,
        stale_after_days: quality.stale_after_days,
    }
}

fn version_detail_metadata(version: &OnlineVersionSpec) -> ToolVersionDetail {
    ToolVersionDetail {
        version: version.version.clone(),
        channel: version.channel.clone(),
        latest: version.latest,
        prerelease: version.prerelease,
        lts: version.lts,
        release_date: version.release_date.clone(),
        eol_date: version.eol_date.clone(),
        changelog_url: clean_optional_url(version.changelog_url.clone()),
    }
}

fn download_detail_metadata(download: &OnlineDownloadSpec) -> ToolDownloadDetail {
    ToolDownloadDetail {
        id: if download.id.trim().is_empty() {
            "default".to_string()
        } else {
            download.id.clone()
        },
        version: download.version.clone(),
        platform: download.platform.clone(),
        architecture: download.architecture.clone(),
        kind: if download.kind.trim().is_empty() {
            "package".to_string()
        } else {
            download.kind.clone()
        },
        url: clean_optional_url(download.url.clone()),
        url_type: download.url_type.clone(),
        direct: download.direct,
        sha256: download.sha256.clone(),
        size_bytes: download.size_bytes,
    }
}

fn command_detail_metadata(command: &OnlineCommandSpec) -> ToolCommandDetail {
    ToolCommandDetail {
        action: command.action.clone(),
        manager: command.manager.clone(),
        platform: command.platform.clone(),
        version: command.version.clone(),
        requires_admin: command.requires_admin,
        supports_version: command.supports_version,
        shell: command.shell.clone(),
        command: command.command.clone(),
        template: command.template.clone(),
    }
}

fn normalize_descriptions(mut descriptions: ToolDescriptions) -> ToolDescriptions {
    descriptions.homepage = clean_optional_url(descriptions.homepage);
    descriptions
}

fn normalize_links(mut links: ToolLinks) -> ToolLinks {
    links.homepage = clean_optional_url(links.homepage);
    links.download = clean_optional_url(links.download);
    links.releases = clean_optional_url(links.releases);
    links.docs = clean_optional_url(links.docs);
    links
}

fn normalize_package(mut package: ToolPackageMetadata) -> ToolPackageMetadata {
    package.license_url = clean_optional_url(package.license_url);
    package
}

fn clean_optional_url(value: Option<String>) -> Option<String> {
    value.and_then(|url| {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn selected_source<'a>(
    tool: &'a OnlineToolSpec,
    data: EnvironmentData,
    manager_versions: &HashMap<String, String>,
) -> Option<&'a OnlineSourceSpec> {
    selected_source_for_action(tool, data, manager_versions, "install")
}

fn selected_source_for_action<'a>(
    tool: &'a OnlineToolSpec,
    data: EnvironmentData,
    manager_versions: &HashMap<String, String>,
    action: &str,
) -> Option<&'a OnlineSourceSpec> {
    let platform = env::consts::OS;
    let policy = data.policy.platforms.get(platform);
    tool.sources
        .iter()
        .filter(|source| platform_matches(&source.platforms, platform))
        .filter(|source| source.scan.status != "failed")
        .filter(|source| source.quality.confidence != "low")
        .filter(|source| {
            source.quality.score
                >= policy
                    .map(|policy| policy.minimum_quality_score)
                    .unwrap_or_else(default_minimum_quality_score)
        })
        .filter(|source| manager_versions.contains_key(&source.manager))
        .filter(|source| command_for_source(source, action).is_some())
        .min_by_key(|source| source_rank(source, tool, policy))
}

fn source_rank(
    source: &OnlineSourceSpec,
    tool: &OnlineToolSpec,
    policy: Option<&PlatformSourcePolicy>,
) -> (i32, i32, i32, i32, i32) {
    let manager_rank = policy
        .and_then(|policy| {
            policy
                .manager_priority
                .iter()
                .position(|manager| manager == &source.manager)
        })
        .unwrap_or(99) as i32;
    (
        -tool.quality.score,
        -source.quality.score,
        i32::from(!source.official),
        source.priority,
        manager_rank,
    )
}

fn platform_matches(platforms: &[String], platform: &str) -> bool {
    platforms
        .iter()
        .any(|item| item == platform || item == "all")
}

fn install_command_for_source(source: &OnlineSourceSpec) -> Option<&OnlineCommandSpec> {
    command_for_source(source, "install")
}

fn uninstall_command_for_source(source: &OnlineSourceSpec) -> Option<&OnlineCommandSpec> {
    command_for_source(source, "uninstall")
}

fn command_for_source<'a>(
    source: &'a OnlineSourceSpec,
    action: &str,
) -> Option<&'a OnlineCommandSpec> {
    source.commands.iter().find(|command| {
        command.action == action
            && command.manager == source.manager
            && command.shell == "argv"
            && !command.command.is_empty()
            && command.platform == env::consts::OS
    })
}

fn build_version_options_from_source(
    source: &OnlineSourceSpec,
    plan: &InstallPlan,
) -> Vec<InstallVersionOption> {
    let mut options = vec![InstallVersionOption {
        id: format!("{}:default", source.id),
        label: "Default / latest".to_string(),
        package_id: plan.package_id.clone(),
        command: plan.command.clone(),
        needs_admin: plan.needs_admin,
        source_id: source.id.clone(),
        manager: source.manager.clone(),
        quality_score: source.quality.score,
        download_url: plan.download_url.clone(),
    }];

    let Some(command) = install_command_for_source(source) else {
        return options;
    };
    if !command.supports_version || !command.template.iter().any(|part| part == "{{version}}") {
        return options;
    }

    for version in source
        .versions
        .iter()
        .filter(|version| !version.prerelease)
        .take(30)
    {
        let version_command = command
            .template
            .iter()
            .map(|part| {
                if part == "{{version}}" {
                    version.version.clone()
                } else {
                    part.clone()
                }
            })
            .collect::<Vec<_>>();
        let suffix = if version.latest {
            "latest"
        } else if version.channel.is_empty() {
            "stable"
        } else {
            version.channel.as_str()
        };
        options.push(InstallVersionOption {
            id: format!("{}:{}", source.id, version.version),
            label: format!("{} ({suffix})", version.version),
            package_id: plan.package_id.clone(),
            command: version_command,
            needs_admin: plan.needs_admin,
            source_id: source.id.clone(),
            manager: source.manager.clone(),
            quality_score: source.quality.score,
            download_url: download_url_for_source(source, Some(&version.version)),
        });
    }

    options
}

fn download_url_for_source(source: &OnlineSourceSpec, version: Option<&str>) -> Option<String> {
    source
        .downloads
        .iter()
        .filter(|download| {
            platform_matches(std::slice::from_ref(&download.platform), env::consts::OS)
                && (download.direct || download.url_type == "download-page")
        })
        .find(|download| version.is_none() || download.version.as_deref() == version)
        .or_else(|| {
            source.downloads.iter().find(|download| {
                platform_matches(std::slice::from_ref(&download.platform), env::consts::OS)
                    && (download.direct || download.url_type == "download-page")
            })
        })
        .and_then(|download| download.url.clone())
}

fn tool_notes(tool: &CatalogToolSpec, source: &OnlineSourceSpec) -> String {
    let base = tool
        .description
        .as_deref()
        .or(tool.summary.as_deref())
        .unwrap_or("");
    let source_text = format!(
        "Source: {} / quality {}",
        source.manager, source.quality.score
    );
    if base.trim().is_empty() {
        source_text
    } else {
        format!("{base} {source_text}")
    }
}

fn suite_tool_ids(suite: &CatalogSuiteSpec, identities: &IdentityFile) -> Vec<String> {
    if suite.required_tool_ids.is_empty() && suite.optional_tool_ids.is_empty() {
        return unique_stable_ids(suite.tool_ids.iter(), identities);
    }
    unique_stable_ids(
        suite
            .required_tool_ids
            .iter()
            .chain(suite.optional_tool_ids.iter()),
        identities,
    )
}

fn unique_stable_ids<'a, I>(ids: I, identities: &IdentityFile) -> Vec<String>
where
    I: Iterator<Item = &'a String>,
{
    let mut seen = HashSet::new();
    ids.filter_map(|id| {
        let stable_id = stable_id_for(identities, id);
        if seen.insert(stable_id.clone()) {
            Some(stable_id)
        } else {
            None
        }
    })
    .collect()
}

fn suite_required_ids(suite: &CatalogSuiteSpec, identities: &IdentityFile) -> Vec<String> {
    unique_stable_ids(suite.required_tool_ids.iter(), identities)
}

fn suite_required_tool_ids(
    suites: &[CatalogSuiteSpec],
    identities: &IdentityFile,
) -> HashSet<String> {
    suites
        .iter()
        .flat_map(|suite| suite.required_tool_ids.iter())
        .map(|id| stable_id_for(identities, id))
        .collect()
}

fn tool_is_required(
    tool: &CatalogToolSpec,
    identities: &IdentityFile,
    suite_required: &HashSet<String>,
) -> bool {
    tool.requirements.required || suite_required.contains(&stable_id_for(identities, &tool.id))
}

fn stable_id_for(identities: &IdentityFile, id: &str) -> String {
    identities
        .identities
        .iter()
        .find(|identity| identity.current_id == id || identity.stable_id == id)
        .map(|identity| identity.stable_id.clone())
        .unwrap_or_else(|| id.to_string())
}

fn identity_name_for(identities: &IdentityFile, id: &str) -> Option<String> {
    identities
        .identities
        .iter()
        .find(|identity| identity.current_id == id || identity.stable_id == id)
        .map(|identity| identity.current_name.clone())
        .filter(|name| !name.trim().is_empty())
}

fn detection_timeout() -> Duration {
    Duration::from_millis(900)
}

fn manager_detection_timeout() -> Duration {
    Duration::from_millis(650)
}

fn install_plan_for_package(
    tool: &DevToolCheck,
    option_id: &str,
) -> Result<Option<InstallPlan>, String> {
    let option = tool
        .version_options
        .iter()
        .find(|option| option.id == option_id || option.package_id == option_id)
        .ok_or_else(|| "Unknown install version".to_string())?;
    Ok(Some(InstallPlan {
        package_manager: option.manager.clone(),
        package_id: option.package_id.clone(),
        command: option.command.clone(),
        needs_admin: option.needs_admin,
        notes: tool
            .install_plan
            .as_ref()
            .map(|plan| plan.notes.clone())
            .unwrap_or_default(),
        source_id: option.source_id.clone(),
        source_quality: option.quality_score,
        download_url: option.download_url.clone(),
    }))
}

fn shell_join(command: &[String]) -> String {
    command
        .iter()
        .map(|part| {
            if part.chars().all(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ':' | '/' | '\\')
            }) {
                part.clone()
            } else {
                format!("\"{}\"", part.replace('"', "\\\""))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn cmd_echo_escape(value: &str) -> String {
    value
        .replace('^', "^^")
        .replace('&', "^&")
        .replace('|', "^|")
        .replace('<', "^<")
        .replace('>', "^>")
        .replace('%', "%%")
}

fn open_terminal_with_command(command: &[String]) -> Result<(), String> {
    if command.is_empty() {
        return Err("Install command is empty".to_string());
    }
    let command_text = shell_join(command);
    if cfg!(target_os = "windows") {
        let display_command = format!(
            "echo {} && echo. && echo Run the command above when ready.",
            cmd_echo_escape(&command_text)
        );
        Command::new(windows_cmd_executable())
            .args(["/C", "start", "cmd.exe", "/K", &display_command])
            .spawn()
            .map_err(|error| error.to_string())?;
    } else if cfg!(target_os = "macos") {
        let terminal_command = format!(
            "printf '%s\\n' {}; printf '%s\\n' 'Run the command above when ready.'",
            shell_single_quote(&command_text)
        );
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"",
            terminal_command.replace('\\', "\\\\").replace('"', "\\\"")
        );
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|error| error.to_string())?;
    } else {
        let terminal_command = format!(
            "printf '%s\\n' {}; printf '%s\\n' 'Run the command above when ready.'; exec sh",
            shell_single_quote(&command_text)
        );
        Command::new("sh")
            .args([
                "-c",
                &format!(
                    "x-terminal-emulator -e sh -lc {}",
                    shell_single_quote(&terminal_command)
                ),
            ])
            .spawn()
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn fetch_app_update_info() -> AppUpdateInfo {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let mut info = AppUpdateInfo {
        current_version: current_version.clone(),
        latest_version: None,
        update_available: false,
        release_url: APP_RELEASES_URL.to_string(),
        download_url: None,
        asset_name: None,
        published_at: None,
        body: None,
        error: None,
    };

    match fetch_url_text(APP_RELEASE_API_URL, Duration::from_secs(12)).and_then(|content| {
        serde_json::from_str::<GithubRelease>(&content)
            .map_err(|error| format!("Unable to parse GitHub release response: {error}"))
    }) {
        Ok(release) => {
            let asset = select_update_asset(&release.assets);
            info.latest_version = Some(release.tag_name.clone());
            info.update_available = version_is_newer(&release.tag_name, &current_version);
            info.release_url = release.html_url;
            info.published_at = release.published_at;
            info.body = release.body;
            if let Some(asset) = asset {
                info.asset_name = Some(asset.name);
                info.download_url = Some(asset.browser_download_url);
            }
        }
        Err(error) => {
            info.error = Some(error);
        }
    }

    info
}

fn fetch_url_text(url: &str, timeout: Duration) -> Result<String, String> {
    if cfg!(target_os = "windows") {
        let timeout_secs = timeout.as_secs().max(1).to_string();
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; \
             [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; \
             (Invoke-WebRequest -UseBasicParsing -Headers @{{'User-Agent'='DevEnv Manager'}} -Uri {} -TimeoutSec {}).Content",
            powershell_quote(url),
            timeout_secs
        );
        let mut command = Command::new(windows_powershell_executable());
        hide_command_window(&mut command);
        let output = command
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if error.is_empty() {
                "Unable to request GitHub release information.".to_string()
            } else {
                error
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let timeout_secs = timeout.as_secs().max(1).to_string();
        let output = Command::new("curl")
            .args([
                "-fsSL",
                "--max-time",
                &timeout_secs,
                "-H",
                "User-Agent: DevEnv Manager",
                url,
            ])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if error.is_empty() {
                "Unable to request GitHub release information.".to_string()
            } else {
                error
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn select_update_asset(assets: &[GithubReleaseAsset]) -> Option<GithubReleaseAsset> {
    assets
        .iter()
        .map(|asset| (update_asset_score(&asset.name), asset))
        .filter(|(score, _)| *score > 0)
        .max_by_key(|(score, _)| *score)
        .map(|(_, asset)| asset.clone())
}

fn update_asset_score(name: &str) -> i32 {
    let lower = name.to_ascii_lowercase();
    if cfg!(target_os = "windows") {
        [
            (lower.ends_with(".exe"), 120),
            (lower.ends_with(".msi"), 110),
            (lower.contains("setup"), 30),
            (lower.contains("x64") || lower.contains("amd64"), 20),
            (lower.contains("windows") || lower.contains("win"), 15),
            (lower.ends_with(".zip"), 5),
        ]
        .into_iter()
        .filter_map(|(matches, score)| matches.then_some(score))
        .sum()
    } else if cfg!(target_os = "macos") {
        [
            (lower.ends_with(".dmg"), 120),
            (lower.ends_with(".app.tar.gz"), 90),
            (lower.contains("universal") || lower.contains("darwin"), 25),
            (lower.contains("mac") || lower.contains("apple"), 15),
        ]
        .into_iter()
        .filter_map(|(matches, score)| matches.then_some(score))
        .sum()
    } else {
        [
            (lower.ends_with(".appimage"), 120),
            (lower.ends_with(".deb"), 100),
            (lower.ends_with(".rpm"), 70),
            (lower.contains("linux"), 20),
            (lower.contains("x86_64") || lower.contains("amd64"), 15),
        ]
        .into_iter()
        .filter_map(|(matches, score)| matches.then_some(score))
        .sum()
    }
}

fn version_is_newer(candidate: &str, current: &str) -> bool {
    let candidate_parts = version_parts(candidate);
    let current_parts = version_parts(current);
    let max_len = candidate_parts.len().max(current_parts.len());
    for index in 0..max_len {
        let candidate_part = candidate_parts.get(index).copied().unwrap_or(0);
        let current_part = current_parts.get(index).copied().unwrap_or(0);
        if candidate_part != current_part {
            return candidate_part > current_part;
        }
    }
    false
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn open_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") && !trimmed.starts_with("http://") {
        return Err("Only HTTP or HTTPS links can be opened.".to_string());
    }

    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new(windows_cmd_executable());
        command.args(["/C", "start", "", trimmed]);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(trimmed);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(trimmed);
        command
    };

    command.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn install_tool_check(tool: DevToolCheck) -> Result<InstallResult, String> {
    let plan = tool
        .install_plan
        .clone()
        .ok_or_else(|| "No install plan is available for this tool".to_string())?;
    run_tool_plan(tool, plan, "install-tool", "installed", "Install command")
}

fn uninstall_tool_check(tool: DevToolCheck) -> Result<InstallResult, String> {
    if !tool.installed {
        return Err("Tool is not installed".to_string());
    }
    let plan = tool
        .uninstall_plan
        .clone()
        .ok_or_else(|| "No uninstall plan is available for this tool".to_string())?;
    run_tool_plan(
        tool,
        plan,
        "uninstall-tool",
        "uninstalled",
        "Uninstall command",
    )
}

fn run_tool_plan(
    tool: DevToolCheck,
    plan: InstallPlan,
    audit_action: &str,
    success_status: &str,
    command_label: &str,
) -> Result<InstallResult, String> {
    if plan.command.is_empty() {
        return Err(format!("{command_label} is empty"));
    }
    let output = run_command_with_timeout(
        &plan.command[0],
        &plan.command[1..],
        Duration::from_secs(60 * 30),
    )
    .map_err(|error| error.to_string())?
    .ok_or_else(|| format!("{command_label} timed out"))?;
    let status = if output.status.success() {
        success_status
    } else {
        "failed"
    }
    .to_string();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    append_audit_log(
        audit_action,
        &tool.name,
        &status,
        &format!("{}: {}", plan.package_manager, combined.trim()),
    )?;
    Ok(InstallResult {
        tool_id: tool.id,
        status,
        exit_code: output.status.code(),
        output: combined,
    })
}

#[derive(Debug, Clone)]
struct CommandResolver {
    search_paths: Vec<PathBuf>,
}

impl CommandResolver {
    fn new() -> Self {
        Self {
            search_paths: command_search_paths(),
        }
    }

    fn resolve(&self, command: &str) -> Option<String> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return None;
        }

        if command_has_path_separator(trimmed) {
            let path = PathBuf::from(trimmed);
            return valid_resolved_command_path(&path);
        }

        resolve_command_path_in_dirs(trimmed, &self.search_paths).or_else(|| {
            command_lookup_fallback(trimmed)
                .into_iter()
                .filter(|path| !is_windows_store_alias(path))
                .find(|path| !cfg!(target_os = "windows") || is_windows_executable_path(path))
        })
    }

    #[cfg(test)]
    fn from_search_paths(search_paths: Vec<PathBuf>) -> Self {
        Self { search_paths }
    }
}

fn probe_command_with_resolver(
    resolver: &CommandResolver,
    command: &str,
    version_arg: &str,
    timeout: Duration,
) -> Option<String> {
    let executable = resolver.resolve(command)?;
    if !should_execute_version_probe(command, version_arg) {
        return Some("installed".to_string());
    }
    let output =
        run_command_with_timeout(&executable, &[version_arg.to_string()], timeout).ok()??;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let version = combined.lines().next().unwrap_or("").trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn should_execute_version_probe(command: &str, version_arg: &str) -> bool {
    if cfg!(target_os = "windows") && command.eq_ignore_ascii_case("wt") {
        return false;
    }
    !version_arg.trim().is_empty()
}

fn run_command_with_timeout(
    command: &str,
    args: &[String],
    timeout: Duration,
) -> Result<Option<Output>, std::io::Error> {
    let resolved_command = if command_has_path_separator(command) {
        command.to_string()
    } else {
        CommandResolver::new()
            .resolve(command)
            .unwrap_or_else(|| command.to_string())
    };
    let mut process = if cfg!(target_os = "windows") && is_windows_batch_path(&resolved_command) {
        let mut process = Command::new(windows_cmd_executable());
        process.arg("/C").arg(format!(
            "call \"{}\"",
            resolved_command.replace('"', "\"\"")
        ));
        process
    } else if cfg!(target_os = "windows") && is_windows_powershell_script_path(&resolved_command) {
        let mut process = Command::new(windows_powershell_executable());
        process
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&resolved_command);
        process
    } else {
        Command::new(&resolved_command)
    };
    hide_command_window(&mut process);
    let mut child = process
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().map(Some);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn command_output_hidden(command: &mut Command) -> Result<Output, std::io::Error> {
    hide_command_window(command);
    command.stdin(Stdio::null()).output()
}

#[cfg(target_os = "windows")]
fn hide_command_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_command_window(_command: &mut Command) {}

fn command_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path_value) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path_value));
    }
    if cfg!(target_os = "windows") {
        for value in windows_registry_path_values() {
            paths.extend(env::split_paths(std::ffi::OsStr::new(&value)));
        }
        paths.extend(windows_common_command_paths());
    }
    dedupe_paths(paths)
}

fn resolve_command_path_in_dirs(command: &str, search_paths: &[PathBuf]) -> Option<String> {
    let candidate_names = command_candidate_names(command);
    for directory in search_paths {
        for candidate_name in &candidate_names {
            if let Some(path) = valid_resolved_command_path(&directory.join(candidate_name)) {
                return Some(path);
            }
        }
    }
    None
}

fn valid_resolved_command_path(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    let path_text = path.to_string_lossy().to_string();
    if cfg!(target_os = "windows")
        && (is_windows_store_alias(&path_text) || !is_windows_executable_path(&path_text))
    {
        return None;
    }
    Some(path_text)
}

fn command_lookup_fallback(command: &str) -> Vec<String> {
    let lookup_tool = if cfg!(target_os = "windows") {
        windows_system32_executable("where.exe")
    } else {
        PathBuf::from("which")
    };
    let mut lookup_command = Command::new(lookup_tool);
    lookup_command.arg(command);
    let Ok(output) = command_output_hidden(&mut lookup_command) else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn command_candidate_names(command: &str) -> Vec<String> {
    if !cfg!(target_os = "windows") || Path::new(command).extension().is_some() {
        return vec![command.to_string()];
    }

    let mut names = Vec::new();
    let mut seen = HashSet::new();
    for extension in windows_executable_extensions() {
        let candidate = format!("{command}{extension}");
        if seen.insert(candidate.to_ascii_lowercase()) {
            names.push(candidate);
        }
    }
    if seen.insert(command.to_ascii_lowercase()) {
        names.push(command.to_string());
    }
    names
}

fn windows_executable_extensions() -> Vec<String> {
    let mut extensions = Vec::new();
    let mut seen = HashSet::new();
    for extension in [".exe", ".cmd", ".bat", ".com", ".ps1"] {
        if seen.insert(extension.to_string()) {
            extensions.push(extension.to_string());
        }
    }
    if let Some(value) = env::var_os("PATHEXT") {
        for extension in value.to_string_lossy().split(';') {
            let trimmed = extension.trim();
            if trimmed.is_empty() {
                continue;
            }
            let normalized = if trimmed.starts_with('.') {
                trimmed.to_ascii_lowercase()
            } else {
                format!(".{}", trimmed.to_ascii_lowercase())
            };
            if seen.insert(normalized.clone()) {
                extensions.push(normalized);
            }
        }
    }
    extensions
}

fn command_has_path_separator(command: &str) -> bool {
    command.contains('\\') || command.contains('/') || Path::new(command).is_absolute()
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        if path.as_os_str().is_empty() {
            continue;
        }
        let key = if cfg!(target_os = "windows") {
            path.to_string_lossy()
                .trim_matches('"')
                .to_ascii_lowercase()
        } else {
            path.to_string_lossy().trim_matches('"').to_string()
        };
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        deduped.push(PathBuf::from(
            path.to_string_lossy().trim_matches('"').to_string(),
        ));
    }
    deduped
}

fn windows_registry_path_values() -> Vec<String> {
    if !cfg!(target_os = "windows") {
        return Vec::new();
    }
    [
        r"HKCU\Environment",
        r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
    ]
    .into_iter()
    .filter_map(|key| read_windows_registry_value(key, "Path"))
    .map(|value| expand_windows_env_vars(&value))
    .collect()
}

fn read_windows_registry_value(key: &str, value_name: &str) -> Option<String> {
    let mut command = Command::new(windows_system32_executable("reg.exe"));
    command.args(["query", key, "/v", value_name]);
    let output = command_output_hidden(&mut command).ok()?;
    if !output.status.success() {
        return None;
    }
    let value_name_lower = value_name.to_ascii_lowercase();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() >= 3
            && parts[0].eq_ignore_ascii_case(&value_name_lower)
            && parts[1].starts_with("REG_")
        {
            return Some(parts[2..].join(" "));
        }
    }
    None
}

fn expand_windows_env_vars(value: &str) -> String {
    let mut output = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('%') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let name = &after_start[..end];
        if let Some(replacement) = env::var_os(name) {
            output.push_str(&replacement.to_string_lossy());
        } else {
            output.push('%');
            output.push_str(name);
            output.push('%');
        }
        rest = &after_start[end + 1..];
    }
    output.push_str(rest);
    output
}

fn windows_common_command_paths() -> Vec<PathBuf> {
    let home = home_dir().unwrap_or_else(|| PathBuf::from(r"C:\Users\Default"));
    let local_app_data = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Local"));
    let app_data = env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Roaming"));
    let program_files = env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    let program_files_x86 = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
    let program_data = env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let system_root = windows_system_root();

    windows_common_command_paths_for(
        &home,
        &local_app_data,
        &app_data,
        &program_files,
        &program_files_x86,
        &program_data,
        &system_root,
    )
}

fn windows_common_command_paths_for(
    home: &Path,
    local_app_data: &Path,
    app_data: &Path,
    program_files: &Path,
    program_files_x86: &Path,
    program_data: &Path,
    system_root: &Path,
) -> Vec<PathBuf> {
    dedupe_paths(vec![
        system_root.join("System32"),
        system_root.to_path_buf(),
        system_root.join("System32").join("Wbem"),
        system_root
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0"),
        local_app_data.join("Microsoft").join("WindowsApps"),
        local_app_data
            .join("Programs")
            .join("Python")
            .join("Launcher"),
        local_app_data
            .join("Programs")
            .join("Microsoft VS Code")
            .join("bin"),
        local_app_data.join("Programs").join("Git").join("cmd"),
        local_app_data.join("Programs").join("Git").join("bin"),
        app_data.join("npm"),
        home.join(".cargo").join("bin"),
        home.join("go").join("bin"),
        home.join(".deno").join("bin"),
        home.join(".bun").join("bin"),
        program_files.join("Git").join("cmd"),
        program_files.join("Git").join("bin"),
        program_files.join("nodejs"),
        program_files.join("Go").join("bin"),
        program_files.join("GitHub CLI"),
        program_files
            .join("Docker")
            .join("Docker")
            .join("resources")
            .join("bin"),
        program_files.join("PowerShell").join("7"),
        program_files.join("Microsoft VS Code").join("bin"),
        program_files
            .join("Microsoft Visual Studio")
            .join("2022")
            .join("Community")
            .join("Common7")
            .join("IDE"),
        program_files
            .join("Microsoft Visual Studio")
            .join("2022")
            .join("BuildTools")
            .join("Common7")
            .join("IDE"),
        program_files_x86
            .join("Microsoft Visual Studio")
            .join("2022")
            .join("Community")
            .join("Common7")
            .join("IDE"),
        program_files_x86
            .join("Microsoft Visual Studio")
            .join("2022")
            .join("BuildTools")
            .join("Common7")
            .join("IDE"),
        program_data.join("chocolatey").join("bin"),
    ])
}

fn windows_system_root() -> PathBuf {
    env::var_os("SystemRoot")
        .or_else(|| env::var_os("windir"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

fn windows_system32_executable(name: &str) -> PathBuf {
    windows_system_root().join("System32").join(name)
}

fn windows_cmd_executable() -> PathBuf {
    let cmd = windows_system32_executable("cmd.exe");
    if cmd.exists() {
        cmd
    } else {
        PathBuf::from("cmd.exe")
    }
}

fn windows_powershell_executable() -> PathBuf {
    let system_powershell = windows_system_root()
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if system_powershell.exists() {
        system_powershell
    } else {
        PathBuf::from("powershell.exe")
    }
}

fn is_windows_store_alias(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    if !lowered.contains("\\windowsapps\\") || !lowered.ends_with(".exe") {
        return false;
    }
    let file_name = lowered.rsplit(['\\', '/']).next().unwrap_or_default();
    matches!(
        file_name,
        "python.exe" | "python3.exe" | "pythonw.exe" | "python3w.exe"
    )
}

fn is_windows_executable_path(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.ends_with(".exe")
        || lowered.ends_with(".cmd")
        || lowered.ends_with(".bat")
        || lowered.ends_with(".com")
        || lowered.ends_with(".ps1")
}

fn is_windows_batch_path(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.ends_with(".cmd") || lowered.ends_with(".bat")
}

fn is_windows_powershell_script_path(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".ps1")
}

fn spec_owned(
    id: String,
    name_key: String,
    path: PathBuf,
    category: Category,
    risk: Risk,
    description_key: String,
) -> EnvSpec {
    EnvSpec {
        id,
        name_key,
        path,
        category,
        risk,
        description_key,
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

fn desktop_dir() -> Option<PathBuf> {
    let home = home_dir()?;
    let desktop = home.join("Desktop");
    if desktop.exists() {
        Some(desktop)
    } else {
        Some(home)
    }
}

fn quarantine_dir() -> Result<PathBuf, String> {
    Ok(home_dir()
        .ok_or_else(|| "Unable to resolve home directory".to_string())?
        .join(".devenv-manager")
        .join("quarantine"))
}

fn quarantine_manifest() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".devenv-manager")
        .join("quarantine.json")
}

fn audit_log_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".devenv-manager")
        .join("audit-log.json")
}

fn write_quarantine_manifest(item: &QuarantineItem) -> Result<(), String> {
    let mut items = list_quarantine().unwrap_or_default();
    items.push(item.clone());
    let manifest = quarantine_manifest();
    if let Some(parent) = manifest.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        manifest,
        serde_json::to_string_pretty(&items).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn read_audit_log() -> Result<Vec<AuditLogEntry>, String> {
    let path = audit_log_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn append_audit_log(
    action: &str,
    target_path: &str,
    status: &str,
    detail: &str,
) -> Result<(), String> {
    let mut entries = read_audit_log().unwrap_or_default();
    let timestamp = current_epoch_secs();
    entries.insert(
        0,
        AuditLogEntry {
            id: format!("{timestamp}_{}", sanitize_id(action)),
            action: action.to_string(),
            target_path: target_path.to_string(),
            status: status.to_string(),
            detail: detail.to_string(),
            created: timestamp.to_string(),
        },
    );
    entries.truncate(300);
    let path = audit_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&entries).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn available_roots() -> Vec<PathBuf> {
    if cfg!(target_os = "windows") {
        ('A'..='Z')
            .map(|letter| PathBuf::from(format!("{letter}:\\")))
            .filter(|path| path.exists())
            .collect()
    } else {
        vec![PathBuf::from("/")]
    }
}

fn normalize_root(value: &str) -> Result<PathBuf, String> {
    let mut input = value.trim().to_string();
    if input.len() == 2 && input.ends_with(':') {
        input.push('\\');
    }
    let path = PathBuf::from(input);
    if !path.exists() {
        return Err("Drive does not exist".to_string());
    }
    Ok(path)
}

fn path_entries() -> Vec<PathEntry> {
    let separator = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };
    env::var_os("PATH")
        .unwrap_or_default()
        .to_string_lossy()
        .split(separator)
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(PathEntry {
                path: trimmed.to_string(),
                exists: Path::new(trimmed).exists(),
                source: "PATH".to_string(),
            })
        })
        .collect()
}

fn dir_size_limited(path: &Path, limits: ScanLimits) -> SizeEstimate {
    let started = Instant::now();
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return SizeEstimate {
            bytes: 0,
            files: 0,
            dirs: 0,
            truncated: false,
        };
    };
    if metadata.file_type().is_symlink() {
        return SizeEstimate {
            bytes: 0,
            files: 0,
            dirs: 0,
            truncated: true,
        };
    }
    if metadata.is_file() {
        return SizeEstimate {
            bytes: metadata.len(),
            files: 1,
            dirs: 0,
            truncated: false,
        };
    }

    let bytes = Arc::new(AtomicU64::new(0));
    let files = Arc::new(AtomicU64::new(0));
    let dirs = Arc::new(AtomicU64::new(0));
    let visited = Arc::new(AtomicU64::new(0));
    let truncated = Arc::new(AtomicBool::new(false));
    let walker = ignore::WalkBuilder::new(path)
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false)
        .threads(walker_threads(limits))
        .build_parallel();

    walker.run(|| {
        let bytes = Arc::clone(&bytes);
        let files = Arc::clone(&files);
        let dirs = Arc::clone(&dirs);
        let visited = Arc::clone(&visited);
        let truncated = Arc::clone(&truncated);
        Box::new(move |result| {
            if truncated.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }
            let seen = visited.fetch_add(1, Ordering::Relaxed);
            if seen >= limits.max_entries || started.elapsed() >= limits.max_duration {
                truncated.store(true, Ordering::Relaxed);
                return ignore::WalkState::Quit;
            }

            let Ok(entry) = result else {
                return ignore::WalkState::Continue;
            };
            let file_type = entry.file_type();
            if file_type.is_some_and(|kind| kind.is_symlink()) {
                return ignore::WalkState::Continue;
            }
            if file_type.is_some_and(|kind| kind.is_file()) {
                if let Ok(metadata) = entry.metadata() {
                    files.fetch_add(1, Ordering::Relaxed);
                    bytes.fetch_add(metadata.len(), Ordering::Relaxed);
                }
                return ignore::WalkState::Continue;
            }
            if file_type.is_some_and(|kind| kind.is_dir()) {
                dirs.fetch_add(1, Ordering::Relaxed);
            }
            ignore::WalkState::Continue
        })
    });

    SizeEstimate {
        bytes: bytes.load(Ordering::Relaxed),
        files: files.load(Ordering::Relaxed),
        dirs: dirs.load(Ordering::Relaxed),
        truncated: truncated.load(Ordering::Relaxed),
    }
}

fn walker_threads(limits: ScanLimits) -> usize {
    if limits.max_duration <= Duration::from_millis(30) || limits.max_entries <= 1_000 {
        1
    } else {
        std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(4)
            .clamp(2, 8)
    }
}

fn modified_time(path: &Path) -> Option<String> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let since_epoch = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    Some(since_epoch.as_secs().to_string())
}

fn render_html_report(report: &ScanReport, locale: &str) -> String {
    let zh = locale == "zh-CN";
    let title = if zh {
        "开发环境管家报告"
    } else {
        "DevEnv Manager Report"
    };
    let rows = report
        .entries
        .iter()
        .filter(|entry| entry.exists)
        .map(|entry| {
            format!(
                "<tr><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                tr(&entry.name_key, locale),
                html_escape(&entry.path),
                format_bytes(entry.size_bytes),
                if entry.size_approximate {
                    if zh {
                        "估算"
                    } else {
                        "Estimated"
                    }
                } else {
                    if zh {
                        "精确"
                    } else {
                        "Exact"
                    }
                },
                tr_risk(&entry.risk, locale)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!doctype html>
<html lang="{locale}">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
body{{font-family:Segoe UI,Microsoft YaHei,Arial,sans-serif;margin:28px;background:#f6f8fb;color:#182235}}
main{{max-width:1180px;margin:0 auto}}
table{{width:100%;border-collapse:collapse;background:#fff;border:1px solid #d9e0ea}}
th,td{{padding:10px;border-bottom:1px solid #d9e0ea;text-align:left;vertical-align:top}}
th{{background:#edf3f8}}
code{{background:#eef5ff;border:1px solid #d6e6fb;border-radius:6px;padding:2px 5px}}
.muted{{color:#637083}}
</style>
</head>
<body><main>
<h1>{title}</h1>
<p class="muted">{home}: <code>{home_dir}</code> · {platform}: {platform_value} · {total}: {total_size}</p>
<table>
<thead><tr><th>{name}</th><th>{path}</th><th>{size}</th><th>{accuracy}</th><th>{risk}</th></tr></thead>
<tbody>{rows}</tbody>
</table>
</main></body></html>"#,
        locale = locale,
        title = title,
        home = if zh { "用户目录" } else { "Home" },
        home_dir = html_escape(&report.home_dir),
        platform = if zh { "平台" } else { "Platform" },
        platform_value = html_escape(&report.platform),
        total = if zh { "总大小" } else { "Total size" },
        total_size = format_bytes(report.total_size_bytes),
        name = if zh { "名称" } else { "Name" },
        path = if zh { "路径" } else { "Path" },
        size = if zh { "大小" } else { "Size" },
        accuracy = if zh { "精度" } else { "Accuracy" },
        risk = if zh { "风险" } else { "Risk" },
        rows = rows
    )
}

fn tr(key: &str, locale: &str) -> String {
    if let Some((_, drive)) = key.split_once(':') {
        return if locale == "zh-CN" {
            format!("磁盘根目录 {drive}")
        } else {
            format!("Drive root {drive}")
        };
    }
    let zh = locale == "zh-CN";
    match (key, zh) {
        ("env.codexHome", true) => "Codex 主目录",
        ("env.codexSkills", true) => "Codex Skills",
        ("env.codexCache", true) => "Codex 运行时缓存",
        ("env.generalCache", true) => "通用缓存",
        ("env.cargoHome", true) => "Cargo 主目录",
        ("env.cargoBin", true) => "Cargo 命令入口",
        ("env.rustupHome", true) => "Rustup 主目录",
        ("env.rustToolchains", true) => "Rust 工具链",
        ("env.openclawHome", true) => "OpenClaw",
        ("env.qclawHome", true) => "QClaw",
        ("env.claudeHome", true) => "Claude 主目录",
        ("env.codeiumHome", true) => "Codeium/Windsurf",
        ("env.configHome", true) => "配置目录",
        ("env.goPath", true) => "Go 工作区",
        ("env.goCache", true) => "Go 构建缓存",
        ("env.mavenHome", true) => "Maven 本地仓库",
        ("env.gradleHome", true) => "Gradle 主目录",
        ("env.ivyHome", true) => "Ivy 缓存",
        ("env.sbtHome", true) => "sbt 缓存",
        ("env.denoHome", true) => "Deno",
        ("env.bunHome", true) => "Bun",
        ("env.pnpmHome", true) => "pnpm Store",
        ("env.yarnCache", true) => "Yarn 缓存",
        ("env.poetryCache", true) => "Poetry 缓存",
        ("env.uvCache", true) => "uv 缓存",
        ("env.condaHome", true) => "Conda",
        ("env.dockerHome", true) => "Docker 配置",
        ("env.vscodeHome", true) => "VS Code 配置",
        ("env.androidHome", true) => "Android 配置",
        ("env.androidGradle", true) => "Android/Gradle 缓存",
        ("env.flutterHome", true) => "Flutter 配置",
        ("env.npmGlobal", true) => "npm 全局目录",
        ("env.npmCache", true) => "npm 缓存",
        ("env.pythonPrograms", true) => "Python 安装",
        ("env.pipCache", true) => "pip 缓存",
        ("env.openaiLocal", true) => "OpenAI 本地数据",
        ("env.claudeRoaming", true) => "Claude 数据",
        ("env.pnpmLocal", true) => "pnpm 本地数据",
        ("env.yarnLocalCache", true) => "Yarn 本地缓存",
        ("env.dockerDesktop", true) => "Docker Desktop",
        ("env.jetbrainsCache", true) => "JetBrains 数据",
        ("env.vscodeRoaming", true) => "VS Code 用户数据",
        ("env.visualStudioCache", true) => "Visual Studio 数据",
        ("env.nugetCache", true) => "NuGet 包缓存",
        ("env.androidSdk", true) => "Android SDK",
        ("env.wingetCache", true) => "winget 缓存",
        ("env.temp", true) => "临时目录",
        ("env.driveUsers", true) => "用户目录集合",
        ("env.programFiles", true) => "Program Files",
        ("env.programFilesX86", true) => "Program Files (x86)",
        ("env.programData", true) => "ProgramData",
        ("env.vscodeExtensions", true) => "VS Code 扩展",
        ("env.nodeGlobalModules", true) => "Node 全局模块",
        ("env.goModuleCache", true) => "Go 模块缓存",
        ("env.cargoRegistry", true) => "Cargo Registry",
        ("env.cargoGitCache", true) => "Cargo Git 缓存",
        ("env.rustTargetCache", true) => "Rust 构建产物",
        ("env.dockerWslData", true) => "Docker WSL 数据",
        ("env.visualStudioPackages", true) => "Visual Studio 安装包缓存",
        ("env.dotnetTools", true) => ".NET 全局工具",
        ("env.denoCache", true) => "Deno 缓存",
        ("env.bunInstallCache", true) => "Bun 安装缓存",
        ("env.androidBuildCache", true) => "Android 构建缓存",
        ("env.windowsTemp", true) => "Windows 临时目录",
        ("env.windowsDownloaded", true) => "Windows 更新缓存",
        ("env.codexHome", false) => "Codex Home",
        ("env.codexSkills", false) => "Codex Skills",
        ("env.codexCache", false) => "Codex Runtime Cache",
        ("env.generalCache", false) => "General Cache",
        ("env.cargoHome", false) => "Cargo Home",
        ("env.cargoBin", false) => "Cargo Binaries",
        ("env.rustupHome", false) => "Rustup Home",
        ("env.rustToolchains", false) => "Rust Toolchains",
        ("env.openclawHome", false) => "OpenClaw",
        ("env.qclawHome", false) => "QClaw",
        ("env.claudeHome", false) => "Claude Home",
        ("env.codeiumHome", false) => "Codeium/Windsurf",
        ("env.configHome", false) => "Config Home",
        ("env.goPath", false) => "Go Workspace",
        ("env.goCache", false) => "Go Build Cache",
        ("env.mavenHome", false) => "Maven Local Repository",
        ("env.gradleHome", false) => "Gradle Home",
        ("env.ivyHome", false) => "Ivy Cache",
        ("env.sbtHome", false) => "sbt Cache",
        ("env.denoHome", false) => "Deno",
        ("env.bunHome", false) => "Bun",
        ("env.pnpmHome", false) => "pnpm Store",
        ("env.yarnCache", false) => "Yarn Cache",
        ("env.poetryCache", false) => "Poetry Cache",
        ("env.uvCache", false) => "uv Cache",
        ("env.condaHome", false) => "Conda",
        ("env.dockerHome", false) => "Docker Config",
        ("env.vscodeHome", false) => "VS Code Config",
        ("env.androidHome", false) => "Android Config",
        ("env.androidGradle", false) => "Android/Gradle Cache",
        ("env.flutterHome", false) => "Flutter Config",
        ("env.npmGlobal", false) => "npm Global",
        ("env.npmCache", false) => "npm Cache",
        ("env.pythonPrograms", false) => "Python Programs",
        ("env.pipCache", false) => "pip Cache",
        ("env.openaiLocal", false) => "OpenAI Local Data",
        ("env.claudeRoaming", false) => "Claude Data",
        ("env.pnpmLocal", false) => "pnpm Local Data",
        ("env.yarnLocalCache", false) => "Yarn Local Cache",
        ("env.dockerDesktop", false) => "Docker Desktop",
        ("env.jetbrainsCache", false) => "JetBrains Data",
        ("env.vscodeRoaming", false) => "VS Code User Data",
        ("env.visualStudioCache", false) => "Visual Studio Data",
        ("env.nugetCache", false) => "NuGet Package Cache",
        ("env.androidSdk", false) => "Android SDK",
        ("env.wingetCache", false) => "winget Cache",
        ("env.temp", false) => "Temporary Files",
        ("env.driveUsers", false) => "Users",
        ("env.programFiles", false) => "Program Files",
        ("env.programFilesX86", false) => "Program Files (x86)",
        ("env.programData", false) => "ProgramData",
        ("env.vscodeExtensions", false) => "VS Code Extensions",
        ("env.nodeGlobalModules", false) => "Node Global Modules",
        ("env.goModuleCache", false) => "Go Module Cache",
        ("env.cargoRegistry", false) => "Cargo Registry",
        ("env.cargoGitCache", false) => "Cargo Git Cache",
        ("env.rustTargetCache", false) => "Rust Build Artifacts",
        ("env.dockerWslData", false) => "Docker WSL Data",
        ("env.visualStudioPackages", false) => "Visual Studio Package Cache",
        ("env.dotnetTools", false) => ".NET Global Tools",
        ("env.denoCache", false) => "Deno Cache",
        ("env.bunInstallCache", false) => "Bun Install Cache",
        ("env.androidBuildCache", false) => "Android Build Cache",
        ("env.windowsTemp", false) => "Windows Temp",
        ("env.windowsDownloaded", false) => "Windows Update Cache",
        _ => "Unknown",
    }
    .to_string()
}

fn tr_risk(risk: &Risk, locale: &str) -> &'static str {
    match (risk, locale == "zh-CN") {
        (Risk::Keep, true) => "保留",
        (Risk::Caution, true) => "谨慎",
        (Risk::Cleanable, true) => "可清理",
        (Risk::Keep, false) => "Keep",
        (Risk::Caution, false) => "Caution",
        (Risk::Cleanable, false) => "Cleanable",
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_online_metadata_selects_install_source_and_versions() {
        let data = environment_data().expect("embedded environment data should parse");
        let manager = if cfg!(target_os = "windows") {
            "winget"
        } else {
            "homebrew"
        };
        let manager_versions = HashMap::from([(manager.to_string(), "test-version".to_string())]);
        let catalog_tool = data
            .catalog
            .tools
            .iter()
            .find(|tool| tool.id == "git")
            .expect("catalog should include git");
        let online_tool = data
            .metadata
            .tools
            .iter()
            .find(|tool| tool.id == "git")
            .expect("online metadata should include git");

        let source = selected_source(online_tool, data, &manager_versions)
            .expect("policy should select an install source");
        let plan =
            build_install_plan_from_metadata(online_tool, catalog_tool, data, &manager_versions)
                .expect("selected source should produce install plan");
        let options = build_version_options_from_source(source, &plan);

        assert_eq!(source.manager, manager);
        assert!(!plan.command.is_empty());
        assert!(plan.source_quality >= default_minimum_quality_score());
        assert!(options.iter().any(|option| option.id.ends_with(":default")));
        if source
            .commands
            .iter()
            .any(|command| command.action == "install" && command.supports_version)
        {
            assert!(
                options.iter().any(|option| {
                    option.command.iter().any(|part| {
                        part != "{{version}}" && part.chars().any(|ch| ch.is_ascii_digit())
                    })
                }),
                "version-capable source should expose version install options"
            );
        }
    }

    #[test]
    fn embedded_online_metadata_exposes_uninstall_plan() {
        let data = environment_data().expect("embedded environment data should parse");
        let manager = if cfg!(target_os = "windows") {
            "winget"
        } else {
            "homebrew"
        };
        let manager_versions = HashMap::from([(manager.to_string(), "test-version".to_string())]);
        let catalog_tool = data
            .catalog
            .tools
            .iter()
            .find(|tool| tool.id == "git_lfs")
            .expect("catalog should include git lfs");
        let online_tool = data
            .metadata
            .tools
            .iter()
            .find(|tool| tool.id == "git_lfs")
            .expect("online metadata should include git lfs");

        let plan =
            build_uninstall_plan_from_metadata(online_tool, catalog_tool, data, &manager_versions)
                .expect("selected source should produce uninstall plan");

        assert_eq!(plan.package_manager, manager);
        assert!(plan.command.iter().any(|part| part == "uninstall"));
    }

    #[test]
    fn suite_required_tool_ids_drive_required_status() {
        let suite = CatalogSuiteSpec {
            id: "frontend".to_string(),
            name: "Frontend".to_string(),
            description: String::new(),
            tool_ids: vec!["node_lts".to_string(), "vscode".to_string()],
            required_tool_ids: vec!["node_lts".to_string(), "git".to_string()],
            optional_tool_ids: vec![
                "vscode".to_string(),
                "pnpm".to_string(),
                "yarn".to_string(),
                "bun".to_string(),
            ],
        };
        let identities = IdentityFile {
            identities: Vec::new(),
        };
        let required = suite_required_tool_ids(std::slice::from_ref(&suite), &identities);
        let node = CatalogToolSpec {
            id: "node_lts".to_string(),
            name: "Node.js LTS".to_string(),
            category_id: "javascript".to_string(),
            summary: None,
            description: None,
            platforms: vec!["windows".to_string()],
            detection: ToolDetectionSpec {
                commands: vec![ToolCommandSpec {
                    command: "node".to_string(),
                    version_arg: "--version".to_string(),
                }],
            },
            requirements: ToolRequirementSpec {
                required: false,
                requires_admin: false,
            },
        };

        assert!(tool_is_required(&node, &identities, &required));
        assert_eq!(
            suite_tool_ids(&suite, &identities),
            vec!["node_lts", "git", "vscode", "pnpm", "yarn", "bun"]
        );
    }

    #[test]
    fn version_comparison_handles_multi_digit_segments() {
        assert!(version_is_newer("v1.10.0", "1.2.9"));
        assert!(version_is_newer("2.0.0", "1.99.99"));
        assert!(!version_is_newer("v1.2.0", "1.2.0"));
        assert!(!version_is_newer("v1.2.0-beta.1", "1.2.0"));
    }

    #[test]
    fn update_asset_selection_prefers_platform_installers() {
        let assets = vec![
            GithubReleaseAsset {
                name: "devenv-manager-source.zip".to_string(),
                browser_download_url: "https://example.test/source.zip".to_string(),
            },
            GithubReleaseAsset {
                name: if cfg!(target_os = "windows") {
                    "DevEnv-Manager_0.2.0_x64-setup.exe"
                } else if cfg!(target_os = "macos") {
                    "DevEnv-Manager_0.2.0_universal.dmg"
                } else {
                    "devenv-manager_0.2.0_amd64.AppImage"
                }
                .to_string(),
                browser_download_url: "https://example.test/installer".to_string(),
            },
        ];

        let selected = select_update_asset(&assets).expect("asset should be selected");

        assert_eq!(
            selected.browser_download_url,
            "https://example.test/installer"
        );
    }

    #[test]
    fn catalog_split_metadata_and_identities_load() {
        let catalog = load_catalog_tool_file().expect("catalog config should parse");
        let online = load_online_metadata_file().expect("online metadata should parse");
        let policy = load_source_policy_file().expect("source policy should parse");
        let identities = load_identity_file().expect("identity mapping should parse");

        assert!(catalog.tools.len() >= 100);
        assert!(online.tools.len() >= 100);
        assert!(catalog.tools.iter().any(|tool| tool.id == "git"));
        assert!(online.tools.iter().any(|tool| tool.id == "node_lts"));
        assert!(policy.platforms.contains_key("windows"));
        assert!(identities.identities.iter().any(|identity| {
            identity.stable_id == "vscode"
                && identity.current_id == "vscode"
                && identity.current_name == "Visual Studio Code"
        }));
    }

    #[test]
    fn catalog_covers_core_install_suites() {
        let catalog = load_catalog_tool_file().expect("catalog config should parse");
        let suite_ids = catalog
            .suites
            .iter()
            .map(|suite| suite.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let tool_ids = catalog
            .tools
            .iter()
            .map(|tool| tool.id.as_str())
            .collect::<std::collections::HashSet<_>>();

        for expected_suite in [
            "core",
            "frontend",
            "python",
            "native",
            "java",
            "mobile",
            "container",
            "cloud",
        ] {
            assert!(
                suite_ids.contains(expected_suite),
                "missing suite {expected_suite}"
            );
        }
        for expected_tool in [
            "git",
            "node_lts",
            "python_312",
            "rustup",
            "go",
            "temurin_21",
            "dotnet_9",
            "android_studio",
            "docker_desktop",
            "kubectl",
        ] {
            assert!(
                tool_ids.contains(expected_tool),
                "missing tool {expected_tool}"
            );
        }
    }

    #[test]
    fn scan_rule_config_covers_common_developer_locations() {
        let config = load_rule_file().expect("scan rule config should parse");
        let rule_ids = config
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<std::collections::HashSet<_>>();

        for expected_rule in [
            "vscode_extensions",
            "node_global_modules",
            "go_module_cache",
            "cargo_registry",
            "rust_target_cache",
            "docker_wsl_data",
            "visual_studio_packages",
        ] {
            assert!(
                rule_ids.contains(expected_rule),
                "missing scan rule {expected_rule}"
            );
        }
    }

    #[test]
    fn drive_scan_uses_developer_rules_in_selected_root() {
        let home = if cfg!(target_os = "windows") {
            PathBuf::from(r"C:\Users\dev")
        } else {
            PathBuf::from("/home/dev")
        };
        let drive = if cfg!(target_os = "windows") {
            r"C:\"
        } else {
            "/"
        };
        let request = ScanRequest {
            mode: ScanMode::Drive,
            drive: Some(drive.to_string()),
        };

        let specs = scan_specs(&home, &request).expect("drive scan specs should build");

        assert!(
            specs.iter().any(|spec| spec.id == "cargo_home"),
            "drive scan should include configured developer environment rules"
        );
        assert!(
            !specs.iter().any(|spec| spec.id.starts_with("drive_")),
            "drive scan should not replace developer rules with drive summary placeholders"
        );
    }

    #[test]
    fn root_filter_does_not_match_similar_prefixes() {
        if cfg!(target_os = "windows") {
            assert!(path_is_under_root(
                Path::new(r"C:\Users\dev\.cargo"),
                Path::new(r"C:\")
            ));
            assert!(!path_is_under_root(
                Path::new(r"D:\Users\dev\.cargo"),
                Path::new(r"C:\")
            ));
        } else {
            assert!(path_is_under_root(
                Path::new("/home/dev/.cargo"),
                Path::new("/")
            ));
            assert!(!path_is_under_root(
                Path::new("/mnt/dev/.cargo"),
                Path::new("/home")
            ));
        }
    }

    #[test]
    fn windows_store_alias_is_not_treated_as_installed_tool() {
        assert!(is_windows_store_alias(
            r"C:\Users\dev\AppData\Local\Microsoft\WindowsApps\python.exe"
        ));
        assert!(!is_windows_store_alias(
            r"C:\Program Files\Python313\python.exe"
        ));
    }

    #[test]
    fn winget_in_windows_apps_is_not_filtered_as_store_alias() {
        assert!(!is_windows_store_alias(
            r"C:\Users\dev\AppData\Local\Microsoft\WindowsApps\winget.exe"
        ));
    }

    #[test]
    fn windows_batch_paths_are_executable_candidates() {
        assert!(is_windows_executable_path(
            r"C:\Program Files\nodejs\npm.cmd"
        ));
        assert!(is_windows_batch_path(r"C:\Program Files\nodejs\npm.cmd"));
        assert!(!is_windows_batch_path(r"C:\Program Files\nodejs\node.exe"));
    }

    #[test]
    fn windows_command_candidates_prefer_real_launchers() {
        if !cfg!(target_os = "windows") {
            return;
        }
        let candidates = command_candidate_names("npm");

        assert!(candidates.iter().any(|name| name == "npm.exe"));
        assert!(candidates.iter().any(|name| name == "npm.cmd"));
        assert!(
            candidates
                .iter()
                .position(|name| name == "npm.cmd")
                .unwrap()
                < candidates.iter().position(|name| name == "npm").unwrap(),
            "extension-qualified launchers should be checked before extensionless shims"
        );
    }

    #[test]
    fn common_windows_paths_cover_gui_process_path_gaps() {
        if !cfg!(target_os = "windows") {
            return;
        }
        let paths = windows_common_command_paths_for(
            Path::new(r"C:\Users\dev"),
            Path::new(r"C:\Users\dev\AppData\Local"),
            Path::new(r"C:\Users\dev\AppData\Roaming"),
            Path::new(r"C:\Program Files"),
            Path::new(r"C:\Program Files (x86)"),
            Path::new(r"C:\ProgramData"),
            Path::new(r"C:\Windows"),
        )
        .into_iter()
        .map(|path| path.to_string_lossy().to_ascii_lowercase())
        .collect::<Vec<_>>();

        for expected in [
            r"c:\users\dev\appdata\local\microsoft\windowsapps",
            r"c:\program files\git\cmd",
            r"c:\program files\nodejs",
            r"c:\users\dev\.cargo\bin",
            r"c:\program files\github cli",
        ] {
            assert!(
                paths.iter().any(|path| path == expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn resolver_finds_windows_cmd_launcher_without_path_env() {
        if !cfg!(target_os = "windows") {
            return;
        }
        let directory = env::temp_dir().join(format!(
            "devenv-manager-resolver-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temp resolver directory should be created");
        let launcher = directory.join("fake-dev-tool.cmd");
        fs::write(&launcher, "@echo off\r\necho fake\r\n")
            .expect("fake launcher should be created");

        let resolver = CommandResolver::from_search_paths(vec![directory.clone()]);
        let resolved = resolver
            .resolve("fake-dev-tool")
            .expect("resolver should find .cmd launcher");

        assert_eq!(resolved, launcher.to_string_lossy());

        let _ = fs::remove_file(launcher);
        let _ = fs::remove_dir(directory);
    }
}
