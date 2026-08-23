// The connect and execute paths that consume these land with the Connector Lifecycle task group;
// Task Group 3 lands the storage they are written through.
#![cfg_attr(not(test), allow(dead_code))]

//! Validated identities for connector subjects, instances, bindings, and what an instance holds.
//!
//! Four of these carry weight beyond tidiness.
//!
//! **`DisplayLabel` and `LabelKey` are separate.** The label a person typed is theirs — case,
//! spacing, and all. Uniqueness is decided on a normalised key, so `Acme Prod` and `acme  prod`
//! cannot both exist and confuse whoever picks one from a list. Normalising the stored label
//! instead would rewrite what the user wrote; keying on the raw label instead would admit the
//! confusable pair. **Neither is identity** — that is `instance_id`, so renaming an instance keeps
//! every binding and its credential.
//!
//! **`CredentialHandle` is opaque and never a secret.** It names an entry in the OS credential
//! store. SQLite holds the handle; the store holds the secret; and nothing that crosses a DTO, a
//! log line, or an audit row holds either.
//!
//! **`PublicConfiguration` refuses secret-shaped keys.** Not a value scanner — a name check at the
//! one boundary where the name is reliable, because a field a definition declared *public* is by
//! construction never the one called `api_key`. It catches the specific mistake of pasting a token
//! into the visible settings form.
//!
//! **`ConnectorTarget` is a kind plus a key.** SQLite treats `NULL` as distinct from every other
//! `NULL` in a unique index, so a nullable target would admit unlimited global bindings for one
//! instance, each invisible to the others.

const MAX_GLOBAL_ID_CHARACTERS: usize = 160;
const MAX_OPAQUE_ID_CHARACTERS: usize = 128;
const MAX_LABEL_CHARACTERS: usize = 96;
const MAX_TARGET_KEY_CHARACTERS: usize = 256;
const MAX_CONFIGURATION_KEY_CHARACTERS: usize = 64;
const MAX_CONFIGURATION_VALUE_CHARACTERS: usize = 2_048;
const MAX_CONFIGURATION_ENTRIES: usize = 64;
/// SHA-256, rendered lower-case hex.
const DIGEST_CHARACTERS: usize = 64;

/// Which identity failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ConnectorIdentifierKind {
    ConnectorGlobal,
    OwnerExtension,
    SnapshotRef,
    Instance,
    Binding,
    DefinitionDigest,
    CredentialHandle,
    DisplayLabel,
    TargetKind,
    TargetKey,
    ConfigurationKey,
    ConfigurationValue,
    SecretShapedConfiguration,
}

impl ConnectorIdentifierKind {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::ConnectorGlobal => "invalid_connector_global_id",
            Self::OwnerExtension => "invalid_connector_owner_extension",
            Self::SnapshotRef => "invalid_connector_snapshot_ref",
            Self::Instance => "invalid_connector_instance_id",
            Self::Binding => "invalid_connector_binding_id",
            Self::DefinitionDigest => "invalid_connector_definition_digest",
            Self::CredentialHandle => "invalid_connector_credential_handle",
            Self::DisplayLabel => "invalid_connector_display_label",
            Self::TargetKind => "invalid_connector_target_kind",
            Self::TargetKey => "invalid_connector_target_key",
            Self::ConfigurationKey => "invalid_connector_configuration_key",
            Self::ConfigurationValue => "invalid_connector_configuration_value",
            Self::SecretShapedConfiguration => "secret_shaped_public_configuration",
        }
    }
}

pub(crate) const ALL_CONNECTOR_IDENTIFIER_KINDS: &[ConnectorIdentifierKind] = &[
    ConnectorIdentifierKind::ConnectorGlobal,
    ConnectorIdentifierKind::OwnerExtension,
    ConnectorIdentifierKind::SnapshotRef,
    ConnectorIdentifierKind::Instance,
    ConnectorIdentifierKind::Binding,
    ConnectorIdentifierKind::DefinitionDigest,
    ConnectorIdentifierKind::CredentialHandle,
    ConnectorIdentifierKind::DisplayLabel,
    ConnectorIdentifierKind::TargetKind,
    ConnectorIdentifierKind::TargetKey,
    ConnectorIdentifierKind::ConfigurationKey,
    ConnectorIdentifierKind::ConfigurationValue,
    ConnectorIdentifierKind::SecretShapedConfiguration,
];

/// Why a value could not be read as an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorIdentityError {
    pub(crate) kind: ConnectorIdentifierKind,
    pub(crate) value: String,
}

impl ConnectorIdentityError {
    pub(super) fn new(kind: ConnectorIdentifierKind, value: &str) -> Self {
        Self {
            kind,
            // Bounded, so a hostile value cannot make the diagnostic itself unbounded.
            value: value.chars().take(MAX_GLOBAL_ID_CHARACTERS).collect(),
        }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.kind.code()
    }
}

fn is_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_OPAQUE_ID_CHARACTERS
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

macro_rules! opaque_identifier {
    ($name:ident, $kind:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: &str) -> Result<Self, ConnectorIdentityError> {
                if is_opaque_id(value) {
                    Ok(Self(value.to_string()))
                } else {
                    Err(ConnectorIdentityError::new($kind, value))
                }
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

opaque_identifier!(ConnectorSnapshotRef, ConnectorIdentifierKind::SnapshotRef);
opaque_identifier!(InstanceId, ConnectorIdentifierKind::Instance);
opaque_identifier!(BindingId, ConnectorIdentifierKind::Binding);

/// The stable identity of one connector, for as long as any evidence mentions it.
///
/// Namespaced by its extension, so the grammar admits `:` and `.`. Validated as shape only — what
/// the segments mean is `extension_platform`'s business.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ConnectorGlobalId(String);

impl ConnectorGlobalId {
    pub(crate) fn parse(value: &str) -> Result<Self, ConnectorIdentityError> {
        let acceptable = |character: char| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_' | '.' | ':')
        };
        if value.is_empty()
            || value.len() > MAX_GLOBAL_ID_CHARACTERS
            || value.starts_with(['-', '_', '.', ':'])
            || value.ends_with(['-', '_', '.', ':'])
            || !value.chars().all(acceptable)
        {
            return Err(ConnectorIdentityError::new(
                ConnectorIdentifierKind::ConnectorGlobal,
                value,
            ));
        }
        Ok(Self(value.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The extension that contributes a connector, as named by something outside this subdomain.
///
/// Opaque text. `extension_platform` owns extensions; recording which one owns a subject is what
/// lets an operator find the package to uninstall, and nothing more.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OwnerExtensionId(String);

impl OwnerExtensionId {
    pub(crate) fn parse(value: &str) -> Result<Self, ConnectorIdentityError> {
        let acceptable = |character: char| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '.')
        };
        if value.is_empty()
            || value.len() > MAX_GLOBAL_ID_CHARACTERS
            || value.starts_with(['-', '.'])
            || value.ends_with(['-', '.'])
            || !value.chars().all(acceptable)
        {
            return Err(ConnectorIdentityError::new(
                ConnectorIdentifierKind::OwnerExtension,
                value,
            ));
        }
        Ok(Self(value.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The digest of a definition's canonical form.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ConnectorDefinitionDigest(String);

impl ConnectorDefinitionDigest {
    pub(crate) fn parse(value: &str) -> Result<Self, ConnectorIdentityError> {
        if value.len() == DIGEST_CHARACTERS
            && value
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
        {
            Ok(Self(value.to_string()))
        } else {
            Err(ConnectorIdentityError::new(
                ConnectorIdentifierKind::DefinitionDigest,
                value,
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Names an entry in the OS credential store. Never the secret itself.
///
/// Host-generated: it is not derived from the label, the instance id, or anything a person typed,
/// so a handle that leaked into a log would still not say which account it belongs to.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CredentialHandle(String);

impl CredentialHandle {
    pub(crate) fn parse(value: &str) -> Result<Self, ConnectorIdentityError> {
        if is_opaque_id(value) {
            Ok(Self(value.to_string()))
        } else {
            Err(ConnectorIdentityError::new(
                ConnectorIdentifierKind::CredentialHandle,
                value,
            ))
        }
    }

    /// Deliberately not `as_str`. A handle leaves this type only for the credential store and for
    /// the repository column, and both go through here, so every other use has to be written down
    /// as one.
    pub(crate) fn expose_for_storage(&self) -> &str {
        &self.0
    }
}

/// Redacted. A handle is not a secret, but printing one turns every log line into a map of which
/// credential-store entries exist.
impl std::fmt::Debug for CredentialHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CredentialHandle(<redacted>)")
    }
}

/// What a person called an instance, as they typed it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DisplayLabel(String);

/// The normalised form uniqueness is decided on. Not identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LabelKey(String);

impl DisplayLabel {
    pub(crate) fn parse(value: &str) -> Result<Self, ConnectorIdentityError> {
        let refuse = || ConnectorIdentityError::new(ConnectorIdentifierKind::DisplayLabel, value);
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.chars().count() > MAX_LABEL_CHARACTERS
            || trimmed
                .chars()
                .any(|character| character == '\0' || (character.is_control() && character != '\t'))
        {
            return Err(refuse());
        }
        Ok(Self(trimmed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Case-folded with runs of whitespace collapsed to one space.
    ///
    /// Enough to stop the confusable pairs a person actually produces — a stray double space, a
    /// different capitalisation — without pretending to solve Unicode confusables, which would
    /// need a full skeleton algorithm and is a different problem.
    pub(crate) fn key(&self) -> LabelKey {
        let mut key = String::with_capacity(self.0.len());
        let mut pending_space = false;
        for character in self.0.chars() {
            if character.is_whitespace() {
                pending_space = !key.is_empty();
                continue;
            }
            if pending_space {
                key.push(' ');
                pending_space = false;
            }
            key.extend(character.to_lowercase());
        }
        LabelKey(key)
    }
}

impl LabelKey {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a binding applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum TargetKind {
    Global,
    Project,
    Agent,
    Session,
}

impl TargetKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
            Self::Agent => "agent",
            Self::Session => "session",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_TARGET_KINDS
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
    }
}

pub(crate) const ALL_TARGET_KINDS: &[TargetKind] = &[
    TargetKind::Global,
    TargetKind::Project,
    TargetKind::Agent,
    TargetKind::Session,
];

/// The global target's key. Empty rather than absent, so the unique index is total.
pub(crate) const GLOBAL_TARGET_KEY: &str = "";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ConnectorTarget {
    kind: TargetKind,
    key: String,
}

impl ConnectorTarget {
    pub(crate) fn global() -> Self {
        Self {
            kind: TargetKind::Global,
            key: GLOBAL_TARGET_KEY.to_string(),
        }
    }

    pub(crate) fn scoped(kind: TargetKind, key: &str) -> Result<Self, ConnectorIdentityError> {
        let refuse = || ConnectorIdentityError::new(ConnectorIdentifierKind::TargetKey, key);
        if kind == TargetKind::Global
            || key.is_empty()
            || key.len() > MAX_TARGET_KEY_CHARACTERS
            || key.contains('\0')
        {
            return Err(refuse());
        }
        Ok(Self {
            kind,
            key: key.to_string(),
        })
    }

    /// Rebuilds a target from the two columns it is stored as, refusing any pair the constructors
    /// could not have produced.
    pub(crate) fn parse(kind: &str, key: &str) -> Result<Self, ConnectorIdentityError> {
        let Some(kind) = TargetKind::parse(kind) else {
            return Err(ConnectorIdentityError::new(
                ConnectorIdentifierKind::TargetKind,
                kind,
            ));
        };
        match kind {
            TargetKind::Global if key == GLOBAL_TARGET_KEY => Ok(Self::global()),
            TargetKind::Global => Err(ConnectorIdentityError::new(
                ConnectorIdentifierKind::TargetKey,
                key,
            )),
            other => Self::scoped(other, key),
        }
    }

    pub(crate) const fn kind(&self) -> TargetKind {
        self.kind
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

/// Configuration field names that a *public* configuration never legitimately has.
///
/// A name check, not a value scanner: at this one boundary the name is reliable, because a field
/// the definition declared public is by construction not the one called `api_key`. It catches the
/// specific mistake this column invites — pasting a token into the visible settings form.
const SECRET_SHAPED_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "authorization",
    "auth_token",
    "access_token",
    "client_secret",
    "credential",
    "password",
    "passwd",
    "private_key",
    "refresh_token",
    "secret",
    "session_key",
    "token",
];

/// The non-secret settings of one instance.
///
/// Ordered and deduplicated by key, so the stored form does not depend on the order a form
/// submitted its fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PublicConfiguration(Vec<(String, String)>);

impl PublicConfiguration {
    pub(crate) fn empty() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn of(entries: &[(&str, &str)]) -> Result<Self, ConnectorIdentityError> {
        if entries.len() > MAX_CONFIGURATION_ENTRIES {
            return Err(ConnectorIdentityError::new(
                ConnectorIdentifierKind::ConfigurationKey,
                "too many entries",
            ));
        }
        let mut ordered: Vec<(String, String)> = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            let key = normalise_configuration_key(key)?;
            if value.len() > MAX_CONFIGURATION_VALUE_CHARACTERS || value.contains('\0') {
                return Err(ConnectorIdentityError::new(
                    ConnectorIdentifierKind::ConfigurationValue,
                    value,
                ));
            }
            ordered.push((key, (*value).to_string()));
        }
        ordered.sort_by(|left, right| left.0.cmp(&right.0));
        ordered.dedup_by(|left, right| left.0 == right.0);
        Ok(Self(ordered))
    }

    /// The stored form: `key=value` pairs, newline separated. Values cannot contain a newline
    /// because they cannot contain a control character.
    pub(crate) fn as_str(&self) -> String {
        self.0
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ConnectorIdentityError> {
        if value.is_empty() {
            return Ok(Self::empty());
        }
        let mut entries = Vec::new();
        for line in value.split('\n') {
            let Some((key, item)) = line.split_once('=') else {
                return Err(ConnectorIdentityError::new(
                    ConnectorIdentifierKind::ConfigurationKey,
                    line,
                ));
            };
            entries.push((key.to_string(), item.to_string()));
        }
        let borrowed: Vec<(&str, &str)> = entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        Self::of(&borrowed)
    }

    pub(crate) fn entries(&self) -> &[(String, String)] {
        &self.0
    }
}

fn normalise_configuration_key(key: &str) -> Result<String, ConnectorIdentityError> {
    let acceptable = |character: char| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    };
    if key.is_empty()
        || key.len() > MAX_CONFIGURATION_KEY_CHARACTERS
        || key.starts_with('_')
        || key.ends_with('_')
        || !key.chars().all(acceptable)
    {
        return Err(ConnectorIdentityError::new(
            ConnectorIdentifierKind::ConfigurationKey,
            key,
        ));
    }
    if SECRET_SHAPED_KEYS.contains(&key) {
        return Err(ConnectorIdentityError::new(
            ConnectorIdentifierKind::SecretShapedConfiguration,
            key,
        ));
    }
    Ok(key.to_string())
}
