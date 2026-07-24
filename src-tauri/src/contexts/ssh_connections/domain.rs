use thiserror::Error;

pub(crate) mod runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SshAuthMode {
    Password,
    Key,
}

impl SshAuthMode {
    pub(crate) fn parse(value: &str) -> Result<Self, SshConnectionDomainError> {
        match value {
            "password" => Ok(Self::Password),
            "key" => Ok(Self::Key),
            _ => Err(SshConnectionDomainError::InvalidAuthMode),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Key => "key",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SshConnectionTestStatus {
    NotTested,
    Succeeded,
    Failed,
}

impl SshConnectionTestStatus {
    pub(crate) fn parse(value: &str) -> Result<Self, SshConnectionDomainError> {
        match value {
            "not-tested" => Ok(Self::NotTested),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(SshConnectionDomainError::InvalidTestStatus),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotTested => "not-tested",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshHostTrustMetadata {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) algorithm: String,
    pub(crate) fingerprint: String,
    pub(crate) confirmed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshConnectionProfile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) default_path: String,
    pub(crate) auth_mode: SshAuthMode,
    pub(crate) key_path: Option<String>,
    pub(crate) credential_ref: Option<String>,
    pub(crate) revision: i64,
    pub(crate) host_trust: Option<SshHostTrustMetadata>,
    pub(crate) test_status: SshConnectionTestStatus,
    pub(crate) last_connected_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl SshConnectionProfile {
    pub(crate) fn validate(&self) -> Result<(), SshConnectionDomainError> {
        validate_id(&self.id)?;
        validate_required(&self.name, "name")?;
        validate_host(&self.host)?;
        validate_required(&self.user, "user")?;
        validate_required(&self.default_path, "default path")?;
        validate_port(self.port)?;
        if self.revision < 1 {
            return Err(SshConnectionDomainError::InvalidRevision);
        }
        if let Some(trust) = &self.host_trust {
            if trust.host != self.host || trust.port != self.port {
                return Err(SshConnectionDomainError::HostTrustEndpointMismatch);
            }
            validate_required(&trust.algorithm, "host key algorithm")?;
            validate_required(&trust.fingerprint, "host key fingerprint")?;
            validate_required(&trust.confirmed_at, "host key confirmation time")?;
        }
        match self.auth_mode {
            SshAuthMode::Password => {
                if self
                    .key_path
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    return Err(SshConnectionDomainError::KeyPathForPasswordAuth);
                }
            }
            SshAuthMode::Key => {
                validate_required(self.key_path.as_deref().unwrap_or(""), "key path")?;
            }
        }
        Ok(())
    }

    pub(crate) fn has_password(&self) -> bool {
        self.credential_ref.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshConnectionDraft {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) default_path: String,
    pub(crate) auth_mode: SshAuthMode,
    pub(crate) key_path: Option<String>,
}

impl SshConnectionDraft {
    pub(crate) fn validate(&self) -> Result<(), SshConnectionDomainError> {
        validate_required(&self.name, "name")?;
        validate_host(&self.host)?;
        validate_port(self.port)?;
        validate_required(&self.user, "user")?;
        validate_required(&self.default_path, "default path")?;
        match self.auth_mode {
            SshAuthMode::Password => {
                if self
                    .key_path
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    return Err(SshConnectionDomainError::KeyPathForPasswordAuth);
                }
            }
            SshAuthMode::Key => {
                validate_required(self.key_path.as_deref().unwrap_or(""), "key path")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum SshConnectionDomainError {
    #[error("SSH connection {0} cannot be empty.")]
    Required(&'static str),
    #[error("SSH connection id is invalid.")]
    InvalidId,
    #[error("SSH host is invalid.")]
    InvalidHost,
    #[error("SSH port is invalid.")]
    InvalidPort,
    #[error("SSH auth mode is invalid.")]
    InvalidAuthMode,
    #[error("SSH test status is invalid.")]
    InvalidTestStatus,
    #[error("SSH profile revision is invalid.")]
    InvalidRevision,
    #[error("SSH host trust does not match the profile endpoint.")]
    HostTrustEndpointMismatch,
    #[error("SSH key path cannot be set for password authentication.")]
    KeyPathForPasswordAuth,
}

fn validate_id(value: &str) -> Result<(), SshConnectionDomainError> {
    if value.trim().is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(SshConnectionDomainError::InvalidId);
    }
    Ok(())
}

fn validate_required(value: &str, field: &'static str) -> Result<(), SshConnectionDomainError> {
    if value.trim().is_empty() {
        Err(SshConnectionDomainError::Required(field))
    } else {
        Ok(())
    }
}

fn validate_host(value: &str) -> Result<(), SshConnectionDomainError> {
    validate_required(value, "host")?;
    if value
        .chars()
        .any(|character| character.is_control() || matches!(character, '/' | '\\' | '@'))
    {
        return Err(SshConnectionDomainError::InvalidHost);
    }
    Ok(())
}

fn validate_port(value: u16) -> Result<(), SshConnectionDomainError> {
    if value == 0 {
        Err(SshConnectionDomainError::InvalidPort)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_required_fields_auth_mode_and_port() {
        let draft = SshConnectionDraft {
            name: "Dev".to_string(),
            host: "dev.example.com".to_string(),
            port: 22,
            user: "cdavid".to_string(),
            default_path: "/work/app".to_string(),
            auth_mode: SshAuthMode::Key,
            key_path: Some("C:\\keys\\dev".to_string()),
        };

        assert_eq!(draft.validate(), Ok(()));
        assert_eq!(SshAuthMode::parse("password"), Ok(SshAuthMode::Password));
        assert_eq!(
            SshConnectionTestStatus::parse("succeeded"),
            Ok(SshConnectionTestStatus::Succeeded)
        );
    }

    #[test]
    fn rejects_invalid_host_and_key_path_for_password_auth() {
        let mut draft = SshConnectionDraft {
            name: "Dev".to_string(),
            host: "bad/host".to_string(),
            port: 22,
            user: "cdavid".to_string(),
            default_path: "/work/app".to_string(),
            auth_mode: SshAuthMode::Password,
            key_path: None,
        };
        assert_eq!(draft.validate(), Err(SshConnectionDomainError::InvalidHost));

        draft.host = "dev.example.com".to_string();
        draft.key_path = Some("C:\\keys\\dev".to_string());
        assert_eq!(
            draft.validate(),
            Err(SshConnectionDomainError::KeyPathForPasswordAuth)
        );
    }
}
