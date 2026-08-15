use super::SkillToolDomainError;

/// Longest provider-visible tool name any supported interface format accepts. Anthropic, OpenAI,
/// and Gemini all cap tool names at 64 characters, so the canonical name is rejected here rather
/// than silently truncated into a colliding name at catalog-assembly time.
pub(crate) const MAX_CANONICAL_NAME_CHARACTERS: usize = 64;
/// Hex characters of the revision witness carried in the canonical name. Long enough that two
/// concurrently registered revisions of the same tool do not collide, short enough to leave room
/// for real ids inside `MAX_CANONICAL_NAME_CHARACTERS`.
pub(crate) const REVISION_FRAGMENT_CHARACTERS: usize = 12;
const MAX_TOOL_ID_CHARACTERS: usize = 48;
const MAX_OWNER_ID_CHARACTERS: usize = 64;

/// The Skill that owns a tool.
///
/// Deliberately a separate type from `skills::domain::SkillId` even though the lexical rule is
/// identical: a `tooling` subdomain may not reach into another subdomain's domain model, and the
/// duplication is one predicate rather than a shared mutable contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillToolOwnerId(String);

impl SkillToolOwnerId {
    pub(crate) fn parse(value: &str) -> Result<Self, SkillToolDomainError> {
        if is_normalized_id(value, MAX_OWNER_ID_CHARACTERS) {
            Ok(Self(value.to_string()))
        } else {
            Err(SkillToolDomainError::InvalidOwnerId(bounded(value)))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A tool's Skill-local id, as declared in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillToolId(String);

impl SkillToolId {
    pub(crate) fn parse(value: &str) -> Result<Self, SkillToolDomainError> {
        if is_normalized_id(value, MAX_TOOL_ID_CHARACTERS) {
            Ok(Self(value.to_string()))
        } else {
            Err(SkillToolDomainError::InvalidToolId(bounded(value)))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillToolScope {
    Global,
    Workspace,
}

impl SkillToolScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Workspace => "workspace",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "global" => Some(Self::Global),
            "workspace" => Some(Self::Workspace),
            _ => None,
        }
    }
}

/// Where the owning Skill package was resolved from. Workspace-scoped tools are keyed by their
/// workspace so the same Skill id in two projects never shares trust or quarantine state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillToolSourceScope {
    pub(crate) scope: SkillToolScope,
    pub(crate) workspace_path: Option<String>,
}

impl SkillToolSourceScope {
    pub(crate) fn new(
        scope: SkillToolScope,
        workspace_path: Option<&str>,
    ) -> Result<Self, SkillToolDomainError> {
        let workspace_path = match scope {
            SkillToolScope::Global => None,
            SkillToolScope::Workspace => Some(
                workspace_path
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .ok_or(SkillToolDomainError::WorkspacePathRequired)?
                    .to_string(),
            ),
        };
        Ok(Self {
            scope,
            workspace_path,
        })
    }

    pub(crate) fn global() -> Self {
        Self {
            scope: SkillToolScope::Global,
            workspace_path: None,
        }
    }

    pub(crate) fn storage_workspace_key(&self) -> &str {
        self.workspace_path.as_deref().unwrap_or("")
    }
}

/// The immutable content witness a tool revision is bound to.
///
/// Produced by `trust::revision_witness` from the base package revision plus every
/// integrity-bound value, so any change to manifest, module, or capability content produces a
/// different witness — which is what makes an existing trust decision stop applying.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillToolRevision(String);

impl SkillToolRevision {
    pub(crate) fn parse(value: &str) -> Result<Self, SkillToolDomainError> {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.len() == 64 && normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(normalized))
        } else {
            Err(SkillToolDomainError::InvalidRevisionWitness)
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn short_fragment(&self) -> &str {
        &self.0[..REVISION_FRAGMENT_CHARACTERS]
    }
}

/// The immutable internal identity of one tool revision. The model never sees this; it sees
/// `canonical_name`, which maps back to a key rather than being parsed as authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillToolKey {
    pub(crate) owner: SkillToolOwnerId,
    pub(crate) source: SkillToolSourceScope,
    pub(crate) tool: SkillToolId,
    pub(crate) revision: SkillToolRevision,
}

impl SkillToolKey {
    pub(crate) fn new(
        owner: SkillToolOwnerId,
        source: SkillToolSourceScope,
        tool: SkillToolId,
        revision: SkillToolRevision,
    ) -> Self {
        Self {
            owner,
            source,
            tool,
            revision,
        }
    }

    /// `skill__<skill-id>__<tool-id>__<short-revision>`.
    ///
    /// Ids are already normalized to `[a-z0-9-]`, so `__` cannot occur inside a segment and the
    /// name stays unambiguously decomposable. Failing on length instead of truncating keeps two
    /// long-named tools from encoding to the same provider-visible name.
    pub(crate) fn canonical_name(&self) -> Result<String, SkillToolDomainError> {
        let name = format!(
            "skill__{}__{}__{}",
            self.owner.as_str(),
            self.tool.as_str(),
            self.revision.short_fragment()
        );
        let actual = name.chars().count();
        if actual > MAX_CANONICAL_NAME_CHARACTERS {
            return Err(SkillToolDomainError::CanonicalNameTooLong {
                maximum: MAX_CANONICAL_NAME_CHARACTERS,
                actual,
            });
        }
        Ok(name)
    }

    /// Stable identity of the tool across revisions, for state that outlives one revision (usage
    /// counters, audit correlation). Trust, validation, and quarantine stay revision-bound.
    pub(crate) fn lineage_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.owner.as_str(),
            self.source.scope.as_str(),
            self.source.storage_workspace_key(),
            self.tool.as_str()
        )
    }
}

fn is_normalized_id(value: &str, maximum: usize) -> bool {
    if value.is_empty()
        || value.len() > maximum
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
    {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Keeps a rejected value out of logs at full length; ids are short, so anything long is either
/// an attack or a mistake and its tail carries no diagnostic value.
fn bounded(value: &str) -> String {
    value.chars().take(64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(fill: char) -> SkillToolRevision {
        SkillToolRevision::parse(&fill.to_string().repeat(64)).expect("revision")
    }

    #[test]
    fn ids_accept_only_normalized_kebab_case() {
        for value in ["review", "diff-summary", "a1", "x-9"] {
            assert_eq!(SkillToolId::parse(value).expect("valid").as_str(), value);
        }
        for value in ["", "-x", "x-", "a--b", "Review", "diff_summary", "diff/x"] {
            assert!(matches!(
                SkillToolId::parse(value),
                Err(SkillToolDomainError::InvalidToolId(_))
            ));
        }
        assert!(matches!(
            SkillToolId::parse(&"a".repeat(49)),
            Err(SkillToolDomainError::InvalidToolId(_))
        ));
        assert!(matches!(
            SkillToolOwnerId::parse("Code Review"),
            Err(SkillToolDomainError::InvalidOwnerId(_))
        ));
    }

    #[test]
    fn revision_witness_requires_a_full_sha256_digest() {
        assert_eq!(revision('a').as_str().len(), 64);
        assert_eq!(revision('a').short_fragment(), "aaaaaaaaaaaa");
        assert_eq!(
            SkillToolRevision::parse(&"A".repeat(64))
                .expect("uppercase normalizes")
                .as_str(),
            "a".repeat(64)
        );
        for value in ["", "abc", &"z".repeat(64), &"a".repeat(63)] {
            assert_eq!(
                SkillToolRevision::parse(value),
                Err(SkillToolDomainError::InvalidRevisionWitness)
            );
        }
    }

    #[test]
    fn canonical_names_are_revision_scoped_decomposable_and_length_bounded() {
        let key = SkillToolKey::new(
            SkillToolOwnerId::parse("code-review").expect("owner"),
            SkillToolSourceScope::global(),
            SkillToolId::parse("diff-summary").expect("tool"),
            revision('b'),
        );
        let name = key.canonical_name().expect("canonical name");
        assert_eq!(name, "skill__code-review__diff-summary__bbbbbbbbbbbb");
        assert_eq!(name.split("__").collect::<Vec<_>>().len(), 4);

        let other_revision = SkillToolKey::new(
            key.owner.clone(),
            key.source.clone(),
            key.tool.clone(),
            revision('c'),
        );
        assert_ne!(
            other_revision.canonical_name().expect("canonical name"),
            name
        );
        assert_eq!(other_revision.lineage_key(), key.lineage_key());

        let long = SkillToolKey::new(
            SkillToolOwnerId::parse(&"a".repeat(40)).expect("owner"),
            SkillToolSourceScope::global(),
            SkillToolId::parse(&"b".repeat(40)).expect("tool"),
            revision('d'),
        );
        assert!(matches!(
            long.canonical_name(),
            Err(SkillToolDomainError::CanonicalNameTooLong { maximum: 64, .. })
        ));
    }

    #[test]
    fn the_same_skill_in_two_workspaces_keys_separately() {
        let first = SkillToolSourceScope::new(SkillToolScope::Workspace, Some("  D:/a  "))
            .expect("workspace");
        let second =
            SkillToolSourceScope::new(SkillToolScope::Workspace, Some("D:/b")).expect("workspace");
        assert_eq!(first.storage_workspace_key(), "D:/a");
        assert_ne!(first, second);
        assert_eq!(
            SkillToolSourceScope::new(SkillToolScope::Workspace, Some("   ")),
            Err(SkillToolDomainError::WorkspacePathRequired)
        );
        assert_eq!(
            SkillToolSourceScope::new(SkillToolScope::Global, Some("ignored")).expect("global"),
            SkillToolSourceScope::global()
        );
        assert_eq!(
            SkillToolScope::parse("workspace"),
            Some(SkillToolScope::Workspace)
        );
        assert_eq!(SkillToolScope::parse("other"), None);
    }
}
