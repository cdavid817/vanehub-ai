use super::DelegationTarget;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationEnvironmentError {
    MissingRequiredVariable,
    UnsafeValue,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DelegationEnvironmentBuilder;

impl DelegationEnvironmentBuilder {
    pub(crate) fn build(
        &self,
        target: DelegationTarget,
        ambient: &BTreeMap<String, String>,
        workspace: &Path,
    ) -> Result<BTreeMap<String, String>, DelegationEnvironmentError> {
        let mut environment = BTreeMap::new();
        for key in required_keys() {
            let value = find_case_insensitive(ambient, key)
                .ok_or(DelegationEnvironmentError::MissingRequiredVariable)?;
            insert_safe(&mut environment, key, value)?;
        }
        for key in optional_keys(target) {
            if let Some(value) = find_case_insensitive(ambient, key) {
                insert_safe(&mut environment, key, value)?;
            }
        }
        environment.insert(
            "VANEHUB_DELEGATION_WORKSPACE".to_owned(),
            workspace.to_string_lossy().into_owned(),
        );
        environment.insert("NO_PROXY".to_owned(), "*".to_owned());
        environment.insert("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned());
        environment.insert("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned());
        Ok(environment)
    }
}

fn required_keys() -> &'static [&'static str] {
    if cfg!(windows) {
        &[
            "SYSTEMROOT",
            "WINDIR",
            "COMSPEC",
            "PATH",
            "PATHEXT",
            "USERPROFILE",
        ]
    } else {
        &["HOME", "PATH"]
    }
}

fn optional_keys(target: DelegationTarget) -> &'static [&'static str] {
    match target {
        DelegationTarget::ClaudeCode => &["APPDATA", "LOCALAPPDATA", "TEMP", "TMP"],
        DelegationTarget::CodexCli => {
            &["APPDATA", "LOCALAPPDATA", "TEMP", "TMP", "XDG_CONFIG_HOME"]
        }
    }
}

fn find_case_insensitive<'a>(ambient: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    ambient
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.as_str())
}

fn insert_safe(
    environment: &mut BTreeMap<String, String>,
    key: &str,
    value: &str,
) -> Result<(), DelegationEnvironmentError> {
    if value.is_empty() || value.len() > 4096 || value.contains(['\0', '\r', '\n']) {
        return Err(DelegationEnvironmentError::UnsafeValue);
    }
    environment.insert(key.to_owned(), value.to_owned());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ambient() -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();
        for key in required_keys() {
            values.insert((*key).to_owned(), format!("safe-{key}"));
        }
        values.extend([
            ("APPDATA".to_owned(), "safe-appdata".to_owned()),
            ("LOCALAPPDATA".to_owned(), "safe-local".to_owned()),
            ("TEMP".to_owned(), "safe-temp".to_owned()),
            ("TMP".to_owned(), "safe-tmp".to_owned()),
            (
                "ANTHROPIC_API_KEY".to_owned(),
                "secret-anthropic".to_owned(),
            ),
            ("OPENAI_API_KEY".to_owned(), "secret-openai".to_owned()),
            ("HTTP_PROXY".to_owned(), "http://secret-proxy".to_owned()),
            ("AWS_SECRET_ACCESS_KEY".to_owned(), "secret-aws".to_owned()),
            ("UNRELATED".to_owned(), "ambient-secret".to_owned()),
        ]);
        values
    }

    #[test]
    fn minimal_environment_keeps_cli_owned_profile_lookup_without_raw_credentials() {
        for target in [DelegationTarget::ClaudeCode, DelegationTarget::CodexCli] {
            let environment = DelegationEnvironmentBuilder
                .build(target, &ambient(), Path::new("C:/isolated/workspace"))
                .expect("environment");
            assert_eq!(environment.get("NO_PROXY").map(String::as_str), Some("*"));
            assert!(environment.contains_key("USERPROFILE") || environment.contains_key("HOME"));
            for forbidden in [
                "ANTHROPIC_API_KEY",
                "OPENAI_API_KEY",
                "HTTP_PROXY",
                "AWS_SECRET_ACCESS_KEY",
                "UNRELATED",
            ] {
                assert!(!environment.contains_key(forbidden));
            }
            let rendered = format!("{environment:?}");
            for secret in [
                "secret-anthropic",
                "secret-openai",
                "secret-proxy",
                "secret-aws",
                "ambient-secret",
            ] {
                assert!(!rendered.contains(secret));
            }
        }
    }

    #[test]
    fn missing_profile_or_control_characters_fail_closed() {
        assert_eq!(
            DelegationEnvironmentBuilder.build(
                DelegationTarget::CodexCli,
                &BTreeMap::new(),
                Path::new("C:/workspace")
            ),
            Err(DelegationEnvironmentError::MissingRequiredVariable)
        );
        let mut values = ambient();
        values.insert(required_keys()[0].to_owned(), "unsafe\nvalue".to_owned());
        assert_eq!(
            DelegationEnvironmentBuilder.build(
                DelegationTarget::ClaudeCode,
                &values,
                Path::new("C:/workspace")
            ),
            Err(DelegationEnvironmentError::UnsafeValue)
        );
    }
}
