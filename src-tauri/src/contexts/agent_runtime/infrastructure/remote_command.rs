use crate::contexts::agent_runtime::application::{RunnerError, RunnerErrorKind, RunnerLaunchSpec};

const MAX_ARGUMENTS: usize = 256;
const MAX_ENVIRONMENT: usize = 64;
const MAX_FIELD_BYTES: usize = 8 * 1024;
const MAX_COMMAND_BYTES: usize = 64 * 1024;
pub(crate) const REMOTE_PID_PREFIX: &str = "\u{1e}VANEHUB_PID:";
pub(crate) const REMOTE_EXIT_PREFIX: &str = "\u{1e}VANEHUB_EXIT:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedRemoteCommand {
    pub(crate) bytes: Vec<u8>,
}

pub(crate) fn encode_owned_remote_command(
    spec: &RunnerLaunchSpec,
) -> Result<EncodedRemoteCommand, RunnerError> {
    validate_field(&spec.executable, "executable")?;
    if spec.arguments.len() > MAX_ARGUMENTS || spec.environment.len() > MAX_ENVIRONMENT {
        return Err(invalid_command());
    }
    for argument in &spec.arguments {
        validate_field(argument, "argument")?;
    }
    if let Some(cwd) = &spec.cwd {
        validate_field(cwd, "cwd")?;
    }
    for (name, value) in &spec.environment {
        if !valid_environment_name(name) {
            return Err(invalid_command());
        }
        validate_field(value, "environment value")?;
    }

    let mut command = String::new();
    if let Some(cwd) = &spec.cwd {
        command.push_str("cd ");
        command.push_str(&quote(cwd));
        command.push_str(" && ");
    }
    command.push_str("command -v setsid >/dev/null 2>&1 || exit 125; setsid env");
    for (name, value) in &spec.environment {
        command.push(' ');
        command.push_str(&quote(&format!("{name}={value}")));
    }
    command.push(' ');
    command.push_str(&quote(&spec.executable));
    for argument in &spec.arguments {
        command.push(' ');
        command.push_str(&quote(argument));
    }
    command.push_str(" & pid=$!; printf '\\036VANEHUB_PID:%s\\n' \"$pid\"; wait \"$pid\"; status=$?; printf '\\036VANEHUB_EXIT:%s\\n' \"$status\"; exit \"$status\"");
    if command.len() > MAX_COMMAND_BYTES {
        return Err(invalid_command());
    }
    Ok(EncodedRemoteCommand {
        bytes: command.into_bytes(),
    })
}

pub(crate) fn encode_remote_command_probe(executable: &str) -> Result<Vec<u8>, RunnerError> {
    validate_field(executable, "executable")?;
    Ok(format!("command -v {} >/dev/null 2>&1", quote(executable)).into_bytes())
}

pub(crate) fn encode_remote_process_probe(pid: u32) -> Result<Vec<u8>, RunnerError> {
    validate_owned_pid(pid)?;
    Ok(format!("kill -0 -- -{pid} >/dev/null 2>&1").into_bytes())
}

pub(crate) fn encode_remote_process_signal(pid: u32, force: bool) -> Result<Vec<u8>, RunnerError> {
    validate_owned_pid(pid)?;
    let signal = if force { "KILL" } else { "TERM" };
    Ok(format!("kill -{signal} -- -{pid} >/dev/null 2>&1").into_bytes())
}

fn validate_owned_pid(pid: u32) -> Result<(), RunnerError> {
    if pid > 1 {
        Ok(())
    } else {
        Err(invalid_command())
    }
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn validate_field(value: &str, _field: &'static str) -> Result<(), RunnerError> {
    if value.is_empty()
        || value.len() > MAX_FIELD_BYTES
        || value.chars().any(|character| character.is_control())
    {
        Err(invalid_command())
    } else {
        Ok(())
    }
}

fn valid_environment_name(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some('A'..='Z') | Some('_'))
        && characters.all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
}

fn invalid_command() -> RunnerError {
    RunnerError::new(RunnerErrorKind::InvalidLaunch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn spec() -> RunnerLaunchSpec {
        RunnerLaunchSpec {
            session_id: Some("session-1".to_string()),
            executable: "codex".to_string(),
            arguments: vec!["exec".to_string(), "a'b;$(touch nope)".to_string()],
            cwd: Some("/srv/work space".to_string()),
            environment: BTreeMap::from([("TRACEPARENT".to_string(), "00-value".to_string())]),
            pipe_stdin: true,
        }
    }

    #[test]
    fn encodes_every_value_as_posix_data_inside_an_owned_group_wrapper() {
        let encoded = encode_owned_remote_command(&spec()).expect("encoded command");
        let command = String::from_utf8(encoded.bytes).expect("UTF-8 command");

        assert!(command.starts_with("cd '/srv/work space' && command -v setsid"));
        assert!(command.contains("'TRACEPARENT=00-value'"));
        assert!(command.contains("'a'\\''b;$(touch nope)'"));
        assert!(command.contains("setsid env"));
        assert!(command.contains("VANEHUB_PID"));
        assert!(command.contains("VANEHUB_EXIT"));
    }

    #[test]
    fn rejects_controls_unsafe_environment_names_and_all_declared_bounds() {
        for invalid in ["bad\nvalue", "bad\0value", "\u{7f}"] {
            let mut launch = spec();
            launch.arguments = vec![invalid.to_string()];
            assert_eq!(
                encode_owned_remote_command(&launch)
                    .expect_err("unsafe argument")
                    .kind,
                RunnerErrorKind::InvalidLaunch
            );
        }
        for name in ["PATH;id", "lower", "1TOKEN", "A=B", ""] {
            let mut launch = spec();
            launch.environment = BTreeMap::from([(name.to_string(), "value".to_string())]);
            assert!(encode_owned_remote_command(&launch).is_err(), "{name:?}");
        }
        let mut too_many = spec();
        too_many.arguments = vec!["x".to_string(); MAX_ARGUMENTS + 1];
        assert!(encode_owned_remote_command(&too_many).is_err());
        let mut too_large = spec();
        too_large.executable = "x".repeat(MAX_FIELD_BYTES + 1);
        assert!(encode_owned_remote_command(&too_large).is_err());
    }

    #[test]
    fn deterministic_quote_property_never_reopens_unquoted_shell_data() {
        let alphabet = ["a", "'", ";", "$", "(", ")", " ", "\\"];
        for left in alphabet {
            for right in alphabet {
                let value = format!("{left}{right}payload");
                let encoded = quote(&value);
                assert!(encoded.starts_with('\'') && encoded.ends_with('\''));
                assert!(!encoded[1..encoded.len() - 1]
                    .replace("'\\''", "")
                    .contains('\''));
            }
        }
    }

    #[test]
    fn command_probe_encodes_the_executable_as_data() {
        assert_eq!(
            String::from_utf8(encode_remote_command_probe("a'; id").expect("probe"))
                .expect("UTF-8"),
            "command -v 'a'\\''; id' >/dev/null 2>&1"
        );
    }

    #[test]
    fn process_control_accepts_only_an_owned_numeric_process_group() {
        assert_eq!(
            encode_remote_process_signal(42, false).expect("TERM"),
            b"kill -TERM -- -42 >/dev/null 2>&1"
        );
        assert_eq!(
            encode_remote_process_signal(42, true).expect("KILL"),
            b"kill -KILL -- -42 >/dev/null 2>&1"
        );
        assert_eq!(
            encode_remote_process_probe(42).expect("probe"),
            b"kill -0 -- -42 >/dev/null 2>&1"
        );
        assert!(encode_remote_process_signal(0, false).is_err());
        assert!(encode_remote_process_probe(1).is_err());
    }
}
