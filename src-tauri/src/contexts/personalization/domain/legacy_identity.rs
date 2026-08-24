use serde::{Deserialize, Serialize};

use super::error::{IdentityRejection, PersonalizationDomainError};

/// Upper bound shared with the other identities. A legacy source id is a pre-migration filename, so
/// it is bounded by what a filename could be.
const LEGACY_SOURCE_ID_MAX_CHARS: usize = 255;

/// What a memory was addressed by *before* migration.
///
/// Under v1 the display name was the identity: it produced the filename, and saving under an
/// existing name replaced that file. v2 breaks that — names are metadata and duplicates are legal —
/// so the old identity has to be recorded explicitly rather than reconstructed by searching for a
/// name, which can now match more than one record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct LegacySourceId(String);

impl LegacySourceId {
    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        if value.is_empty() {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::Empty,
            ));
        }
        if value != value.trim() {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::NotTrimmed,
            ));
        }
        if value.chars().count() > LEGACY_SOURCE_ID_MAX_CHARS {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::TooLong {
                    limit: LEGACY_SOURCE_ID_MAX_CHARS,
                },
            ));
        }
        if value.contains('/') || value.contains('\\') {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::ContainsSeparator,
            ));
        }
        if value.chars().any(char::is_control) {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::ContainsControlCharacter,
            ));
        }
        Ok(Self(value.to_string()))
    }

    /// The identity a display name had under v1: its sanitized filename.
    ///
    /// Reproduces the old rule rather than inventing one, because the whole point is to recognize
    /// records that a v1 caller would have considered the same memory. A name that could never have
    /// been a v1 filename has no legacy identity, which is correct — nothing under v1 could have
    /// created it.
    pub(crate) fn from_display_name(name: &str) -> Result<Self, PersonalizationDomainError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::Empty,
            ));
        }
        if trimmed.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
                )
        }) {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::UnsupportedCharacter,
            ));
        }
        Self::parse(&format!("{trimmed}.md"))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LegacySourceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// How far one legacy source has progressed through migration.
///
/// Ordered, and every transition is persisted before the step it authorizes. That is what makes an
/// interrupted migration resumable rather than restartable: the next run reads the stage and knows
/// exactly which side of each irreversible action it stopped on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MigrationStage {
    /// Enumerated, nothing written.
    Discovered,
    /// The manifest row exists, including the backup location. Nothing has been created or removed.
    ManifestWritten,
    /// The v2 file exists. Not yet proven readable.
    V2Written,
    /// The v2 file was re-read and its hash, revision, and provenance check out.
    V2Verified,
    /// The projection row exists.
    ProjectionWritten,
    /// The legacy source has been removed. Only reachable after verification and backup.
    LegacyRemoved,
    /// Derived views have been rebuilt for this record.
    DerivedRebuilt,
    Completed,
    /// Terminal for this source; other sources continue.
    Failed,
}

impl MigrationStage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::ManifestWritten => "manifest_written",
            Self::V2Written => "v2_written",
            Self::V2Verified => "v2_verified",
            Self::ProjectionWritten => "projection_written",
            Self::LegacyRemoved => "legacy_removed",
            Self::DerivedRebuilt => "derived_rebuilt",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        match value {
            "discovered" => Ok(Self::Discovered),
            "manifest_written" => Ok(Self::ManifestWritten),
            "v2_written" => Ok(Self::V2Written),
            "v2_verified" => Ok(Self::V2Verified),
            "projection_written" => Ok(Self::ProjectionWritten),
            "legacy_removed" => Ok(Self::LegacyRemoved),
            "derived_rebuilt" => Ok(Self::DerivedRebuilt),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            other => Err(PersonalizationDomainError::UnknownMigrationStage(
                other.to_string(),
            )),
        }
    }

    /// Whether the v2 record for this source is safe to address.
    ///
    /// `V2Written` deliberately is not: the file exists but has not been proven readable, and
    /// handing a caller a record that might be torn is worse than telling them it is not there yet.
    pub(crate) fn has_usable_memory(self) -> bool {
        matches!(
            self,
            Self::V2Verified
                | Self::ProjectionWritten
                | Self::LegacyRemoved
                | Self::DerivedRebuilt
                | Self::Completed
        )
    }
}

/// One journal row: a legacy source, where it got to, and what it became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrationJournalEntry {
    pub(crate) legacy_source_id: LegacySourceId,
    pub(crate) memory_id: Option<super::MemoryId>,
    pub(crate) stage: MigrationStage,
    /// Where the legacy file was copied before removal. Persisted before any removal, so a rollback
    /// never has to hope the original is still there.
    pub(crate) legacy_backup_path: Option<String>,
    pub(crate) legacy_content_hash: Option<String>,
    pub(crate) last_error_code: Option<String>,
}
