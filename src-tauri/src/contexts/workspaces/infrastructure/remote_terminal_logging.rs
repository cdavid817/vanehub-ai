use crate::contexts::workspaces::application::{ShellLog, WorkspaceLogLevel, WorkspaceShellLogPort};

pub(crate) fn log_remote_terminal_event(logging: &dyn WorkspaceShellLogPort, level: WorkspaceLogLevel, session_id: &str, shell_id: &str, message: &str) {
    logging.write(ShellLog { level, session_id: session_id.to_string(), shell_id: shell_id.to_string(), message: redact_remote_terminal_message(message) });
}

pub(crate) fn redact_remote_terminal_message(message: &str) -> String {
    let mut result = message.to_string();
    for key in ["password", "token", "secret", "api_key", "private_key"] {
        let lower = result.to_ascii_lowercase();
        if let Some(index) = lower.find(key) {
            if let Some(separator) = result[index..].find('=') { result.replace_range(index + separator + 1.., "[REDACTED]"); }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::redact_remote_terminal_message;
    #[test]
    fn redacts_sensitive_diagnostic_values() {
        let redacted = redact_remote_terminal_message("password=hunter2 token=abc key_path=C:\\private\\id command=echo safe");
        assert!(!redacted.contains("hunter2"));
        assert!(!redacted.contains("abc"));
        assert!(redacted.contains("[REDACTED]"));
    }
}
