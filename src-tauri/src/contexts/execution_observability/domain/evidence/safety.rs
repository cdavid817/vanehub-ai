use super::error::EvidenceDomainError;
use super::identity::MAX_BASENAME_LENGTH;

pub(crate) const MAX_REDACTED_DISPLAY_BYTES: usize = 2 * 1024;
pub(crate) const MAX_DISPLAY_PATH_LENGTH: usize = 256;

/// Structural rejections, applied on top of the allowlisted payload rather than instead of it.
///
/// The payload enum is the real boundary: a producer cannot attach a field the enum has no variant
/// for, so there is no channel through which a prompt, a diff, or a tool result can arrive at all.
/// What is left are the few fields that legitimately carry short free text, and the checks below
/// are about the shapes those fields must never take. They are deliberately structural — a path
/// separator, a drive letter, a line break, a PEM header — because pattern-matching a secret is a
/// losing game and treating it as the primary defence is how content leaks in the first place.
fn contains_path_separator(value: &str) -> bool {
    value.contains('/') || value.contains('\\')
}

/// Rejects a path that names a location on the machine rather than a location in the workspace.
/// `\\?\` and UNC forms are covered by the separator check that precedes the drive-letter test.
pub(crate) fn reject_absolute_path(value: &str) -> Result<(), EvidenceDomainError> {
    let bytes = value.as_bytes();
    let windows_drive = bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes
            .get(2)
            .is_none_or(|byte| *byte == b'/' || *byte == b'\\');
    let unix_rooted = value.starts_with('/') || value.starts_with('~');
    let unc = value.starts_with("\\\\");
    if windows_drive || unix_rooted || unc {
        return Err(EvidenceDomainError::AbsolutePathRejected);
    }
    Ok(())
}

/// A PEM block or an authorization header in a display string means the producer sent something it
/// was supposed to have redacted. Refusing is safer than storing it and hoping a later sink masks
/// it, because the journal is itself a sink.
fn reject_credential_shape(value: &str) -> Result<(), EvidenceDomainError> {
    const MARKERS: [&str; 4] = [
        "-----BEGIN",
        "authorization:",
        "x-api-key:",
        "aws_secret_access_key",
    ];
    let lowered = value.to_ascii_lowercase();
    if MARKERS
        .iter()
        .any(|marker| lowered.contains(&marker.to_ascii_lowercase()))
    {
        return Err(EvidenceDomainError::CredentialShapedContentRejected);
    }
    Ok(())
}

/// The file name alone, never the path that leads to it. A separator here would turn evidence into
/// a map of the user's disk; the fingerprint field carries identity instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SafeBasename(String);

impl SafeBasename {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, EvidenceDomainError> {
        let value = value.into();
        if value.is_empty()
            || value.chars().count() > MAX_BASENAME_LENGTH
            || value.chars().any(|character| character.is_control())
        {
            return Err(EvidenceDomainError::InvalidLabel {
                field: "basename",
                max: MAX_BASENAME_LENGTH,
            });
        }
        if contains_path_separator(&value) {
            return Err(EvidenceDomainError::PathSeparatorRejected);
        }
        reject_absolute_path(&value)?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A workspace-relative directory shown next to a command, already elided by its producer.
/// Relative by construction: an absolute one identifies the machine, not the work.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RelativeDisplayPath(String);

impl RelativeDisplayPath {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, EvidenceDomainError> {
        let value = value.into();
        if value.is_empty()
            || value.chars().count() > MAX_DISPLAY_PATH_LENGTH
            || value.chars().any(|character| character.is_control())
        {
            return Err(EvidenceDomainError::InvalidLabel {
                field: "working directory display",
                max: MAX_DISPLAY_PATH_LENGTH,
            });
        }
        reject_absolute_path(&value)?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A bounded single-line command summary the producer has already redacted.
///
/// Single line is the load-bearing rule. Multi-line content is a terminal transcript, and a
/// transcript is precisely what the journal must never hold; the bound then keeps even a
/// legitimate one-liner from becoming an argument dump.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RedactedCommandDisplay(String);

impl RedactedCommandDisplay {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, EvidenceDomainError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_REDACTED_DISPLAY_BYTES
            || value.chars().any(|character| character.is_control())
        {
            return Err(EvidenceDomainError::InvalidRedactedDisplay {
                max: MAX_REDACTED_DISPLAY_BYTES,
            });
        }
        reject_credential_shape(&value)?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
