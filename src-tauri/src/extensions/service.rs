use crate::command_safety;
use crate::extensions::models::*;
use crate::AppError;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

const PYTHON_MIN_MAJOR: u32 = 3;
const PYTHON_MIN_MINOR: u32 = 10;
const INSTALLED_MARKER: &str = ".vanehub-installed";

#[derive(Clone, Copy)]
struct CatalogEntry {
    id: ExtensionFrameworkId,
    capability_id: ExtensionCapabilityId,
    name_key: &'static str,
    description_key: &'static str,
    default_port: u16,
    packages: &'static [&'static str],
    import_module: &'static str,
    version_package: &'static str,
    estimated_download_mb: u64,
    estimated_disk_mb: u64,
    model_id: &'static str,
    model_size_mb: u64,
    model_description_key: &'static str,
}

fn catalog() -> [CatalogEntry; 3] {
    [
        CatalogEntry {
            id: ExtensionFrameworkId::Paddleocr,
            capability_id: ExtensionCapabilityId::Ocr,
            name_key: "extensions.framework.paddleocr.name",
            description_key: "extensions.framework.paddleocr.description",
            default_port: 9875,
            packages: &["paddleocr>=3,<4", "paddlepaddle>=3,<4"],
            import_module: "paddleocr",
            version_package: "paddleocr",
            estimated_download_mb: 650,
            estimated_disk_mb: 1800,
            model_id: "PP-OCRv5-mobile",
            model_size_mb: 120,
            model_description_key: "extensions.model.paddleocr",
        },
        CatalogEntry {
            id: ExtensionFrameworkId::FasterWhisper,
            capability_id: ExtensionCapabilityId::Asr,
            name_key: "extensions.framework.fasterWhisper.name",
            description_key: "extensions.framework.fasterWhisper.description",
            default_port: 9876,
            packages: &["faster-whisper>=1,<2"],
            import_module: "faster_whisper",
            version_package: "faster-whisper",
            estimated_download_mb: 250,
            estimated_disk_mb: 900,
            model_id: "base",
            model_size_mb: 150,
            model_description_key: "extensions.model.fasterWhisper",
        },
        CatalogEntry {
            id: ExtensionFrameworkId::SherpaOnnx,
            capability_id: ExtensionCapabilityId::Tts,
            name_key: "extensions.framework.sherpaOnnx.name",
            description_key: "extensions.framework.sherpaOnnx.description",
            default_port: 9879,
            packages: &["sherpa-onnx>=1,<2"],
            import_module: "sherpa_onnx",
            version_package: "sherpa-onnx",
            estimated_download_mb: 180,
            estimated_disk_mb: 650,
            model_id: "vits-zh-aishell3",
            model_size_mb: 170,
            model_description_key: "extensions.model.sherpaOnnx",
        },
    ]
}

pub fn definitions() -> Vec<ExtensionFrameworkDefinition> {
    catalog().iter().map(|entry| definition(*entry)).collect()
}

fn definition(entry: CatalogEntry) -> ExtensionFrameworkDefinition {
    ExtensionFrameworkDefinition {
        id: entry.id,
        capability_id: entry.capability_id,
        name_key: entry.name_key.to_string(),
        description_key: entry.description_key.to_string(),
        default_port: entry.default_port,
        requirement: ExtensionRequirement {
            runtime: "Python 3.10+".to_string(),
            packages: entry.packages.iter().map(|item| item.to_string()).collect(),
            estimated_download_mb: entry.estimated_download_mb,
            estimated_disk_mb: entry.estimated_disk_mb,
            models: vec![ExtensionModelRequirement {
                id: entry.model_id.to_string(),
                size_mb: entry.model_size_mb,
                description_key: entry.model_description_key.to_string(),
            }],
        },
    }
}

fn entry_by_id(id: ExtensionFrameworkId) -> CatalogEntry {
    catalog()
        .into_iter()
        .find(|entry| entry.id == id)
        .unwrap_or(catalog()[0])
}

pub fn apply_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_framework_state (
            framework_id TEXT PRIMARY KEY,
            capability_id TEXT NOT NULL,
            lifecycle_status TEXT NOT NULL DEFAULT 'not-installed',
            installed INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 0,
            port INTEGER NOT NULL,
            install_path TEXT,
            installed_version TEXT,
            last_health_check TEXT,
            last_error TEXT,
            last_operation_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    for entry in catalog() {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO extension_framework_state
             (framework_id, capability_id, lifecycle_status, installed, enabled, port, created_at, updated_at)
             VALUES (?1, ?2, 'not-installed', 0, 0, ?3, ?4, ?5)",
            params![
                entry.id.as_str(),
                entry.capability_id.as_str(),
                i64::from(entry.default_port),
                now,
                now
            ],
        )?;
    }
    Ok(())
}

pub fn overview(
    conn: &Connection,
    running_frameworks: &HashSet<ExtensionFrameworkId>,
) -> Result<ExtensionOverview, AppError> {
    apply_schema(conn)?;
    let environment = detect_environment();
    let mut statuses = Vec::new();
    for entry in catalog() {
        let mut status = load_status(conn, entry)?;
        status.running = running_frameworks.contains(&entry.id);
        if status.running {
            status.status = ExtensionLifecycleStatus::Running;
        } else if !environment.supported && !status.installed {
            status.status = ExtensionLifecycleStatus::Unsupported;
            status.last_error = environment.reason.clone();
        } else if matches!(status.status, ExtensionLifecycleStatus::Running) {
            status.status = ExtensionLifecycleStatus::Installed;
        }
        statuses.push(status);
    }
    Ok(ExtensionOverview {
        definitions: definitions(),
        statuses,
        environment,
    })
}

fn load_status(
    conn: &Connection,
    entry: CatalogEntry,
) -> Result<ExtensionFrameworkStatus, AppError> {
    conn.query_row(
        "SELECT lifecycle_status, installed, enabled, port, install_path, installed_version,
                last_health_check, last_error, last_operation_id
         FROM extension_framework_state WHERE framework_id = ?1",
        params![entry.id.as_str()],
        |row| {
            Ok(ExtensionFrameworkStatus {
                framework_id: entry.id,
                capability_id: entry.capability_id,
                status: ExtensionLifecycleStatus::parse(&row.get::<_, String>(0)?),
                installed: row.get::<_, i64>(1)? != 0,
                enabled: row.get::<_, i64>(2)? != 0,
                running: false,
                port: row.get::<_, u16>(3)?,
                install_path: row.get(4)?,
                installed_version: row.get(5)?,
                last_health_check: row.get(6)?,
                last_error: row.get(7)?,
                last_operation_id: row.get(8)?,
            })
        },
    )
    .map_err(AppError::from)
}

pub fn detect_environment() -> ExtensionEnvironment {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let platform_supported = os == "windows" && arch == "x86_64";
    let python = resolve_python();
    let version_supported = python
        .as_ref()
        .and_then(|(_, version)| parse_python_version(version))
        .map(|(major, minor)| {
            major > PYTHON_MIN_MAJOR || (major == PYTHON_MIN_MAJOR && minor >= PYTHON_MIN_MINOR)
        })
        .unwrap_or(false);
    let supported = platform_supported && version_supported;
    let reason = if !platform_supported {
        Some("extensions.environment.windowsX64Only".to_string())
    } else if python.is_none() {
        Some("extensions.environment.pythonMissing".to_string())
    } else if !version_supported {
        Some("extensions.environment.pythonVersion".to_string())
    } else {
        None
    };
    ExtensionEnvironment {
        runtime: "tauri".to_string(),
        os,
        arch,
        supported,
        native_operations_available: supported,
        python_path: python.as_ref().map(|(path, _)| path.clone()),
        python_version: python.map(|(_, version)| version),
        reason,
    }
}

fn resolve_python() -> Option<(String, String)> {
    ["python", "python3"].iter().find_map(|candidate| {
        let output = command_safety::std_command(candidate)
            .ok()?
            .arg("--version")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let raw = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };
        let version = raw
            .trim()
            .strip_prefix("Python ")
            .unwrap_or(raw.trim())
            .to_string();
        Some((candidate.to_string(), version))
    })
}

fn parse_python_version(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

pub fn install_preview(id: ExtensionFrameworkId) -> Result<ExtensionInstallPreview, AppError> {
    let entry = entry_by_id(id);
    let environment = detect_environment();
    let path = framework_dir(id)?;
    Ok(ExtensionInstallPreview {
        framework_id: id,
        supported: environment.supported,
        install_path: path.to_string_lossy().to_string(),
        python_path: environment.python_path,
        packages: entry.packages.iter().map(|item| item.to_string()).collect(),
        models: definition(entry).requirement.models,
        estimated_download_mb: entry.estimated_download_mb,
        estimated_disk_mb: entry.estimated_disk_mb,
        inference_local_only: true,
        reason: environment.reason,
    })
}

pub fn set_transition(
    conn: &Connection,
    id: ExtensionFrameworkId,
    status: ExtensionLifecycleStatus,
    operation_id: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE extension_framework_state SET lifecycle_status = ?1, last_operation_id = ?2,
         last_error = NULL, updated_at = ?3 WHERE framework_id = ?4",
        params![
            status.as_str(),
            operation_id,
            Utc::now().to_rfc3339(),
            id.as_str()
        ],
    )?;
    Ok(())
}

pub fn mark_error(
    conn: &Connection,
    id: ExtensionFrameworkId,
    error: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE extension_framework_state SET lifecycle_status = 'error', last_error = ?1,
         updated_at = ?2 WHERE framework_id = ?3",
        params![error, Utc::now().to_rfc3339(), id.as_str()],
    )?;
    Ok(())
}

pub fn install(conn: &Connection, id: ExtensionFrameworkId) -> ExtensionOperationResult {
    let action = ExtensionAction::Install;
    let entry = entry_by_id(id);
    let mut logs = vec![format!("Preparing {} managed environment", id.as_str())];
    let environment = detect_environment();
    let Some(python) = environment.python_path else {
        return failure(
            id,
            action,
            environment
                .reason
                .unwrap_or_else(|| "Python is unavailable".to_string()),
            logs,
        );
    };
    if !environment.supported {
        return failure(
            id,
            action,
            environment
                .reason
                .unwrap_or_else(|| "Unsupported environment".to_string()),
            logs,
        );
    }
    let path = match framework_dir(id) {
        Ok(path) => path,
        Err(error) => return failure(id, action, error.to_string(), logs),
    };
    if let Err(error) = ensure_framework_path(&extensions_root(), &path, id) {
        return failure(id, action, error.to_string(), logs);
    }
    if let Err(error) = fs::create_dir_all(&path) {
        return failure(id, action, error.to_string(), logs);
    }
    logs.push(format!(
        "Creating virtual environment at {}",
        path.display()
    ));
    let path_arg = path.to_string_lossy().to_string();
    if let Err(error) = run_command(&python, &["-m", "venv", &path_arg], &mut logs) {
        return failure(id, action, error.to_string(), logs);
    }
    let venv_python = venv_python(&path);
    let mut install_args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "--disable-pip-version-check".to_string(),
    ];
    install_args.extend(entry.packages.iter().map(|item| item.to_string()));
    let refs = install_args.iter().map(String::as_str).collect::<Vec<_>>();
    logs.push(format!(
        "Installing allowlisted packages: {}",
        entry.packages.join(", ")
    ));
    if let Err(error) = run_command(&venv_python.to_string_lossy(), &refs, &mut logs) {
        return failure(id, action, error.to_string(), logs);
    }
    if let Err(error) = finalize_installation(conn, id, &path, entry, &mut logs) {
        return failure(id, action, error.to_string(), logs);
    }
    logs.push("Framework installation verified".to_string());
    success(id, action, "Framework installed", logs)
}

pub fn uninstall(conn: &Connection, id: ExtensionFrameworkId) -> ExtensionOperationResult {
    let action = ExtensionAction::Uninstall;
    let mut logs = Vec::new();
    let path = match framework_dir(id) {
        Ok(path) => path,
        Err(error) => return failure(id, action, error.to_string(), logs),
    };
    if let Err(error) = ensure_framework_path(&extensions_root(), &path, id) {
        return failure(id, action, error.to_string(), logs);
    }
    logs.push(format!(
        "Removing managed framework directory {}",
        path.display()
    ));
    if path.exists() {
        if let Err(error) = fs::remove_dir_all(&path) {
            return failure(id, action, error.to_string(), logs);
        }
    }
    if let Err(error) = conn.execute(
        "UPDATE extension_framework_state SET lifecycle_status = 'not-installed', installed = 0,
         enabled = 0, install_path = NULL, installed_version = NULL, last_health_check = NULL,
         last_error = NULL, updated_at = ?1 WHERE framework_id = ?2",
        params![Utc::now().to_rfc3339(), id.as_str()],
    ) {
        return failure(id, action, error.to_string(), logs);
    }
    success(id, action, "Framework uninstalled", logs)
}

pub fn set_enabled(
    conn: &Connection,
    id: ExtensionFrameworkId,
    enabled: bool,
) -> ExtensionOperationResult {
    let action = if enabled {
        ExtensionAction::Enable
    } else {
        ExtensionAction::Disable
    };
    let entry = entry_by_id(id);
    let installed = conn
        .query_row(
            "SELECT installed FROM extension_framework_state WHERE framework_id = ?1",
            params![id.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .ok()
        .flatten()
        .unwrap_or(0)
        != 0;
    if enabled && !installed {
        return failure(
            id,
            action,
            "Framework must be installed before it can be enabled".to_string(),
            Vec::new(),
        );
    }
    let transaction = match conn.unchecked_transaction() {
        Ok(transaction) => transaction,
        Err(error) => return failure(id, action, error.to_string(), Vec::new()),
    };
    if enabled {
        if let Err(error) = transaction.execute(
            "UPDATE extension_framework_state SET enabled = 0 WHERE capability_id = ?1",
            params![entry.capability_id.as_str()],
        ) {
            return failure(id, action, error.to_string(), Vec::new());
        }
    }
    if let Err(error) = transaction.execute(
        "UPDATE extension_framework_state SET enabled = ?1, updated_at = ?2 WHERE framework_id = ?3",
        params![if enabled { 1 } else { 0 }, Utc::now().to_rfc3339(), id.as_str()],
    ) {
        return failure(id, action, error.to_string(), Vec::new());
    }
    if let Err(error) = transaction.commit() {
        return failure(id, action, error.to_string(), Vec::new());
    }
    success(
        id,
        action,
        if enabled {
            "Framework enabled"
        } else {
            "Framework disabled"
        },
        Vec::new(),
    )
}

pub fn self_test(conn: &Connection, id: ExtensionFrameworkId) -> ExtensionOperationResult {
    let action = ExtensionAction::SelfTest;
    let entry = entry_by_id(id);
    let path = match framework_dir(id) {
        Ok(path) => path,
        Err(error) => return failure(id, action, error.to_string(), Vec::new()),
    };
    let python = venv_python(&path);
    if !python.exists() || !path.join(INSTALLED_MARKER).exists() {
        return failure(
            id,
            action,
            "Managed framework environment is not installed".to_string(),
            Vec::new(),
        );
    }
    let code = format!("import {}; print('self-test-ok')", entry.import_module);
    let mut logs = vec![format!(
        "Loading {} from the managed environment",
        entry.import_module
    )];
    if let Err(error) = run_command(&python.to_string_lossy(), &["-c", &code], &mut logs) {
        return failure(id, action, error.to_string(), logs);
    }
    let _ = conn.execute(
        "UPDATE extension_framework_state SET last_health_check = ?1, last_error = NULL,
         updated_at = ?2 WHERE framework_id = ?3",
        params![
            Utc::now().to_rfc3339(),
            Utc::now().to_rfc3339(),
            id.as_str()
        ],
    );
    success(id, action, "Runtime self-test passed", logs)
}

pub fn status_for(
    conn: &Connection,
    id: ExtensionFrameworkId,
) -> Result<ExtensionFrameworkStatus, AppError> {
    load_status(conn, entry_by_id(id))
}

pub fn set_running(
    conn: &Connection,
    id: ExtensionFrameworkId,
    running: bool,
    error: Option<&str>,
) -> Result<(), AppError> {
    let status = if running { "running" } else { "installed" };
    conn.execute(
        "UPDATE extension_framework_state SET lifecycle_status = ?1, last_health_check = ?2,
         last_error = ?3, updated_at = ?4 WHERE framework_id = ?5",
        params![
            status,
            Utc::now().to_rfc3339(),
            error,
            Utc::now().to_rfc3339(),
            id.as_str()
        ],
    )?;
    Ok(())
}

pub fn framework_dir(id: ExtensionFrameworkId) -> Result<PathBuf, AppError> {
    let root = extensions_root();
    let target = root.join(id.as_str());
    ensure_framework_path(&root, &target, id)?;
    Ok(target)
}

fn extensions_root() -> PathBuf {
    std::env::var_os("VANEHUB_APP_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("vanehub-ai"))
        .join("extensions")
}

pub fn ensure_framework_path(
    root: &Path,
    target: &Path,
    id: ExtensionFrameworkId,
) -> Result<(), AppError> {
    if target == root || target != root.join(id.as_str()) {
        return Err(AppError::Validation(
            "extension path is outside its managed directory".to_string(),
        ));
    }
    Ok(())
}

pub fn venv_python(path: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        path.join("Scripts").join("python.exe")
    } else {
        path.join("bin").join("python")
    }
}

fn installed_version(python: &Path, entry: CatalogEntry) -> Result<String, AppError> {
    let code = format!(
        "import importlib.metadata as m; print(m.version('{}'))",
        entry.version_package
    );
    let args = vec!["-c".to_string(), code.clone()];
    command_safety::audit_command("extension.operation", &python.to_string_lossy(), &args);
    let output = command_safety::std_command(&python.to_string_lossy())?
        .args(["-c", &code])
        .output()
        .map_err(|error| AppError::LaunchFailed(error.to_string()))?;
    if !output.status.success() {
        return Err(AppError::LaunchFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn finalize_installation(
    conn: &Connection,
    id: ExtensionFrameworkId,
    path: &Path,
    entry: CatalogEntry,
    logs: &mut Vec<String>,
) -> Result<(), AppError> {
    let python = venv_python(path);
    let version = installed_version(&python, entry)?;
    logs.push(format!("Verifying {} import", entry.import_module));
    verify_framework_import(&python, entry, logs)?;

    let marker = path.join(INSTALLED_MARKER);
    fs::write(&marker, &version).map_err(|error| AppError::Storage(error.to_string()))?;
    let update = conn.execute(
        "UPDATE extension_framework_state SET lifecycle_status = 'installed', installed = 1,
         enabled = 0, install_path = ?1, installed_version = ?2, last_error = NULL,
         updated_at = ?3 WHERE framework_id = ?4",
        params![
            path.to_string_lossy(),
            version,
            Utc::now().to_rfc3339(),
            id.as_str()
        ],
    );
    if let Err(error) = update {
        let _ = fs::remove_file(marker);
        return Err(AppError::from(error));
    }
    Ok(())
}

fn verify_framework_import(
    python: &Path,
    entry: CatalogEntry,
    logs: &mut Vec<String>,
) -> Result<(), AppError> {
    let code = format!("import {}; print('self-test-ok')", entry.import_module);
    run_command(&python.to_string_lossy(), &["-c", &code], logs)
}

fn run_command(executable: &str, args: &[&str], logs: &mut Vec<String>) -> Result<(), AppError> {
    let owned_args = args.iter().map(|item| item.to_string()).collect::<Vec<_>>();
    command_safety::audit_command("extension.operation", executable, &owned_args);
    let output = command_safety::std_command(executable)?
        .args(args)
        .output()
        .map_err(|error| AppError::LaunchFailed(error.to_string()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    for line in stdout.lines().chain(stderr.lines()) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            logs.push(trimmed.to_string());
        }
    }
    if !output.status.success() {
        return Err(AppError::LaunchFailed(format!(
            "allowlisted extension command failed with {}",
            output.status
        )));
    }
    Ok(())
}

pub fn spawn_health_sidecar(
    id: ExtensionFrameworkId,
    port: u16,
) -> Result<std::process::Child, AppError> {
    let path = framework_dir(id)?;
    let python = venv_python(&path);
    if !python.exists() {
        return Err(AppError::Validation(
            "managed framework environment is not installed".to_string(),
        ));
    }
    let args = vec![
        "-m".to_string(),
        "http.server".to_string(),
        port.to_string(),
        "--bind".to_string(),
        "127.0.0.1".to_string(),
    ];
    command_safety::audit_command("extension.lifecycle", &python.to_string_lossy(), &args);
    command_safety::std_command(&python.to_string_lossy())?
        .args(args)
        .current_dir(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| AppError::LaunchFailed(error.to_string()))
}

fn success(
    id: ExtensionFrameworkId,
    action: ExtensionAction,
    message: &str,
    logs: Vec<String>,
) -> ExtensionOperationResult {
    ExtensionOperationResult {
        success: true,
        framework_id: id,
        action,
        message: message.to_string(),
        logs,
        error: None,
    }
}

fn failure(
    id: ExtensionFrameworkId,
    action: ExtensionAction,
    error: String,
    logs: Vec<String>,
) -> ExtensionOperationResult {
    ExtensionOperationResult {
        success: false,
        framework_id: id,
        action,
        message: error.clone(),
        logs,
        error: Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("sqlite");
        apply_schema(&conn).expect("schema");
        conn
    }

    #[test]
    fn catalog_preserves_stable_capability_and_framework_ids() {
        let definitions = definitions();
        assert_eq!(
            definitions
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["paddleocr", "faster-whisper", "sherpa-onnx"]
        );
        assert_eq!(
            definitions
                .iter()
                .map(|item| item.capability_id.as_str())
                .collect::<Vec<_>>(),
            vec!["ocr", "asr", "tts"]
        );
    }

    #[test]
    fn schema_seeds_status_rows_and_round_trips_enablement() {
        let conn = test_conn();
        let before = status_for(&conn, ExtensionFrameworkId::Paddleocr).expect("status");
        assert!(!before.installed);
        conn.execute(
            "UPDATE extension_framework_state SET installed = 1 WHERE framework_id = 'paddleocr'",
            [],
        )
        .expect("installed");
        let result = set_enabled(&conn, ExtensionFrameworkId::Paddleocr, true);
        assert!(result.success);
        assert!(
            status_for(&conn, ExtensionFrameworkId::Paddleocr)
                .expect("enabled")
                .enabled
        );
    }

    #[test]
    fn managed_path_rejects_root_and_sibling_targets() {
        let root = std::env::temp_dir().join("vanehub-extension-test");
        assert!(ensure_framework_path(&root, &root, ExtensionFrameworkId::Paddleocr).is_err());
        assert!(
            ensure_framework_path(&root, &root.join("other"), ExtensionFrameworkId::Paddleocr)
                .is_err()
        );
        assert!(ensure_framework_path(
            &root,
            &root.join("paddleocr"),
            ExtensionFrameworkId::Paddleocr
        )
        .is_ok());
    }

    #[test]
    fn python_version_parser_enforces_numeric_major_minor() {
        assert_eq!(parse_python_version("3.12.4"), Some((3, 12)));
        assert_eq!(parse_python_version("invalid"), None);
    }

    #[test]
    fn lifecycle_transitions_persist_without_starting_processes() {
        let conn = test_conn();
        set_transition(
            &conn,
            ExtensionFrameworkId::FasterWhisper,
            ExtensionLifecycleStatus::Installing,
            "op-1",
        )
        .expect("transition");
        let installing =
            status_for(&conn, ExtensionFrameworkId::FasterWhisper).expect("installing");
        assert!(matches!(
            installing.status,
            ExtensionLifecycleStatus::Installing
        ));
        assert_eq!(installing.last_operation_id.as_deref(), Some("op-1"));

        set_running(&conn, ExtensionFrameworkId::FasterWhisper, true, None).expect("running");
        let running =
            status_for(&conn, ExtensionFrameworkId::FasterWhisper).expect("running status");
        assert!(matches!(running.status, ExtensionLifecycleStatus::Running));
        assert!(running.last_health_check.is_some());
    }

    #[test]
    fn environment_support_requires_windows_x64_and_compatible_python() {
        let environment = detect_environment();
        if environment.supported {
            assert_eq!(environment.os, "windows");
            assert_eq!(environment.arch, "x86_64");
            assert!(environment.python_path.is_some());
            assert!(environment.python_version.is_some());
        } else {
            assert!(environment.reason.is_some());
        }
    }

    #[test]
    fn installation_verification_failure_leaves_framework_uninstalled() {
        let conn = test_conn();
        let path = std::env::temp_dir().join(format!(
            "vanehub-extension-verification-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&path).expect("test directory");
        let mut logs = Vec::new();

        let result = finalize_installation(
            &conn,
            ExtensionFrameworkId::Paddleocr,
            &path,
            entry_by_id(ExtensionFrameworkId::Paddleocr),
            &mut logs,
        );

        assert!(result.is_err());
        assert!(!path.join(INSTALLED_MARKER).exists());
        assert!(
            !status_for(&conn, ExtensionFrameworkId::Paddleocr)
                .expect("status")
                .installed
        );
        let _ = fs::remove_dir_all(path);
    }
}
