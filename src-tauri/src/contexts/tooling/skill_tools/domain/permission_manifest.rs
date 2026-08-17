use super::SkillToolDomainError;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::net::IpAddr;
use url::Url;

const MAX_ENTRIES: usize = 32;
const MAX_VALUE_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillProvenanceTrust {
    BuiltIn,
    Verified,
    Community,
    Local,
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillFilesystemPermissions {
    pub(crate) read: Vec<String>,
    pub(crate) write: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillNetworkPermissions {
    pub(crate) origins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillProcessCommand {
    pub(crate) executable: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) environment: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillProcessPermissions {
    pub(crate) commands: Vec<SkillProcessCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillToolPermissions {
    pub(crate) filesystem: SkillFilesystemPermissions,
    pub(crate) network: SkillNetworkPermissions,
    pub(crate) process: SkillProcessPermissions,
    pub(crate) secrets: Vec<String>,
}

pub(crate) fn parse_permission_manifest(
    value: Option<&Value>,
) -> Result<SkillToolPermissions, SkillToolDomainError> {
    let Some(value) = value else {
        return Ok(SkillToolPermissions::default());
    };
    let object = object(value, "permissions")?;
    reject_unknown(object, &["filesystem", "network", "process", "secrets"])?;
    Ok(SkillToolPermissions {
        filesystem: parse_filesystem(object.get("filesystem"))?,
        network: parse_network(object.get("network"))?,
        process: parse_process(object.get("process"))?,
        secrets: parse_identifiers(object.get("secrets"), "secrets")?,
    })
}

fn parse_filesystem(
    value: Option<&Value>,
) -> Result<SkillFilesystemPermissions, SkillToolDomainError> {
    let Some(value) = value else {
        return Ok(SkillFilesystemPermissions::default());
    };
    let object = object(value, "filesystem")?;
    reject_unknown(object, &["read", "write"])?;
    Ok(SkillFilesystemPermissions {
        read: parse_paths(object.get("read"), "filesystem.read")?,
        write: parse_paths(object.get("write"), "filesystem.write")?,
    })
}

fn parse_network(value: Option<&Value>) -> Result<SkillNetworkPermissions, SkillToolDomainError> {
    let Some(value) = value else {
        return Ok(SkillNetworkPermissions::default());
    };
    let object = object(value, "network")?;
    reject_unknown(object, &["origins"])?;
    let origins = parse_strings(object.get("origins"), "network.origins", |entry| {
        let url = Url::parse(entry).map_err(|_| invalid("network.origins"))?;
        let host = url.host_str().ok_or_else(|| invalid("network.origins"))?;
        let private_host = host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<IpAddr>()
                .is_ok_and(|address| !is_public_address(address));
        if url.scheme() != "https"
            || url.username() != ""
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || host.contains('*')
            || private_host
        {
            return Err(invalid("network.origins"));
        }
        Ok(url.origin().ascii_serialization())
    })?;
    Ok(SkillNetworkPermissions { origins })
}

fn parse_process(value: Option<&Value>) -> Result<SkillProcessPermissions, SkillToolDomainError> {
    let Some(value) = value else {
        return Ok(SkillProcessPermissions::default());
    };
    let process_object = object(value, "process")?;
    reject_unknown(process_object, &["commands"])?;
    let entries = array(process_object.get("commands"), "process.commands")?;
    let mut commands = Vec::with_capacity(entries.len());
    for entry in entries {
        let command = object(entry, "process.commands")?;
        reject_unknown(command, &["executable", "arguments", "environment"])?;
        let executable = required_string(command, "executable")?;
        if !is_identifier(&executable) {
            return Err(invalid("process.executable"));
        }
        commands.push(SkillProcessCommand {
            executable,
            arguments: parse_plain_arguments(command.get("arguments"))?,
            environment: parse_identifiers(command.get("environment"), "process.environment")?,
        });
    }
    commands.sort_by(|left, right| left.executable.cmp(&right.executable));
    if commands
        .windows(2)
        .any(|pair| pair[0].executable == pair[1].executable)
    {
        return Err(invalid("process.commands.duplicate"));
    }
    Ok(SkillProcessPermissions { commands })
}

fn parse_paths(value: Option<&Value>, field: &str) -> Result<Vec<String>, SkillToolDomainError> {
    parse_strings(value, field, |entry| {
        if !entry.starts_with("workspace/")
            || entry.contains('\\')
            || entry
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(invalid(field));
        }
        Ok(entry.to_string())
    })
}

fn parse_identifiers(
    value: Option<&Value>,
    field: &str,
) -> Result<Vec<String>, SkillToolDomainError> {
    parse_strings(value, field, |entry| {
        is_identifier(entry)
            .then(|| entry.to_string())
            .ok_or_else(|| invalid(field))
    })
}

fn parse_plain_arguments(value: Option<&Value>) -> Result<Vec<String>, SkillToolDomainError> {
    parse_strings(value, "process.arguments", |entry| {
        if entry.bytes().any(|byte| byte == 0) {
            Err(invalid("process.arguments"))
        } else {
            Ok(entry.to_string())
        }
    })
}

fn parse_strings<F>(
    value: Option<&Value>,
    field: &str,
    mut normalize: F,
) -> Result<Vec<String>, SkillToolDomainError>
where
    F: FnMut(&str) -> Result<String, SkillToolDomainError>,
{
    let entries = array(value, field)?;
    let mut normalized = BTreeSet::new();
    for value in entries {
        let entry = value.as_str().ok_or_else(|| invalid(field))?;
        if entry.is_empty() || entry.len() > MAX_VALUE_BYTES {
            return Err(invalid(field));
        }
        if !normalized.insert(normalize(entry)?) {
            return Err(invalid(&format!("{field}.duplicate")));
        }
    }
    Ok(normalized.into_iter().collect())
}

fn array<'a>(value: Option<&'a Value>, field: &str) -> Result<&'a [Value], SkillToolDomainError> {
    let Some(value) = value else { return Ok(&[]) };
    let entries = value.as_array().ok_or_else(|| invalid(field))?;
    if entries.len() > MAX_ENTRIES {
        return Err(invalid(field));
    }
    Ok(entries)
}

fn object<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a Map<String, Value>, SkillToolDomainError> {
    value.as_object().ok_or_else(|| invalid(field))
}

fn required_string(
    object: &Map<String, Value>,
    field: &str,
) -> Result<String, SkillToolDomainError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(field))?;
    if value.is_empty() || value.len() > MAX_VALUE_BYTES {
        return Err(invalid(field));
    }
    Ok(value.to_string())
}

fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), SkillToolDomainError> {
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(invalid(field));
    }
    Ok(())
}

fn is_identifier(value: &str) -> bool {
    value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !(address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_unspecified()
                || address.is_broadcast()
                || address.is_documentation())
        }
        IpAddr::V6(address) => {
            !(address.is_loopback()
                || address.is_unspecified()
                || address.is_unique_local()
                || address.is_unicast_link_local())
        }
    }
}

fn invalid(field: &str) -> SkillToolDomainError {
    SkillToolDomainError::InvalidPermissionManifest(field.chars().take(96).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_every_permission_dimension_without_grant_semantics() {
        let manifest = json!({
            "filesystem": {
                "read": ["workspace/references/**", "workspace/src/**"],
                "write": ["workspace/src/generated/**"]
            },
            "network": { "origins": ["https://api.github.com"] },
            "process": { "commands": [{
                "executable": "git",
                "arguments": ["status", "--short"],
                "environment": ["GIT_OPTIONAL_LOCKS"]
            }] },
            "secrets": ["github.token"]
        });

        let parsed = parse_permission_manifest(Some(&manifest)).expect("permission manifest");
        assert_eq!(
            parsed.filesystem.read,
            vec!["workspace/references/**", "workspace/src/**"]
        );
        assert_eq!(parsed.network.origins, vec!["https://api.github.com"]);
        assert_eq!(parsed.process.commands[0].executable, "git");
        assert_eq!(parsed.secrets, vec!["github.token"]);
        assert_eq!(SkillProvenanceTrust::BuiltIn, SkillProvenanceTrust::BuiltIn);
        assert_ne!(
            SkillProvenanceTrust::Verified,
            SkillProvenanceTrust::Community
        );
        assert_ne!(SkillProvenanceTrust::Local, SkillProvenanceTrust::Untrusted);
    }

    #[test]
    fn missing_permission_manifest_is_empty_and_fail_closed_by_default() {
        assert_eq!(
            parse_permission_manifest(None).expect("empty permissions"),
            SkillToolPermissions::default()
        );
    }

    #[test]
    fn adversarial_permission_manifest_fixtures_fail_closed() {
        let fixtures: Vec<Value> = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/skill-tools/adversarial/permission-manifest-cases.json"
        ))
        .expect("fixture array");
        for fixture in fixtures {
            let name = fixture["name"].as_str().expect("fixture name");
            assert!(
                parse_permission_manifest(Some(&fixture["permissions"])).is_err(),
                "{name} must fail closed"
            );
        }
    }
}
