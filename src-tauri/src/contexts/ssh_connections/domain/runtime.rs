use super::SshConnectionProfile;
use thiserror::Error;

const MAX_HOST_BYTES: usize = 255;
const MAX_ALGORITHM_BYTES: usize = 96;
const MAX_FINGERPRINT_BYTES: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RemoteSshConnectionKey {
    pub(crate) connection_id: String,
    pub(crate) revision: i64,
}

impl From<&SshConnectionProfile> for RemoteSshConnectionKey {
    fn from(profile: &SshConnectionProfile) -> Self {
        Self {
            connection_id: profile.id.clone(),
            revision: profile.revision,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostKeyChallengeKind {
    FirstSeen,
    Changed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostKeyEvidence {
    pub(crate) algorithm: String,
    pub(crate) fingerprint: String,
}

impl HostKeyEvidence {
    pub(crate) fn new(
        algorithm: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Result<Self, RemoteSshValidationError> {
        let evidence = Self {
            algorithm: algorithm.into(),
            fingerprint: fingerprint.into(),
        };
        validate_bounded(
            &evidence.algorithm,
            MAX_ALGORITHM_BYTES,
            "host key algorithm",
        )?;
        validate_bounded(
            &evidence.fingerprint,
            MAX_FINGERPRINT_BYTES,
            "host key fingerprint",
        )?;
        Ok(evidence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostKeyChallenge {
    pub(crate) connection_key: RemoteSshConnectionKey,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) kind: HostKeyChallengeKind,
    pub(crate) evidence: HostKeyEvidence,
    pub(crate) previous_fingerprint: Option<String>,
}

impl HostKeyChallenge {
    pub(crate) fn validate(&self) -> Result<(), RemoteSshValidationError> {
        validate_bounded(&self.host, MAX_HOST_BYTES, "host")?;
        if self.port == 0 || self.connection_key.revision < 1 {
            return Err(RemoteSshValidationError::InvalidChallenge);
        }
        self.evidence.clone().validate()?;
        if let Some(previous) = &self.previous_fingerprint {
            validate_bounded(previous, MAX_FINGERPRINT_BYTES, "previous fingerprint")?;
        }
        Ok(())
    }
}

impl HostKeyEvidence {
    fn validate(self) -> Result<(), RemoteSshValidationError> {
        Self::new(self.algorithm, self.fingerprint).map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RemotePtyRequest {
    pub(crate) columns: u32,
    pub(crate) rows: u32,
}

impl RemotePtyRequest {
    pub(crate) fn bounded(columns: u16, rows: u16) -> Self {
        Self {
            columns: u32::from(columns.clamp(1, 500)),
            rows: u32::from(rows.clamp(1, 300)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemoteSshChannelEvent {
    Output(Vec<u8>),
    ExtendedOutput { stream: u32, content: Vec<u8> },
    ExitStatus(u32),
    ExitSignal(String),
    Eof,
    Closed,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum RemoteSshValidationError {
    #[error("SSH {0} is empty or exceeds its allowed size.")]
    InvalidBoundedField(&'static str),
    #[error("SSH host-key challenge is invalid.")]
    InvalidChallenge,
}

fn validate_bounded(
    value: &str,
    max_bytes: usize,
    field: &'static str,
) -> Result<(), RemoteSshValidationError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        Err(RemoteSshValidationError::InvalidBoundedField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_key_evidence_and_terminal_dimensions_are_bounded() {
        assert!(HostKeyEvidence::new("ssh-ed25519", "SHA256:fixture").is_ok());
        assert!(HostKeyEvidence::new("", "SHA256:fixture").is_err());
        assert!(HostKeyEvidence::new("ssh-ed25519", "x".repeat(161)).is_err());
        assert_eq!(
            RemotePtyRequest::bounded(0, u16::MAX),
            RemotePtyRequest {
                columns: 1,
                rows: 300
            }
        );
    }
}
