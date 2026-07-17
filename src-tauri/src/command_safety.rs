use crate::{record_native_log, AppError, NativeLogLevel};
use std::process::Command;

pub fn validate_executable(executable: &str) -> Result<(), AppError> {
    let trimmed = executable.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "command executable cannot be empty".to_string(),
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "command executable cannot contain control characters".to_string(),
        ));
    }
    Ok(())
}

pub fn std_command(executable: &str) -> Result<Command, AppError> {
    validate_executable(executable)?;
    let mut command = Command::new(executable);
    crate::network_proxy::apply_to_std_command(&mut command);
    Ok(command)
}

pub fn tokio_command(executable: &str) -> Result<tokio::process::Command, AppError> {
    validate_executable(executable)?;
    let mut command = tokio::process::Command::new(executable);
    crate::network_proxy::apply_to_tokio_command(&mut command);
    Ok(command)
}

pub fn audit_command(category: &str, executable: &str, args: &[String]) {
    let args_label = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    record_native_log(
        NativeLogLevel::Info,
        category,
        &format!("executing {executable}{args_label}"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_executable() {
        assert!(validate_executable("   ").is_err());
    }

    #[test]
    fn rejects_control_characters() {
        assert!(validate_executable("node\nserver").is_err());
    }

    #[test]
    fn allows_normal_executable_names_and_paths() {
        assert!(validate_executable("node").is_ok());
        assert!(validate_executable("C:\\Program Files\\nodejs\\node.exe").is_ok());
    }

    #[test]
    fn injects_network_proxy_environment() {
        crate::network_proxy::apply("http://127.0.0.1:7890", "localhost,127.0.0.1")
            .expect("apply proxy");
        let command = std_command("node").expect("command");
        let envs = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().to_string(),
                    value.map(|v| v.to_string_lossy().to_string()),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            envs.get("HTTP_PROXY").and_then(|value| value.as_deref()),
            Some("http://127.0.0.1:7890")
        );
        assert_eq!(
            envs.get("NO_PROXY").and_then(|value| value.as_deref()),
            Some("localhost,127.0.0.1")
        );
        crate::network_proxy::apply("", crate::network_proxy::DEFAULT_BYPASS).expect("clear proxy");
    }
}
