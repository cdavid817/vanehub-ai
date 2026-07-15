use crate::command_safety;
use crate::sdk::models::*;
use crate::AppError;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MANIFEST_FILE: &str = "manifest.json";
const INSTALLED_MARKER: &str = ".installed";

pub fn definitions() -> Vec<SdkDefinition> {
    catalog().into_iter().map(definition_to_model).collect()
}

pub fn list_statuses() -> Result<SdkStatusMap, AppError> {
    let mut statuses = BTreeMap::new();
    for definition in catalog() {
        let latest_version = definition
            .fallback_versions
            .first()
            .map(|value| value.to_string());
        let status = status_for_definition(definition, latest_version)?;
        statuses.insert(definition.id, status);
    }
    Ok(statuses)
}

pub fn check_environment() -> SdkEnvironmentStatus {
    let node = find_command("node");
    let npm = find_command(if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    })
    .or_else(|| find_command("npm"));

    let node_version = node
        .as_deref()
        .and_then(|path| command_version(path, "--version"));
    let npm_version = npm
        .as_deref()
        .and_then(|path| command_version(path, "--version"));
    let available = node_version.is_some() && npm_version.is_some();

    SdkEnvironmentStatus {
        available,
        node_path: node,
        node_version,
        npm_path: npm,
        npm_version,
        error: if available {
            None
        } else {
            Some("Node.js or npm was not found on PATH.".to_string())
        },
    }
}

pub fn get_versions(sdk_id: Option<SdkId>) -> SdkVersionMap {
    let definitions = catalog()
        .into_iter()
        .filter(|definition| sdk_id.map(|id| id == definition.id).unwrap_or(true));
    let mut versions = BTreeMap::new();
    for definition in definitions {
        versions.insert(definition.id, version_info(definition));
    }
    versions
}

pub fn check_updates(sdk_id: Option<SdkId>) -> Result<SdkUpdateMap, AppError> {
    let mut updates = BTreeMap::new();
    for definition in catalog() {
        if sdk_id
            .map(|target| target != definition.id)
            .unwrap_or(false)
        {
            continue;
        }
        let installed_version = installed_version(definition.id, definition.npm_package);
        let latest = get_latest_version(definition.id);
        let (latest_version, error_message) = match latest {
            Ok(version) => (Some(version), None),
            Err(error) => (
                definition
                    .fallback_versions
                    .first()
                    .map(|value| value.to_string()),
                Some(error.to_string()),
            ),
        };
        let has_update = installed_version
            .as_deref()
            .zip(latest_version.as_deref())
            .map(|(current, latest)| compare_versions(current, latest) < 0)
            .unwrap_or(false);
        updates.insert(
            definition.id,
            SdkUpdateInfo {
                id: definition.id,
                latest_version,
                has_update,
                error_message,
            },
        );
    }
    Ok(updates)
}

pub fn install(request: SdkOperationRequest, operation: SdkOperationType) -> SdkOperationResult {
    let Some(definition) = definition_by_id(request.sdk_id) else {
        return operation_failure(
            request.sdk_id,
            operation,
            "Unknown SDK id",
            None,
            Vec::new(),
        );
    };
    install_definition(definition, request.version.as_deref(), operation)
}

pub fn uninstall(sdk_id: SdkId) -> SdkOperationResult {
    let operation = SdkOperationType::Uninstall;
    let mut logs = Vec::new();
    let mut push = |line: String| logs.push(log(sdk_id, operation, line));

    let sdk_dir = sdk_dir(sdk_id);
    push(format!("Removing {}", sdk_dir.display()));
    if let Err(error) = ensure_child_of_dependencies(&sdk_dir) {
        return operation_failure(sdk_id, operation, &error.to_string(), None, logs);
    }
    if !sdk_dir.exists() {
        push("SDK directory does not exist; nothing to remove".to_string());
        return operation_success(sdk_id, operation, None, None, logs);
    }
    match fs::remove_dir_all(&sdk_dir) {
        Ok(()) => {
            let _ = remove_from_manifest(sdk_id);
            push("SDK directory removed".to_string());
            operation_success(sdk_id, operation, None, None, logs)
        }
        Err(error) => operation_failure(sdk_id, operation, &error.to_string(), None, logs),
    }
}

pub fn operation_logs(_sdk_id: Option<SdkId>) -> Vec<SdkOperationLog> {
    Vec::new()
}

pub fn is_installed(sdk_id: SdkId) -> bool {
    definition_by_id(sdk_id)
        .and_then(|definition| installed_version(definition.id, definition.npm_package))
        .is_some()
}

#[derive(Debug, Clone, Copy)]
struct CatalogDefinition {
    id: SdkId,
    display_name: &'static str,
    npm_package: &'static str,
    default_version: &'static str,
    companion_packages: &'static [&'static str],
    fallback_versions: &'static [&'static str],
    description: &'static str,
    related_providers: &'static [&'static str],
}

fn catalog() -> Vec<CatalogDefinition> {
    vec![
        CatalogDefinition {
            id: SdkId::ClaudeSdk,
            display_name: "Claude Code SDK",
            npm_package: "@anthropic-ai/claude-agent-sdk",
            default_version: "0.2.88",
            companion_packages: &["@anthropic-ai/sdk", "@anthropic-ai/bedrock-sdk"],
            fallback_versions: &["0.2.88", "0.2.81", "0.2.58"],
            description: "Claude AI 功能所需，包含 Agent SDK 和 Bedrock 支持。",
            related_providers: &["anthropic", "bedrock"],
        },
        CatalogDefinition {
            id: SdkId::CodexSdk,
            display_name: "Codex SDK",
            npm_package: "@openai/codex-sdk",
            default_version: "0.117.0",
            companion_packages: &[],
            fallback_versions: &["0.117.0", "0.116.0", "0.115.0"],
            description: "Codex AI 功能所需。",
            related_providers: &["openai"],
        },
    ]
}

fn definition_by_id(sdk_id: SdkId) -> Option<CatalogDefinition> {
    catalog()
        .into_iter()
        .find(|definition| definition.id == sdk_id)
}

fn definition_to_model(definition: CatalogDefinition) -> SdkDefinition {
    SdkDefinition {
        id: definition.id,
        display_name: definition.display_name.to_string(),
        npm_package: definition.npm_package.to_string(),
        companion_packages: definition
            .companion_packages
            .iter()
            .map(|value| value.to_string())
            .collect(),
        fallback_versions: definition
            .fallback_versions
            .iter()
            .map(|value| value.to_string())
            .collect(),
        description: definition.description.to_string(),
        related_providers: definition
            .related_providers
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn dependencies_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    home.join(".vanehub").join("dependencies")
}

fn sdk_dir(sdk_id: SdkId) -> PathBuf {
    dependencies_dir().join(sdk_id.as_str())
}

fn package_dir(sdk_id: SdkId, npm_package: &str) -> PathBuf {
    npm_package
        .split('/')
        .fold(sdk_dir(sdk_id).join("node_modules"), |path, part| {
            path.join(part)
        })
}

fn ensure_child_of_dependencies(path: &Path) -> Result<(), AppError> {
    let normalized_path = normalize_path(path);
    let normalized_root = normalize_path(&dependencies_dir());
    if normalized_path.starts_with(&normalized_root) && normalized_path != normalized_root {
        Ok(())
    } else {
        Err(AppError::Validation(
            "SDK path is outside the VaneHub dependencies directory".to_string(),
        ))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

fn status_for_definition(
    definition: CatalogDefinition,
    latest_version: Option<String>,
) -> Result<SdkStatus, AppError> {
    let install_path = sdk_dir(definition.id);
    let installed_version = installed_version(definition.id, definition.npm_package);
    let has_update = installed_version
        .as_deref()
        .zip(latest_version.as_deref())
        .map(|(current, latest)| compare_versions(current, latest) < 0)
        .unwrap_or(false);
    Ok(SdkStatus {
        id: definition.id,
        display_name: definition.display_name.to_string(),
        npm_package: definition.npm_package.to_string(),
        description: definition.description.to_string(),
        related_providers: definition
            .related_providers
            .iter()
            .map(|value| value.to_string())
            .collect(),
        status: if installed_version.is_some() {
            SdkInstallStatus::Installed
        } else {
            SdkInstallStatus::NotInstalled
        },
        installed_version,
        latest_version,
        has_update,
        install_path: Some(install_path.to_string_lossy().to_string()),
        last_checked: Some(now_string()),
        error_message: None,
    })
}

fn installed_version(sdk_id: SdkId, npm_package: &str) -> Option<String> {
    let package_json = package_dir(sdk_id, npm_package).join("package.json");
    let raw = fs::read_to_string(package_json).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    value
        .get("version")?
        .as_str()
        .map(|value| value.to_string())
}

fn version_info(definition: CatalogDefinition) -> SdkVersionInfo {
    match get_available_versions(definition.id) {
        Ok(versions) if !versions.is_empty() => SdkVersionInfo {
            sdk_id: definition.id,
            latest_version: versions.first().cloned(),
            versions,
            fallback_versions: definition
                .fallback_versions
                .iter()
                .map(|value| value.to_string())
                .collect(),
            source: SdkVersionSource::Remote,
            error: None,
        },
        Ok(_) => fallback_version_info(definition, Some("No remote versions returned".to_string())),
        Err(error) => fallback_version_info(definition, Some(error.to_string())),
    }
}

fn fallback_version_info(definition: CatalogDefinition, error: Option<String>) -> SdkVersionInfo {
    let versions = definition
        .fallback_versions
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    SdkVersionInfo {
        sdk_id: definition.id,
        latest_version: versions.first().cloned(),
        versions: versions.clone(),
        fallback_versions: versions,
        source: SdkVersionSource::Fallback,
        error,
    }
}

fn get_latest_version(sdk_id: SdkId) -> Result<String, AppError> {
    let definition = definition_by_id(sdk_id)
        .ok_or_else(|| AppError::Validation("Unknown SDK id".to_string()))?;
    run_npm_capture(&["view", definition.npm_package, "version"], 5)
}

fn get_available_versions(sdk_id: SdkId) -> Result<Vec<String>, AppError> {
    let definition = definition_by_id(sdk_id)
        .ok_or_else(|| AppError::Validation("Unknown SDK id".to_string()))?;
    let raw = run_npm_capture(&["view", definition.npm_package, "versions", "--json"], 30)?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|error| AppError::Validation(error.to_string()))?;
    let Some(array) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut versions = array
        .iter()
        .filter_map(|item| item.as_str())
        .filter_map(normalize_requested_version)
        .filter(|version| !version.contains('-'))
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| compare_versions(right, left).cmp(&0));
    Ok(versions)
}

fn install_definition(
    definition: CatalogDefinition,
    requested_version: Option<&str>,
    operation: SdkOperationType,
) -> SdkOperationResult {
    let mut logs = Vec::new();
    let mut push = |line: String| logs.push(log(definition.id, operation, line));

    let environment = check_environment();
    if !environment.available {
        return operation_failure(
            definition.id,
            operation,
            environment
                .error
                .as_deref()
                .unwrap_or("Node.js or npm is unavailable"),
            requested_version.map(|value| value.to_string()),
            logs,
        );
    }

    let normalized_version = requested_version
        .and_then(normalize_requested_version)
        .unwrap_or_else(|| definition.default_version.to_string());
    let sdk_dir = sdk_dir(definition.id);
    if let Err(error) = ensure_child_of_dependencies(&sdk_dir) {
        return operation_failure(
            definition.id,
            operation,
            &error.to_string(),
            Some(normalized_version),
            logs,
        );
    }
    if let Err(error) = fs::create_dir_all(&sdk_dir) {
        return operation_failure(
            definition.id,
            operation,
            &error.to_string(),
            Some(normalized_version),
            logs,
        );
    }
    if let Err(error) = create_package_json(&sdk_dir, definition) {
        return operation_failure(
            definition.id,
            operation,
            &error.to_string(),
            Some(normalized_version),
            logs,
        );
    }

    let package_specs = package_specs(definition, &normalized_version);
    push(format!(
        "Using npm: {}",
        environment
            .npm_path
            .clone()
            .unwrap_or_else(|| "npm".to_string())
    ));
    push(format!("Installing {}", package_specs.join(" ")));

    let npm = environment.npm_path.unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            "npm.cmd".to_string()
        } else {
            "npm".to_string()
        }
    });
    let mut command = match command_safety::std_command(&npm) {
        Ok(command) => command,
        Err(error) => {
            return operation_failure(
                definition.id,
                operation,
                &error.to_string(),
                Some(normalized_version),
                logs,
            )
        }
    };
    command_safety::audit_command(
        "sdk.npm.install",
        &npm,
        &[
            "install".to_string(),
            "--prefix".to_string(),
            sdk_dir.to_string_lossy().to_string(),
        ],
    );
    command
        .arg("install")
        .arg("--include=optional")
        .arg("--ignore-scripts")
        .arg("--prefix")
        .arg(&sdk_dir)
        .args(&package_specs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match command.spawn() {
        Ok(mut child) => {
            if let Some(stdout) = child.stdout.take() {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    push(line);
                }
            }
            if let Some(stderr) = child.stderr.take() {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    push(line);
                }
            }
            match child.wait() {
                Ok(status) if status.success() => {
                    let installed = installed_version(definition.id, definition.npm_package)
                        .unwrap_or_else(|| normalized_version.clone());
                    let _ = fs::write(sdk_dir.join(INSTALLED_MARKER), &installed);
                    let _ = update_manifest(definition.id, &installed);
                    push(format!("Installed version: {installed}"));
                    operation_success(
                        definition.id,
                        operation,
                        Some(installed),
                        Some(normalized_version),
                        logs,
                    )
                }
                Ok(status) => operation_failure(
                    definition.id,
                    operation,
                    &format!("npm install failed with status {status}"),
                    Some(normalized_version),
                    logs,
                ),
                Err(error) => operation_failure(
                    definition.id,
                    operation,
                    &error.to_string(),
                    Some(normalized_version),
                    logs,
                ),
            }
        }
        Err(error) => operation_failure(
            definition.id,
            operation,
            &error.to_string(),
            Some(normalized_version),
            logs,
        ),
    }
}

fn run_npm_capture(args: &[&str], _timeout_seconds: u64) -> Result<String, AppError> {
    let environment = check_environment();
    if !environment.available {
        return Err(AppError::Validation(
            environment
                .error
                .unwrap_or_else(|| "Node.js or npm is unavailable".to_string()),
        ));
    }
    let npm = environment.npm_path.unwrap_or_else(|| "npm".to_string());
    let mut command = command_safety::std_command(&npm)?;
    command_safety::audit_command(
        "sdk.npm.capture",
        &npm,
        &args
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
    );
    command.args(args);
    let output = command_output_with_timeout(&mut command, Duration::from_secs(_timeout_seconds))
        .map_err(AppError::LaunchFailed)?;
    if !output.success {
        return Err(AppError::Validation(output.stderr));
    }
    Ok(output.stdout.trim().to_string())
}

fn find_command(command_name: &str) -> Option<String> {
    let output = if cfg!(target_os = "windows") {
        let mut command = command_safety::std_command("where").ok()?;
        command.arg(command_name);
        command_output_with_timeout(&mut command, Duration::from_secs(2)).ok()?
    } else {
        let mut command = command_safety::std_command("sh").ok()?;
        command.arg("-lc").arg(format!("command -v {command_name}"));
        command_output_with_timeout(&mut command, Duration::from_secs(2)).ok()?
    };
    if !output.success {
        return None;
    }
    output
        .stdout
        .lines()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn command_version(command_path: &str, arg: &str) -> Option<String> {
    let mut command = command_safety::std_command(command_path).ok()?;
    command.arg(arg);
    let output = command_output_with_timeout(&mut command, Duration::from_secs(2)).ok()?;
    if !output.success {
        return None;
    }
    Some(output.stdout.trim().to_string())
}

struct CapturedOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn command_output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<CapturedOutput, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| error.to_string())?;
                return Ok(CapturedOutput {
                    success: output.status.success(),
                    stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .map_err(|error| error.to_string())?;
                return Ok(CapturedOutput {
                    success: false,
                    stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    stderr: format!(
                        "Command timed out after {} seconds. {}",
                        timeout.as_secs(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn create_package_json(sdk_dir: &Path, definition: CatalogDefinition) -> Result<(), AppError> {
    let package_json = serde_json::json!({
        "name": format!("{}-container", definition.id.as_str()),
        "version": "1.0.0",
        "private": true
    });
    fs::write(
        sdk_dir.join("package.json"),
        serde_json::to_string_pretty(&package_json)
            .map_err(|error| AppError::Validation(error.to_string()))?,
    )
    .map_err(|error| AppError::Storage(error.to_string()))
}

fn update_manifest(sdk_id: SdkId, version: &str) -> Result<(), AppError> {
    fs::create_dir_all(dependencies_dir()).map_err(|error| AppError::Storage(error.to_string()))?;
    let path = dependencies_dir().join(MANIFEST_FILE);
    let mut manifest = read_manifest(&path);
    manifest.insert(
        sdk_id.as_str().to_string(),
        serde_json::json!({ "version": version, "installedAt": now_string() }),
    );
    write_manifest(&path, &manifest)
}

fn remove_from_manifest(sdk_id: SdkId) -> Result<(), AppError> {
    let path = dependencies_dir().join(MANIFEST_FILE);
    let mut manifest = read_manifest(&path);
    manifest.remove(sdk_id.as_str());
    write_manifest(&path, &manifest)
}

fn read_manifest(path: &Path) -> serde_json::Map<String, Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn write_manifest(path: &Path, manifest: &serde_json::Map<String, Value>) -> Result<(), AppError> {
    fs::write(
        path,
        serde_json::to_string_pretty(manifest)
            .map_err(|error| AppError::Validation(error.to_string()))?,
    )
    .map_err(|error| AppError::Storage(error.to_string()))
}

fn package_specs(definition: CatalogDefinition, version: &str) -> Vec<String> {
    let mut packages = vec![format!("{}@{}", definition.npm_package, version)];
    packages.extend(
        definition
            .companion_packages
            .iter()
            .map(|package| package.to_string()),
    );
    packages
}

pub fn normalize_requested_version(version: &str) -> Option<String> {
    let trimmed = version.trim();
    let trimmed = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    if is_semver_like(trimmed) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn is_semver_like(version: &str) -> bool {
    let mut parts = version.splitn(2, '-');
    let core = parts.next().unwrap_or_default();
    let core_parts = core.split('.').collect::<Vec<_>>();
    if core_parts.len() != 3
        || !core_parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return false;
    }
    parts
        .next()
        .map(|suffix| {
            !suffix.is_empty()
                && suffix
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        })
        .unwrap_or(true)
}

pub fn compare_versions(left: &str, right: &str) -> i32 {
    let left = normalize_requested_version(left).unwrap_or_else(|| left.to_string());
    let right = normalize_requested_version(right).unwrap_or_else(|| right.to_string());
    let left_parts = left.split(['.', '-']).collect::<Vec<_>>();
    let right_parts = right.split(['.', '-']).collect::<Vec<_>>();
    let length = left_parts.len().max(right_parts.len());
    for index in 0..length {
        let left_value = left_parts
            .get(index)
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0);
        let right_value = right_parts
            .get(index)
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0);
        if left_value != right_value {
            return left_value - right_value;
        }
    }
    0
}

fn operation_success(
    sdk_id: SdkId,
    operation: SdkOperationType,
    installed_version: Option<String>,
    requested_version: Option<String>,
    logs: Vec<SdkOperationLog>,
) -> SdkOperationResult {
    SdkOperationResult {
        success: true,
        operation_id: None,
        sdk_id,
        operation,
        installed_version,
        requested_version,
        logs,
        error: None,
    }
}

fn operation_failure(
    sdk_id: SdkId,
    operation: SdkOperationType,
    error: &str,
    requested_version: Option<String>,
    logs: Vec<SdkOperationLog>,
) -> SdkOperationResult {
    SdkOperationResult {
        success: false,
        operation_id: None,
        sdk_id,
        operation,
        installed_version: None,
        requested_version,
        logs,
        error: Some(error.to_string()),
    }
}

fn log(sdk_id: SdkId, operation: SdkOperationType, line: String) -> SdkOperationLog {
    SdkOperationLog {
        sdk_id,
        operation,
        line,
        timestamp: now_string(),
    }
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_known_sdk_ids() {
        assert_eq!(SdkId::parse("claude-sdk"), Some(SdkId::ClaudeSdk));
        assert_eq!(SdkId::parse("unknown-sdk"), None);
    }

    #[test]
    fn validates_requested_versions() {
        assert_eq!(
            normalize_requested_version(" v0.2.81 "),
            Some("0.2.81".to_string())
        );
        assert_eq!(
            normalize_requested_version("1.2.3-beta.1"),
            Some("1.2.3-beta.1".to_string())
        );
        assert_eq!(normalize_requested_version("latest"), None);
        assert_eq!(normalize_requested_version(">=1.0.0"), None);
        assert_eq!(normalize_requested_version("1.0.0 && rm -rf /"), None);
    }

    #[test]
    fn builds_package_specs_from_definition() {
        let definition = definition_by_id(SdkId::ClaudeSdk).expect("definition");
        let specs = package_specs(definition, "0.2.81");
        assert_eq!(specs[0], "@anthropic-ai/claude-agent-sdk@0.2.81");
        assert!(specs.contains(&"@anthropic-ai/sdk".to_string()));
    }

    #[test]
    fn compares_versions() {
        assert!(compare_versions("0.2.81", "0.2.88") < 0);
        assert!(compare_versions("0.2.90", "0.2.88") > 0);
        assert_eq!(compare_versions("0.2.88", "v0.2.88"), 0);
    }

    #[test]
    fn blocks_root_as_sdk_child() {
        let root = dependencies_dir();
        assert!(ensure_child_of_dependencies(&root).is_err());
        assert!(ensure_child_of_dependencies(&root.join("claude-sdk")).is_ok());
    }
}
