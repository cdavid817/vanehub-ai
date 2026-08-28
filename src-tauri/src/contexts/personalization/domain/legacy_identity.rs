use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::error::{IdentityRejection, PersonalizationDomainError};

/// Upper bound shared by both legacy identities. Both are derived from what a filename could be, so
/// they are bounded by that.
const LEGACY_IDENTITY_MAX_CHARS: usize = 255;

/// Version prefix on every source id, so a stored id declares which derivation rule produced it and
/// a future rule change cannot be mistaken for the current one.
const SOURCE_ID_VERSION: &str = "v1";

// =================================================================================================
// Compatibility addressing
// =================================================================================================

/// The address a *caller* uses when it still thinks a display name is an identity.
///
/// Under v1 the name produced the filename, and saving under an existing name replaced that file.
/// v2 breaks that — names are metadata and duplicates are legal — so this type exists purely to
/// keep the old `save(name)` contract working while the new UI is built. It answers "which v2
/// record does this old address point at", and nothing else.
///
/// Deliberately not convertible to or from `LegacySourceId`. They answer different questions and a
/// conversion between them would let one be used where the other was meant, which is exactly the
/// confusion this split exists to make impossible.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct LegacyAddressKey(String);

impl LegacyAddressKey {
    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        validate_identity(value)
            .map(Self)
            .map_err(PersonalizationDomainError::InvalidLegacyAddressKey)
    }

    /// The address a display name had under v1: its sanitized filename.
    ///
    /// Reproduces the old rule rather than inventing one, because the point is to recognize what a
    /// v1 caller would have considered the same memory. A name that could never have been a v1
    /// filename has no legacy address — correct rather than restrictive, since nothing under v1
    /// could have created it.
    pub(crate) fn from_display_name(name: &str) -> Result<Self, PersonalizationDomainError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(PersonalizationDomainError::InvalidLegacyAddressKey(
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
            return Err(PersonalizationDomainError::InvalidLegacyAddressKey(
                IdentityRejection::UnsupportedCharacter,
            ));
        }
        Self::parse(&format!("{trimmed}.md"))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LegacyAddressKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

// =================================================================================================
// Migration source identity
// =================================================================================================

/// A memory-directory-relative path, normalized and proven safe.
///
/// Relative to the application's memory root rather than absolute, so a source id survives the
/// application data directory moving and never embeds a machine path into durable state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NormalizedLegacyPath(String);

impl NormalizedLegacyPath {
    /// Normalizes and validates a path already known to be inside the memory root.
    ///
    /// Rejects rather than sanitizes: a caller that produced `..` or an absolute path has a bug,
    /// and quietly repairing it would let the bug reach the filesystem operations downstream.
    pub(crate) fn parse(relative: &str) -> Result<Self, PersonalizationDomainError> {
        let normalized = relative.trim().replace('\\', "/");
        if normalized.is_empty() {
            return Err(PersonalizationDomainError::InvalidLegacySourcePath(
                IdentityRejection::Empty,
            ));
        }
        if normalized.chars().count() > LEGACY_IDENTITY_MAX_CHARS {
            return Err(PersonalizationDomainError::InvalidLegacySourcePath(
                IdentityRejection::TooLong {
                    limit: LEGACY_IDENTITY_MAX_CHARS,
                },
            ));
        }
        if normalized.chars().any(char::is_control) {
            return Err(PersonalizationDomainError::InvalidLegacySourcePath(
                IdentityRejection::ContainsControlCharacter,
            ));
        }
        // Absolute paths and traversal are refused outright. The memory directory is flat by
        // design, so neither can describe an entry this migration owns.
        if normalized.starts_with('/')
            || normalized.contains(':')
            || normalized
                .split('/')
                .any(|part| part == ".." || part == ".")
        {
            return Err(PersonalizationDomainError::InvalidLegacySourcePath(
                IdentityRejection::ContainsSeparator,
            ));
        }
        Ok(Self(normalized))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NormalizedLegacyPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Which legacy store a row came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LegacyTableKind {
    /// The pre-file memory row store.
    AgentMemories,
}

impl LegacyTableKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AgentMemories => "agent_memories",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        match value {
            "agent_memories" => Ok(Self::AgentMemories),
            other => Err(PersonalizationDomainError::UnknownLegacyTableKind(
                other.to_string(),
            )),
        }
    }
}

/// Where a migration source actually was, as discovered by enumeration.
///
/// This is the thing migration is idempotent about. It comes from what the scan found, never from
/// what a file's frontmatter claims its name is: a file's `name` can disagree with its filename,
/// two files can carry the same name, and a malformed file has no readable name at all.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LegacySourceLocator {
    MarkdownFile {
        normalized_relative_path: NormalizedLegacyPath,
    },
    SqliteRow {
        table: LegacyTableKind,
        row_id: String,
    },
}

impl LegacySourceLocator {
    pub(crate) fn markdown(relative: &str) -> Result<Self, PersonalizationDomainError> {
        Ok(Self::MarkdownFile {
            normalized_relative_path: NormalizedLegacyPath::parse(relative)?,
        })
    }

    pub(crate) fn sqlite_row(
        table: LegacyTableKind,
        row_id: &str,
    ) -> Result<Self, PersonalizationDomainError> {
        let row_id = row_id.trim();
        if row_id.is_empty() {
            return Err(PersonalizationDomainError::InvalidLegacySourcePath(
                IdentityRejection::Empty,
            ));
        }
        Ok(Self::SqliteRow {
            table,
            row_id: row_id.to_string(),
        })
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::MarkdownFile { .. } => "file",
            Self::SqliteRow { .. } => "row",
        }
    }

    /// The stable id this locator produces.
    ///
    /// Includes the version prefix and the source kind, so a file and a row can never collide even
    /// if their remaining parts happened to render identically.
    pub(crate) fn source_id(&self) -> LegacySourceId {
        let rendered = match self {
            Self::MarkdownFile {
                normalized_relative_path,
            } => format!(
                "{SOURCE_ID_VERSION}:{}:{normalized_relative_path}",
                self.kind()
            ),
            Self::SqliteRow { table, row_id } => format!(
                "{SOURCE_ID_VERSION}:{}:{}:{row_id}",
                self.kind(),
                table.as_str()
            ),
        };
        LegacySourceId(rendered)
    }
}

/// The idempotency key migration keys on.
///
/// Derived from the locator alone. Deliberately independent of content: a source whose bytes change
/// between two runs is still the same source, and re-importing it as a second record would be data
/// duplication rather than data safety. Content change is the fingerprint's job.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct LegacySourceId(String);

impl LegacySourceId {
    /// Rebuilds an id from its persisted form. Validation only — the id is opaque to readers, and
    /// the locator that produced it is what carries meaning.
    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::Empty,
            ));
        }
        if trimmed.chars().count() > LEGACY_IDENTITY_MAX_CHARS {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::TooLong {
                    limit: LEGACY_IDENTITY_MAX_CHARS,
                },
            ));
        }
        if trimmed.chars().any(char::is_control) {
            return Err(PersonalizationDomainError::InvalidLegacySourceId(
                IdentityRejection::ContainsControlCharacter,
            ));
        }
        Ok(Self(trimmed.to_string()))
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

/// What a source's raw bytes looked like when it was discovered.
///
/// Raw bytes, not the v2 semantic content hash. The two are different values on purpose: a v2 body
/// is LF-normalized before hashing, so a CRLF source and its migrated record legitimately disagree.
/// Only this value can answer "is the file still byte-identical to what I read", which is what a
/// backup verification and a pre-delete recheck need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacySourceFingerprint {
    pub(crate) raw_sha256: String,
    pub(crate) byte_length: u64,
}

impl LegacySourceFingerprint {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        Self {
            raw_sha256: digest.iter().map(|byte| format!("{byte:02x}")).collect(),
            byte_length: bytes.len() as u64,
        }
    }

    /// Length is compared as well as the digest. A digest match with a different length would mean
    /// something is badly wrong, and noticing costs nothing.
    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.raw_sha256 == other.raw_sha256 && self.byte_length == other.byte_length
    }
}

fn validate_identity(value: &str) -> Result<String, IdentityRejection> {
    if value.is_empty() {
        return Err(IdentityRejection::Empty);
    }
    if value != value.trim() {
        return Err(IdentityRejection::NotTrimmed);
    }
    if value.chars().count() > LEGACY_IDENTITY_MAX_CHARS {
        return Err(IdentityRejection::TooLong {
            limit: LEGACY_IDENTITY_MAX_CHARS,
        });
    }
    if value.contains('/') || value.contains('\\') {
        return Err(IdentityRejection::ContainsSeparator);
    }
    if value.chars().any(char::is_control) {
        return Err(IdentityRejection::ContainsControlCharacter);
    }
    Ok(value.to_string())
}

// =================================================================================================
// Migration journal
// =================================================================================================

/// How far one legacy source has progressed.
///
/// Ordered, and every transition is persisted before the step it authorizes. That is what makes an
/// interrupted migration resumable rather than restartable: the next run reads the stage and knows
/// which side of each irreversible action it stopped on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MigrationStage {
    /// Enumerated and journalled. Nothing written, nothing copied.
    Discovered,
    /// A byte-for-byte backup exists. Not yet proven to match.
    BackupWritten,
    /// The backup was re-read and its raw hash and length match the source.
    BackupVerified,
    /// The v2 file exists. Not yet proven readable.
    V2Written,
    /// The v2 file was re-read and its revision, semantic content hash, and provenance check out.
    V2Verified,
    /// The projection row exists.
    ProjectionWritten,
    /// The legacy source has been removed. Only reachable after backup and v2 verification.
    LegacyRemoved,
    /// Derived views still need rebuilding for this record.
    DerivedPending,
    Completed,
    /// Terminal for this source; other sources continue.
    Failed,
    /// The source changed between discovery and a checkpoint. Nothing was overwritten or deleted.
    SourceChanged,
}

impl MigrationStage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::BackupWritten => "backup_written",
            Self::BackupVerified => "backup_verified",
            Self::V2Written => "v2_written",
            Self::V2Verified => "v2_verified",
            Self::ProjectionWritten => "projection_written",
            Self::LegacyRemoved => "legacy_removed",
            Self::DerivedPending => "derived_pending",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::SourceChanged => "source_changed",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        match value {
            "discovered" => Ok(Self::Discovered),
            "backup_written" => Ok(Self::BackupWritten),
            "backup_verified" => Ok(Self::BackupVerified),
            "v2_written" => Ok(Self::V2Written),
            "v2_verified" => Ok(Self::V2Verified),
            "projection_written" => Ok(Self::ProjectionWritten),
            "legacy_removed" => Ok(Self::LegacyRemoved),
            "derived_pending" => Ok(Self::DerivedPending),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "source_changed" => Ok(Self::SourceChanged),
            other => Err(PersonalizationDomainError::UnknownMigrationStage(
                other.to_string(),
            )),
        }
    }

    /// Whether the v2 record for this source is safe for an ordinary reader to address.
    ///
    /// `V2Written` deliberately is not: the file exists but has not been proven readable, and
    /// handing a caller something that might be torn is worse than reporting it absent. A resume
    /// path reaches it by journal id instead, which is what stops an unverified record from being
    /// permanently unreachable.
    pub(crate) fn has_usable_memory(self) -> bool {
        matches!(
            self,
            Self::V2Verified
                | Self::ProjectionWritten
                | Self::LegacyRemoved
                | Self::DerivedPending
                | Self::Completed
        )
    }

    /// Whether a resume run should pick this entry up again.
    pub(crate) fn is_resumable(self) -> bool {
        !matches!(self, Self::Completed | Self::Failed | Self::SourceChanged)
    }

    /// Whether the legacy source may still exist on disk at this stage.
    pub(crate) fn legacy_source_may_exist(self) -> bool {
        self < Self::LegacyRemoved || matches!(self, Self::Failed | Self::SourceChanged)
    }
}

/// One journal row: a source, where it got to, what it became, and where its backup is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrationJournalEntry {
    pub(crate) source_id: LegacySourceId,
    pub(crate) locator: LegacySourceLocator,
    pub(crate) target_memory_id: Option<super::MemoryId>,
    pub(crate) stage: MigrationStage,
    /// Where the raw bytes were copied before any removal. Persisted before the removal it
    /// authorizes, so a rollback never has to hope the original is still there.
    pub(crate) backup_relative_path: Option<String>,
    pub(crate) source_fingerprint: Option<LegacySourceFingerprint>,
    pub(crate) last_error_code: Option<String>,
}
