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
    Ok(Command::new(executable))
}

pub fn tokio_command(executable: &str) -> Result<tokio::process::Command, AppError> {
    validate_executable(executable)?;
    Ok(tokio::process::Command::new(executable))
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
}
