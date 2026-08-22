// Landed ahead of its consumer: `ExtensionManifestV1Decoder` (Task 1.E) is what reads these, and
// the types are validated by their own tests until then. Same idiom `skills`' `config_document`
// uses. Remove when the decoder lands.
#![cfg_attr(not(test), allow(dead_code))]

//! Validated identities for extensions, their packages, and their contributions.
//!
//! Every type here is a newtype with a fallible constructor. Application services take these
//! rather than `&str` so that "is this a real extension id?" is answered once, at the boundary,
//! instead of at each use — and so a package hash can never be passed where an installation id
//! belongs.

use super::{ExtensionDomainError, IdentifierKind};

/// Longest an extension id may be, counting the publisher, the dot, and the name.
pub(crate) const MAX_EXTENSION_ID_CHARACTERS: usize = 128;
/// Shortest an extension id may be. `a.b` is the smallest thing that has both halves.
pub(crate) const MIN_EXTENSION_ID_CHARACTERS: usize = 3;
const MAX_SEGMENT_CHARACTERS: usize = 64;
const MAX_CONTRIBUTION_LOCAL_ID_CHARACTERS: usize = 64;
const MAX_OPAQUE_ID_CHARACTERS: usize = 128;
/// SHA-256, rendered lower-case hex.
const PACKAGE_HASH_CHARACTERS: usize = 64;
/// Prefix that marks a contribution as external. Native ids never carry it, which is what makes
/// "an extension may not claim a native id" mechanically checkable.
pub(crate) const EXTERNAL_CONTRIBUTION_PREFIX: &str = "ext";
const GLOBAL_ID_SEPARATOR: &str = "::";

/// Truncates untrusted text before it enters a diagnostic, so a hostile manifest cannot make the
/// rejection itself unbounded.
fn bounded(value: &str) -> String {
    value.chars().take(MAX_EXTENSION_ID_CHARACTERS).collect()
}

/// One dot-separated half of an extension id: lower-case ASCII alphanumerics and inner dashes.
///
/// Leading and trailing dashes are rejected rather than trimmed. `acme-` and `acme` must not both
/// resolve to the same publisher, or two packages could disagree about who signed them.
fn is_id_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SEGMENT_CHARACTERS
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

/// A contribution's manifest-local id. Same rule as an id segment, plus underscores: the tool
/// names in circulation use them (`git_status`) and rejecting them would be a gratuitous break.
fn is_contribution_local_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CONTRIBUTION_LOCAL_ID_CHARACTERS
        && !value.starts_with(['-', '_'])
        && !value.ends_with(['-', '_'])
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
                || character == '_'
        })
}

/// Application-generated opaque identifier. Not parsed from a manifest, so the rule only has to
/// exclude what would break a log line, a path, or a URL.
fn is_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_OPAQUE_ID_CHARACTERS
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

/// The publishing organisation, as declared in the manifest and as carried by a signing key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PublisherId(String);

impl PublisherId {
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
        if is_id_segment(value) {
            Ok(Self(value.to_string()))
        } else {
            Err(ExtensionDomainError::new(
                IdentifierKind::Publisher,
                bounded(value),
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// `<publisher>.<name>`.
///
/// Stored whole and re-derived on demand rather than kept as two fields, because every consumer
/// wants the joined form and a split representation invites two sources of truth for the same
/// string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ExtensionId(String);

impl ExtensionId {
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
        let invalid = || ExtensionDomainError::new(IdentifierKind::Extension, bounded(value));
        if value.len() < MIN_EXTENSION_ID_CHARACTERS || value.len() > MAX_EXTENSION_ID_CHARACTERS {
            return Err(invalid());
        }
        // Exactly one dot: `acme.tools.git` would make "which part is the publisher?" ambiguous,
        // and publisher trust is decided on that answer.
        let mut halves = value.split('.');
        let (Some(publisher), Some(name), None) = (halves.next(), halves.next(), halves.next())
        else {
            return Err(invalid());
        };
        if !is_id_segment(publisher) || !is_id_segment(name) {
            return Err(invalid());
        }
        Ok(Self(value.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// The publisher half. Infallible: the id was validated as two valid segments.
    pub(crate) fn publisher(&self) -> PublisherId {
        PublisherId(self.0.split('.').next().unwrap_or(&self.0).to_string())
    }

    /// The name half.
    pub(crate) fn name(&self) -> &str {
        self.0.split('.').nth(1).unwrap_or(&self.0)
    }
}

/// What a contribution contributes. Closed, because a global id is built from it and an unknown
/// kind would produce an id no adapter could ever route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ContributionKind {
    Tool,
    Skill,
    Mcp,
    Mode,
    Hook,
    Rule,
    Connector,
    Configuration,
    Transform,
}

pub(crate) const ALL_CONTRIBUTION_KINDS: [ContributionKind; 9] = [
    ContributionKind::Tool,
    ContributionKind::Skill,
    ContributionKind::Mcp,
    ContributionKind::Mode,
    ContributionKind::Hook,
    ContributionKind::Rule,
    ContributionKind::Connector,
    ContributionKind::Configuration,
    ContributionKind::Transform,
];

impl ContributionKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Skill => "skill",
            Self::Mcp => "mcp",
            Self::Mode => "mode",
            Self::Hook => "hook",
            Self::Rule => "rule",
            Self::Connector => "connector",
            Self::Configuration => "configuration",
            Self::Transform => "transform",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_CONTRIBUTION_KINDS
            .into_iter()
            .find(|kind| kind.as_str() == value)
    }
}

/// A contribution's id as written in its own manifest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ContributionLocalId(String);

impl ContributionLocalId {
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
        if is_contribution_local_id(value) {
            Ok(Self(value.to_string()))
        } else {
            Err(ExtensionDomainError::new(
                IdentifierKind::Contribution,
                bounded(value),
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// `ext::<extension-id>::<kind>::<local-id>`.
///
/// Always derived, never taken from a manifest. An extension cannot name its own global id, which
/// is what stops it claiming a native tool's identity — the prefix is not something it can write.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ContributionGlobalId(String);

impl ContributionGlobalId {
    pub(crate) fn new(
        extension: &ExtensionId,
        kind: ContributionKind,
        local: &ContributionLocalId,
    ) -> Self {
        Self(
            [
                EXTERNAL_CONTRIBUTION_PREFIX,
                extension.as_str(),
                kind.as_str(),
                local.as_str(),
            ]
            .join(GLOBAL_ID_SEPARATOR),
        )
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Round-trips a stored id. Used when reading persisted provenance, not when reading a
    /// manifest.
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
        let invalid = || ExtensionDomainError::new(IdentifierKind::Contribution, bounded(value));
        let parts: Vec<&str> = value.split(GLOBAL_ID_SEPARATOR).collect();
        let [prefix, extension, kind, local] = parts.as_slice() else {
            return Err(invalid());
        };
        if *prefix != EXTERNAL_CONTRIBUTION_PREFIX {
            return Err(invalid());
        }
        let extension = ExtensionId::parse(extension).map_err(|_| invalid())?;
        let kind = ContributionKind::parse(kind).ok_or_else(invalid)?;
        let local = ContributionLocalId::parse(local).map_err(|_| invalid())?;
        Ok(Self::new(&extension, kind, &local))
    }
}

/// Whether an identifier is one an extension is forbidden to claim.
///
/// A native id is anything without the external prefix. Checked on the id a manifest *writes*, so
/// a package declaring `shell` as a contribution id is rejected before namespacing hides the
/// collision.
pub(crate) fn is_external_contribution_id(value: &str) -> bool {
    value.starts_with(&format!(
        "{EXTERNAL_CONTRIBUTION_PREFIX}{GLOBAL_ID_SEPARATOR}"
    ))
}

/// SHA-256 of the package bytes, lower-case hex.
///
/// Case is part of the rule: the same digest written two ways would compare unequal and defeat
/// content addressing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PackageHash(String);

impl PackageHash {
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
        let valid = value.len() == PACKAGE_HASH_CHARACTERS
            && value
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));
        if valid {
            Ok(Self(value.to_string()))
        } else {
            Err(ExtensionDomainError::new(
                IdentifierKind::PackageHash,
                bounded(value),
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Application-generated identifiers.
///
/// Four types with one rule. Written as a macro rather than four copies so that a later change to
/// the rule cannot land on three of them: the alternative is 150 lines whose only differences are
/// the type name and the identifier kind.
macro_rules! opaque_identifier {
    ($(#[$doc:meta])* $name:ident, $kind:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
                if is_opaque_id(value) {
                    Ok(Self(value.to_string()))
                } else {
                    Err(ExtensionDomainError::new(
                        IdentifierKind::$kind,
                        bounded(value),
                    ))
                }
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

opaque_identifier!(
    /// One immutable set of package bytes on disk.
    SnapshotId,
    Snapshot
);
opaque_identifier!(
    /// One installed extension, across all the snapshots it has pointed at.
    InstallationId,
    Installation
);
opaque_identifier!(
    /// One activation of an installation's runtime. Pinned by in-flight calls.
    RuntimeGenerationId,
    RuntimeGeneration
);
opaque_identifier!(
    /// Binds a previewed operation to the state it was previewed against.
    OperationWitness,
    OperationWitness
);
