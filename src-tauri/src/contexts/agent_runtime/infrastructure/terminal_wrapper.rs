use crate::platform::logging;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentTerminalShell {
    WindowsPowerShell,
    WindowsCmd,
    UnixDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentTerminalWrapperRequest {
    pub(crate) terminal_id: String,
    pub(crate) session_folder: Option<PathBuf>,
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) shell: AgentTerminalShell,
    pub(crate) shell_executable: String,
    pub(crate) wrapper_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentTerminalWrapperSpec {
    pub(crate) executable: String,
    pub(crate) args: Vec<OsString>,
    pub(crate) wrapper_path: PathBuf,
    pub(crate) redacted_command: String,
}

pub(crate) fn default_agent_terminal_shell() -> (AgentTerminalShell, String) {
    if cfg!(target_os = "windows") {
        if crate::platform::process::command_exists(
            "powershell.exe",
            std::time::Duration::from_secs(2),
        ) {
            (
                AgentTerminalShell::WindowsPowerShell,
                "powershell.exe".to_string(),
            )
        } else {
            (AgentTerminalShell::WindowsCmd, "cmd.exe".to_string())
        }
    } else {
        (
            AgentTerminalShell::UnixDefault,
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
        )
    }
}

pub(crate) fn generate_agent_terminal_wrapper(
    request: &AgentTerminalWrapperRequest,
) -> Result<AgentTerminalWrapperSpec, String> {
    validate_token(&request.executable, "executable")?;
    for arg in &request.args {
        validate_token(arg, "argument")?;
    }
    fs::create_dir_all(&request.wrapper_dir).map_err(|error| error.to_string())?;
    let wrapper_path = request.wrapper_dir.join(format!(
        "{}{}",
        safe_file_segment(&request.terminal_id),
        script_extension(request.shell)
    ));
    let body = wrapper_body(request);
    fs::write(&wrapper_path, body).map_err(|error| error.to_string())?;
    Ok(AgentTerminalWrapperSpec {
        executable: request.shell_executable.clone(),
        args: wrapper_launch_args(request.shell, &wrapper_path),
        wrapper_path,
        redacted_command: redacted_command(&request.executable, &request.args),
    })
}

fn wrapper_launch_args(shell: AgentTerminalShell, wrapper_path: &Path) -> Vec<OsString> {
    match shell {
        AgentTerminalShell::WindowsPowerShell => vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            wrapper_path.as_os_str().to_owned(),
        ],
        AgentTerminalShell::WindowsCmd => vec![
            "/d".into(),
            "/s".into(),
            "/c".into(),
            wrapper_path.as_os_str().to_owned(),
        ],
        AgentTerminalShell::UnixDefault => vec![wrapper_path.as_os_str().to_owned()],
    }
}

fn wrapper_body(request: &AgentTerminalWrapperRequest) -> String {
    match request.shell {
        AgentTerminalShell::WindowsPowerShell => powershell_wrapper_body(
            request.session_folder.as_deref(),
            &request.executable,
            &request.args,
        ),
        AgentTerminalShell::WindowsCmd => cmd_wrapper_body(
            request.session_folder.as_deref(),
            &request.executable,
            &request.args,
        ),
        AgentTerminalShell::UnixDefault => unix_wrapper_body(
            request.session_folder.as_deref(),
            &request.executable,
            &request.args,
        ),
    }
}

fn powershell_wrapper_body(folder: Option<&Path>, executable: &str, args: &[String]) -> String {
    let mut lines = vec![
        "$ErrorActionPreference = 'Stop'".to_string(),
        "[Console]::InputEncoding = [System.Text.UTF8Encoding]::new()".to_string(),
        "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()".to_string(),
    ];
    if let Some(folder) = folder {
        lines.push(format!(
            "Set-Location -LiteralPath {}",
            powershell_single_quote(&folder.to_string_lossy())
        ));
    }
    let args = args
        .iter()
        .map(|arg| powershell_single_quote(arg))
        .collect::<Vec<_>>();
    let suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    lines.push(format!(
        "& {}{}",
        powershell_single_quote(executable),
        suffix
    ));
    lines.push("exit $LASTEXITCODE".to_string());
    format!("{}\r\n", lines.join("\r\n"))
}

fn cmd_wrapper_body(folder: Option<&Path>, executable: &str, args: &[String]) -> String {
    let mut lines = vec![
        "@echo off".to_string(),
        "setlocal DisableDelayedExpansion".to_string(),
    ];
    if let Some(folder) = folder {
        lines.push(format!("cd /d {}", cmd_quote(&folder.to_string_lossy())));
    }
    let args = args.iter().map(|arg| cmd_quote(arg)).collect::<Vec<_>>();
    let suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    lines.push(format!("{}{}", cmd_quote(executable), suffix));
    lines.push("exit /b %ERRORLEVEL%".to_string());
    format!("{}\r\n", lines.join("\r\n"))
}

fn unix_wrapper_body(folder: Option<&Path>, executable: &str, args: &[String]) -> String {
    let mut lines = vec!["set -e".to_string()];
    if let Some(folder) = folder {
        lines.push(format!(
            "cd -- {}",
            shell_single_quote(&folder.to_string_lossy())
        ));
    }
    let args = args
        .iter()
        .map(|arg| shell_single_quote(arg))
        .collect::<Vec<_>>();
    let suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    lines.push(format!("exec {}{}", shell_single_quote(executable), suffix));
    format!("{}\n", lines.join("\n"))
}

fn script_extension(shell: AgentTerminalShell) -> &'static str {
    match shell {
        AgentTerminalShell::WindowsPowerShell => ".ps1",
        AgentTerminalShell::WindowsCmd => ".cmd",
        AgentTerminalShell::UnixDefault => ".sh",
    }
}

fn powershell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn cmd_quote(value: &str) -> String {
    let escaped = value
        .replace('%', "%%")
        .replace('^', "^^")
        .replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn redacted_command(executable: &str, args: &[String]) -> String {
    let mut tokens = Vec::with_capacity(args.len() + 1);
    tokens.push(executable.to_string());
    tokens.extend(args.iter().cloned());
    logging::redact_text(&tokens.join(" "))
}

fn validate_token(value: &str, label: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} cannot be empty."));
    }
    if trimmed.chars().any(|character| character == '\0') {
        return Err(format!("{label} cannot contain NUL bytes."));
    }
    Ok(())
}

fn safe_file_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn request(shell: AgentTerminalShell) -> (TempDirectory, AgentTerminalWrapperRequest) {
        let directory = TempDirectory::new("agent-terminal-wrapper");
        let folder = directory.path().join("Project With Spaces");
        let wrapper_dir = directory.path().join("wrappers");
        fs::create_dir_all(&folder).expect("folder");
        (
            directory,
            AgentTerminalWrapperRequest {
                terminal_id: "terminal:one".to_string(),
                session_folder: Some(folder),
                executable: r"C:\Program Files\Agent CLI\agent.exe".to_string(),
                args: vec![
                    "--model".to_string(),
                    "gpt test".to_string(),
                    "--api-key=private-token".to_string(),
                    "literal & $(value) \"quoted\" %PATH%".to_string(),
                ],
                shell,
                shell_executable: match shell {
                    AgentTerminalShell::WindowsPowerShell => "powershell.exe",
                    AgentTerminalShell::WindowsCmd => "cmd.exe",
                    AgentTerminalShell::UnixDefault => "/bin/zsh",
                }
                .to_string(),
                wrapper_dir,
            },
        )
    }

    #[test]
    fn powershell_wrapper_preserves_literal_tokens_and_launcher_contract() {
        let (_directory, request) = request(AgentTerminalShell::WindowsPowerShell);

        let spec = generate_agent_terminal_wrapper(&request).expect("wrapper");
        let body = fs::read_to_string(&spec.wrapper_path).expect("body");

        assert_eq!(spec.executable, "powershell.exe");
        assert_eq!(
            spec.args
                .iter()
                .map(|value| value.to_string_lossy().to_string())
                .collect::<Vec<_>>()[..5],
            [
                "-NoLogo",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File"
            ]
        );
        assert!(body.contains("Set-Location -LiteralPath "));
        assert!(body.contains("& 'C:\\Program Files\\Agent CLI\\agent.exe'"));
        assert!(body.contains("'literal & $(value) \"quoted\" %PATH%'"));
    }

    #[test]
    fn cmd_wrapper_preserves_paths_with_spaces_and_cmd_sensitive_chars() {
        let (_directory, request) = request(AgentTerminalShell::WindowsCmd);

        let spec = generate_agent_terminal_wrapper(&request).expect("wrapper");
        let body = fs::read_to_string(&spec.wrapper_path).expect("body");

        assert_eq!(spec.executable, "cmd.exe");
        assert_eq!(
            spec.args
                .iter()
                .map(|value| value.to_string_lossy().to_string())
                .collect::<Vec<_>>()[..3],
            ["/d", "/s", "/c"]
        );
        assert!(body.contains("\"C:\\Program Files\\Agent CLI\\agent.exe\""));
        assert!(body.contains("\"literal & $(value) \"\"quoted\"\" %%PATH%%\""));
    }

    #[test]
    fn unix_wrapper_uses_default_shell_and_single_quote_escaping() {
        let directory = TempDirectory::new("agent-terminal-wrapper-unix");
        let request = AgentTerminalWrapperRequest {
            terminal_id: "terminal-two".to_string(),
            session_folder: Some(PathBuf::from("/tmp/project with spaces")),
            executable: "/usr/local/bin/agent cli".to_string(),
            args: vec![
                "it's literal".to_string(),
                "token=private-token".to_string(),
            ],
            shell: AgentTerminalShell::UnixDefault,
            shell_executable: "/bin/zsh".to_string(),
            wrapper_dir: directory.path().join("wrappers"),
        };

        let spec = generate_agent_terminal_wrapper(&request).expect("wrapper");
        let body = fs::read_to_string(&spec.wrapper_path).expect("body");

        assert_eq!(spec.executable, "/bin/zsh");
        assert_eq!(
            spec.args
                .iter()
                .map(|value| value.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .len(),
            1
        );
        assert!(body.contains("cd -- '/tmp/project with spaces'"));
        assert!(body.contains("exec '/usr/local/bin/agent cli' 'it'\\''s literal'"));
    }

    #[test]
    fn redacted_diagnostics_hide_sensitive_arguments() {
        let (_directory, request) = request(AgentTerminalShell::WindowsPowerShell);

        let spec = generate_agent_terminal_wrapper(&request).expect("wrapper");

        assert!(spec.redacted_command.contains("--api-key=[REDACTED]"));
        assert!(!spec.redacted_command.contains("private-token"));
    }

    #[test]
    fn rejects_empty_and_nul_tokens() {
        let directory = TempDirectory::new("agent-terminal-wrapper-invalid");
        let mut request = AgentTerminalWrapperRequest {
            terminal_id: "terminal".to_string(),
            session_folder: None,
            executable: " ".to_string(),
            args: Vec::new(),
            shell: AgentTerminalShell::UnixDefault,
            shell_executable: "/bin/sh".to_string(),
            wrapper_dir: directory.path().join("wrappers"),
        };
        assert!(generate_agent_terminal_wrapper(&request).is_err());

        request.executable = "agent".to_string();
        request.args = vec!["bad\0arg".to_string()];
        assert!(generate_agent_terminal_wrapper(&request).is_err());
    }
}
