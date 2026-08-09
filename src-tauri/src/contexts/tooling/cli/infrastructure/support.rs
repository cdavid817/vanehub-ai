use crate::contexts::tooling::cli::domain::{
    classify_install_source, EnvironmentType, InstallSource, ScriptInstaller, ToolDefinition,
};
use std::path::Path;

pub(super) fn current_environment_type() -> EnvironmentType {
    if cfg!(target_os = "windows") {
        EnvironmentType::Windows
    } else if cfg!(target_os = "macos") {
        EnvironmentType::Macos
    } else if cfg!(target_os = "linux") {
        EnvironmentType::Linux
    } else {
        EnvironmentType::Unknown
    }
}

pub(super) fn install_command_for(definition: ToolDefinition) -> String {
    let npm_fallback = definition
        .package_name
        .map(|package| format!("npm install -g {package}@latest"));
    match (definition.platform_installer(), npm_fallback) {
        (Some(installer), Some(fallback)) => {
            format!("{} || {fallback}", script_install_command(installer))
        }
        (Some(installer), None) => script_install_command(installer),
        (None, Some(fallback)) => fallback,
        // Nothing automated is reachable here; the page renders manual guidance instead, and this
        // string is only ever displayed.
        (None, None) => format!("Install {} manually", definition.display_name),
    }
}

fn script_install_command(installer: ScriptInstaller) -> String {
    match installer {
        ScriptInstaller::Shell(url) => format!(
            "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" {url} && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status'"
        ),
        ScriptInstaller::PowerShell(url) => {
            format!("powershell -NoProfile -ExecutionPolicy Bypass -Command \"irm {url} | iex\"")
        }
    }
}

pub(super) fn npm_executable() -> &'static str {
    if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    }
}

pub(super) fn is_direct_cli_executable(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        matches!(extension.as_str(), "exe" | "cmd" | "bat" | "com")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        true
    }
}

pub(super) fn install_source(path: &Path) -> InstallSource {
    let has_npm_sibling = path
        .parent()
        .map(|parent| parent.join(npm_executable()))
        .is_some_and(|candidate| candidate.is_file());
    classify_install_source(&path.to_string_lossy(), has_npm_sibling)
}
