use super::{
    ContentHash, DeclarativeFieldSource, SkillToolCapability, SkillToolDeclaration,
    SkillToolDomainError, SkillToolId, SkillToolImplementation, SkillToolOwnerId,
    SkillToolRevision, SkillToolSourceScope,
};
use sha2::{Digest, Sha256};

/// Every value a trust decision is bound to.
///
/// Trust is content trust, so all four move together: change any one and the derived revision
/// witness changes, which is what makes the previous decision stop applying rather than needing an
/// explicit invalidation sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolIntegrity {
    /// Revision witness of the owning Skill package, as resolved by effective Skill selection.
    pub(crate) base_revision: String,
    pub(crate) manifest_hash: ContentHash,
    pub(crate) implementation_hash: ContentHash,
    pub(crate) capability_digest: String,
}

/// Order-independent digest of the requested capabilities.
///
/// Sorting first means reordering a manifest's capability array does not invalidate trust, while
/// adding or removing one does — the operator is re-asked exactly when the upper bound changed.
pub(crate) fn capability_digest(capabilities: &[SkillToolCapability]) -> String {
    let mut declarations = capabilities
        .iter()
        .map(SkillToolCapability::as_declaration)
        .collect::<Vec<_>>();
    declarations.sort();
    declarations.dedup();
    hex_digest(declarations.join("\n").as_bytes())
}

/// Content hash of a declarative implementation.
///
/// Declarative tools ship no separate file, so without this their "implementation hash" would be
/// a constant and retargeting a template to a different operation would not invalidate trust.
pub(crate) fn declarative_implementation_hash(
    implementation: &SkillToolImplementation,
) -> ContentHash {
    match implementation {
        SkillToolImplementation::Module(module) => module.content_hash.clone(),
        SkillToolImplementation::Declarative(declarative) => {
            let mut canonical = String::from("declarative\n");
            canonical.push_str(&declarative.target.as_declaration());
            for field in &declarative.template {
                canonical.push('\n');
                canonical.push_str(&field.name);
                canonical.push('=');
                match &field.source {
                    DeclarativeFieldSource::Input(property) => {
                        canonical.push_str("$input:");
                        canonical.push_str(property);
                    }
                    DeclarativeFieldSource::Constant(value) => {
                        canonical.push_str("$const:");
                        canonical.push_str(&value.to_string());
                    }
                }
            }
            ContentHash::from_digest(&hex_digest(canonical.as_bytes()))
        }
    }
}

/// Derives the immutable revision witness one tool revision is identified by.
///
/// Includes identity as well as content so two Skills that ship byte-identical tools still hold
/// separate trust, quarantine, and audit state.
pub(crate) fn revision_witness(
    owner: &SkillToolOwnerId,
    source: &SkillToolSourceScope,
    tool: &SkillToolId,
    integrity: &SkillToolIntegrity,
) -> Result<SkillToolRevision, SkillToolDomainError> {
    let canonical = [
        owner.as_str(),
        source.scope.as_str(),
        source.storage_workspace_key(),
        tool.as_str(),
        integrity.base_revision.as_str(),
        integrity.manifest_hash.as_str(),
        integrity.implementation_hash.as_str(),
        integrity.capability_digest.as_str(),
    ]
    .join("\u{1f}");
    SkillToolRevision::parse(&hex_digest(canonical.as_bytes()))
}

/// Builds the integrity witness for one declared tool from the values discovery observed.
pub(crate) fn integrity_for(
    declaration: &SkillToolDeclaration,
    base_revision: &str,
    manifest_hash: &ContentHash,
) -> SkillToolIntegrity {
    SkillToolIntegrity {
        base_revision: base_revision.to_string(),
        manifest_hash: manifest_hash.clone(),
        implementation_hash: declarative_implementation_hash(&declaration.implementation),
        capability_digest: capability_digest(&declaration.capabilities),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolTrustDecision {
    Trusted,
    Revoked,
}

impl SkillToolTrustDecision {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Revoked => "revoked",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "trusted" => Some(Self::Trusted),
            "revoked" => Some(Self::Revoked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolTrustRecord {
    pub(crate) revision: SkillToolRevision,
    pub(crate) integrity: SkillToolIntegrity,
    pub(crate) decision: SkillToolTrustDecision,
    pub(crate) actor: String,
    pub(crate) decided_at: String,
}

impl SkillToolTrustRecord {
    /// Whether this record still authorizes the observed revision to be instantiated.
    ///
    /// Re-checks every bound value rather than trusting the revision witness alone: the witness is
    /// derived from these values, so a mismatch means either a hash collision attempt or a stored
    /// record that drifted, and both must fail closed.
    pub(crate) fn authorizes(
        &self,
        revision: &SkillToolRevision,
        integrity: &SkillToolIntegrity,
    ) -> bool {
        self.decision == SkillToolTrustDecision::Trusted
            && &self.revision == revision
            && &self.integrity == integrity
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Content hash of arbitrary bytes, in the manifest's declared hash form.
pub(crate) fn content_hash_of(bytes: &[u8]) -> ContentHash {
    ContentHash::from_digest(&hex_digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::{
        parse_manifest_bytes, SkillToolScope, DEFAULT_MANIFEST_LIMITS,
    };

    fn manifest_bytes(target: &str, constant: &str) -> Vec<u8> {
        format!(
            r#"{{
              "version": 1,
              "skillId": "code-review",
              "tools": [{{
                "id": "diff-summary",
                "description": "Summarize a diff",
                "capabilities": ["tool:{target}"],
                "input": {{ "type": "object", "properties": {{ "path": {{ "type": "string" }} }} }},
                "output": {{ "type": "object" }},
                "implementation": {{
                  "kind": "declarative",
                  "target": "tool:{target}",
                  "template": {{
                    "path": {{ "$input": "path" }},
                    "encoding": {{ "$const": "{constant}" }}
                  }}
                }}
              }}]
            }}"#
        )
        .into_bytes()
    }

    fn witness_for(bytes: &[u8], base_revision: &str) -> (SkillToolIntegrity, SkillToolRevision) {
        let manifest = parse_manifest_bytes(bytes, &DEFAULT_MANIFEST_LIMITS).expect("manifest");
        let declaration = &manifest.tools[0];
        let integrity = integrity_for(declaration, base_revision, &content_hash_of(bytes));
        let source = SkillToolSourceScope::global();
        let revision = revision_witness(&manifest.owner, &source, &declaration.id, &integrity)
            .expect("revision witness");
        (integrity, revision)
    }

    #[test]
    fn capability_digest_ignores_order_but_not_membership() {
        let read = SkillToolCapability::parse("tool:read_file").expect("capability");
        let write = SkillToolCapability::parse("tool:write_file").expect("capability");
        assert_eq!(
            capability_digest(&[read.clone(), write.clone()]),
            capability_digest(&[write.clone(), read.clone()])
        );
        assert_ne!(
            capability_digest(std::slice::from_ref(&read)),
            capability_digest(&[read, write])
        );
    }

    #[test]
    fn any_integrity_bound_change_produces_a_new_revision_and_drops_trust() {
        let baseline = manifest_bytes("read_file", "utf-8");
        let (integrity, revision) = witness_for(&baseline, "base-1");
        let record = SkillToolTrustRecord {
            revision: revision.clone(),
            integrity: integrity.clone(),
            decision: SkillToolTrustDecision::Trusted,
            actor: "operator".to_string(),
            decided_at: "2026-08-16T00:00:00Z".to_string(),
        };
        assert!(record.authorizes(&revision, &integrity));

        for changed in [
            witness_for(&manifest_bytes("read_file", "utf-16"), "base-1"),
            witness_for(&manifest_bytes("write_file", "utf-8"), "base-1"),
            witness_for(&baseline, "base-2"),
        ] {
            let (changed_integrity, changed_revision) = changed;
            assert_ne!(changed_revision, revision);
            assert!(!record.authorizes(&changed_revision, &changed_integrity));
        }
    }

    #[test]
    fn revoked_and_mismatched_records_never_authorize() {
        let baseline = manifest_bytes("read_file", "utf-8");
        let (integrity, revision) = witness_for(&baseline, "base-1");
        let revoked = SkillToolTrustRecord {
            revision: revision.clone(),
            integrity: integrity.clone(),
            decision: SkillToolTrustDecision::Revoked,
            actor: "operator".to_string(),
            decided_at: "2026-08-16T00:00:00Z".to_string(),
        };
        assert!(!revoked.authorizes(&revision, &integrity));

        // A record whose stored integrity was tampered with while its witness was kept.
        let forged = SkillToolTrustRecord {
            integrity: SkillToolIntegrity {
                capability_digest: capability_digest(&[SkillToolCapability::parse(
                    "tool:run_command",
                )
                .expect("capability")]),
                ..integrity.clone()
            },
            decision: SkillToolTrustDecision::Trusted,
            ..revoked
        };
        assert!(!forged.authorizes(&revision, &integrity));
        assert_eq!(
            SkillToolTrustDecision::parse("trusted"),
            Some(SkillToolTrustDecision::Trusted)
        );
        assert_eq!(SkillToolTrustDecision::parse("unknown"), None);
    }

    #[test]
    fn identical_content_in_two_scopes_stays_separately_keyed() {
        let bytes = manifest_bytes("read_file", "utf-8");
        let manifest = parse_manifest_bytes(&bytes, &DEFAULT_MANIFEST_LIMITS).expect("manifest");
        let declaration = &manifest.tools[0];
        let integrity = integrity_for(declaration, "base-1", &content_hash_of(&bytes));
        let global = revision_witness(
            &manifest.owner,
            &SkillToolSourceScope::global(),
            &declaration.id,
            &integrity,
        )
        .expect("global witness");
        let workspace = revision_witness(
            &manifest.owner,
            &SkillToolSourceScope::new(SkillToolScope::Workspace, Some("D:/work"))
                .expect("workspace"),
            &declaration.id,
            &integrity,
        )
        .expect("workspace witness");
        assert_ne!(global, workspace);
    }
}
