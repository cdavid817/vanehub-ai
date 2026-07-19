use crate::contexts::tooling::cli::domain::{
    classify_install_source, EnvironmentType, InstallSource, ToolDefinition,
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
    match definition.script_install_url {
        Some(url) => format!(
            "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" {url} && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status' || npm install -g {}@latest",
            definition.package_name
        ),
        None => format!("npm install -g {}@latest", definition.package_name),
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
