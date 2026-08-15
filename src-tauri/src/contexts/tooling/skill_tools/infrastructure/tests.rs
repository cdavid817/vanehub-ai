use super::{apply_trust, FilesystemSkillToolSource, SqliteSkillToolRepository};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolDiscoveryService, SkillToolPackageRef,
    SkillToolPackageSource, SkillToolRevisionState, SkillToolStateRepository,
};
use crate::contexts::tooling::skill_tools::domain::{
    content_hash_of, ContentHash, SkillToolDiagnostic, SkillToolDiagnosticSeverity,
    SkillToolDiagnosticSummary, SkillToolId, SkillToolIntegrity, SkillToolKey, SkillToolLifecycle,
    SkillToolOwnerId, SkillToolQuarantine, SkillToolRevision, SkillToolSourceScope,
    SkillToolTrustDecision, SkillToolTrustRecord, SkillToolValidationState,
    DEFAULT_MANIFEST_LIMITS,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::sync::Arc;

const DECLARATIVE_MANIFEST: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/valid-declarative.json");
const MODULE_MANIFEST: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/valid-module.json");

fn revision(fill: char) -> SkillToolRevision {
    SkillToolRevision::parse(&fill.to_string().repeat(64)).expect("revision")
}

fn integrity(capability_digest: &str) -> SkillToolIntegrity {
    SkillToolIntegrity {
        base_revision: "base-1".to_string(),
        manifest_hash: ContentHash::from_digest(&"a".repeat(64)),
        implementation_hash: ContentHash::from_digest(&"b".repeat(64)),
        capability_digest: capability_digest.to_string(),
    }
}

fn state(fill: char, capability_digest: &str) -> SkillToolRevisionState {
    SkillToolRevisionState {
        key: SkillToolKey::new(
            SkillToolOwnerId::parse("code-review").expect("owner"),
            SkillToolSourceScope::global(),
            SkillToolId::parse("diff-summary").expect("tool"),
            revision(fill),
        ),
        integrity: integrity(capability_digest),
        implementation_kind: "declarative".to_string(),
        lifecycle: SkillToolLifecycle::default(),
        validation_code: None,
        diagnostics: SkillToolDiagnosticSummary::default(),
        created_at: "2026-08-16T00:00:00Z".to_string(),
        updated_at: "2026-08-16T00:00:00Z".to_string(),
    }
}

fn repository(
    label: &str,
) -> (
    TempDirectory,
    SqliteSkillToolRepository,
    Arc<NativeDatabase>,
) {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("migrated native database"),
    );
    (
        directory,
        SqliteSkillToolRepository::new(Arc::clone(&database)),
        database,
    )
}

fn package(root: &str) -> SkillToolPackageRef {
    SkillToolPackageRef {
        owner: SkillToolOwnerId::parse("code-review").expect("owner"),
        source: SkillToolSourceScope::global(),
        base_revision: "base-1".to_string(),
        root_path: root.to_string(),
    }
}

#[test]
fn a_rediscovered_revision_keeps_the_governance_state_an_operator_already_decided() {
    let (_directory, repository, _database) = repository("skill-tool-rediscovery");
    let discovered = state('a', "digest-1");
    repository.record_discovered(&discovered).expect("insert");

    let mut lifecycle = SkillToolLifecycle {
        validation: SkillToolValidationState::Valid,
        trusted: true,
        enabled: true,
        ..SkillToolLifecycle::default()
    };
    repository
        .save_lifecycle(
            &discovered.key.revision,
            &lifecycle,
            Some("clean"),
            &SkillToolDiagnosticSummary::default(),
            "2026-08-16T01:00:00Z",
        )
        .expect("lifecycle");

    repository
        .record_discovered(&discovered)
        .expect("re-insert");
    let stored = repository
        .revision_state(&discovered.key.revision)
        .expect("read")
        .expect("row");
    assert!(stored.lifecycle.enabled);
    assert_eq!(stored.lifecycle.validation, SkillToolValidationState::Valid);
    assert_eq!(stored.validation_code.as_deref(), Some("clean"));
    assert_eq!(stored.updated_at, "2026-08-16T01:00:00Z");

    lifecycle.record_failure("module trapped");
    lifecycle.record_failure("module trapped");
    lifecycle.record_failure("module trapped");
    repository
        .save_lifecycle(
            &discovered.key.revision,
            &lifecycle,
            Some("clean"),
            &SkillToolDiagnosticSummary::from_entries(vec![SkillToolDiagnostic::new(
                SkillToolDiagnosticSeverity::Error,
                "limit-breach",
                "module trapped",
            )]),
            "2026-08-16T02:00:00Z",
        )
        .expect("quarantine");
    let quarantined = repository
        .revision_state(&discovered.key.revision)
        .expect("read")
        .expect("row");
    assert_eq!(quarantined.lifecycle.consecutive_failures, 3);
    assert!(quarantined.lifecycle.quarantine.is_quarantined());
    assert_eq!(
        quarantined.lifecycle.quarantine.reason(),
        Some("module trapped")
    );
    assert_eq!(quarantined.diagnostics.entries()[0].code, "limit-breach");
}

#[test]
fn trust_binds_to_content_and_a_changed_revision_is_not_authorized_by_it() {
    let (_directory, repository, _database) = repository("skill-tool-trust");
    let original = state('a', "digest-1");
    let replacement = state('c', "digest-2");
    repository.record_discovered(&original).expect("insert");
    repository.record_discovered(&replacement).expect("insert");

    let record = SkillToolTrustRecord {
        revision: original.key.revision.clone(),
        integrity: original.integrity.clone(),
        decision: SkillToolTrustDecision::Trusted,
        actor: "operator".to_string(),
        decided_at: "2026-08-16T03:00:00Z".to_string(),
    };
    repository
        .save_trust(&record, SkillToolTrustDecision::Trusted)
        .expect("trust");

    let mut trusted = repository
        .revision_state(&original.key.revision)
        .expect("read")
        .expect("row");
    let stored_trust = repository
        .trust_record(&original.key.revision)
        .expect("read")
        .expect("record");
    apply_trust(&mut trusted, Some(&stored_trust));
    assert!(trusted.lifecycle.trusted);

    let mut replaced = repository
        .revision_state(&replacement.key.revision)
        .expect("read")
        .expect("row");
    apply_trust(&mut replaced, Some(&stored_trust));
    assert!(!replaced.lifecycle.trusted);
    assert!(repository
        .trust_record(&replacement.key.revision)
        .expect("read")
        .is_none());
}

#[test]
fn revoking_trust_also_disables_the_revision_and_is_refused_for_unknown_content() {
    let (_directory, repository, _database) = repository("skill-tool-revoke");
    let discovered = state('a', "digest-1");
    repository.record_discovered(&discovered).expect("insert");
    repository
        .save_lifecycle(
            &discovered.key.revision,
            &SkillToolLifecycle {
                validation: SkillToolValidationState::Valid,
                trusted: true,
                enabled: true,
                ..SkillToolLifecycle::default()
            },
            None,
            &SkillToolDiagnosticSummary::default(),
            "2026-08-16T04:00:00Z",
        )
        .expect("enable");

    let record = SkillToolTrustRecord {
        revision: discovered.key.revision.clone(),
        integrity: discovered.integrity.clone(),
        decision: SkillToolTrustDecision::Revoked,
        actor: "operator".to_string(),
        decided_at: "2026-08-16T05:00:00Z".to_string(),
    };
    repository
        .save_trust(&record, SkillToolTrustDecision::Revoked)
        .expect("revoke");
    let stored = repository
        .revision_state(&discovered.key.revision)
        .expect("read")
        .expect("row");
    assert!(!stored.lifecycle.enabled);

    let orphan = SkillToolTrustRecord {
        revision: revision('f'),
        ..record
    };
    assert!(matches!(
        repository.save_trust(&orphan, SkillToolTrustDecision::Trusted),
        Err(SkillToolApplicationError::NotFound(_))
    ));
    assert!(matches!(
        repository.save_lifecycle(
            &revision('f'),
            &SkillToolLifecycle::default(),
            None,
            &SkillToolDiagnosticSummary::default(),
            "2026-08-16T06:00:00Z",
        ),
        Err(SkillToolApplicationError::NotFound(_))
    ));
}

#[test]
fn a_corrupt_row_is_skipped_instead_of_making_the_inventory_unreadable() {
    let (_directory, repository, database) = repository("skill-tool-corruption");
    repository
        .record_discovered(&state('a', "digest-1"))
        .expect("insert");
    repository
        .record_discovered(&state('c', "digest-2"))
        .expect("insert");

    let connection = database.connection().expect("connection");
    connection
        .execute(
            "UPDATE skill_tool_revisions SET manifest_hash = 'not-a-hash' WHERE revision_witness = ?1",
            params![revision('c').as_str()],
        )
        .expect("corrupt one row");

    let states = repository
        .revision_states(&package("ignored"))
        .expect("read inventory");
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].key.revision, revision('a'));
    assert!(repository
        .revision_state(&revision('c'))
        .expect("read")
        .is_none());
}

#[test]
fn the_migration_leaves_existing_skill_records_and_manifest_free_skills_untouched() {
    let directory = TempDirectory::new("skill-tool-migration-equivalence");
    let database =
        NativeDatabase::new(directory.path().to_path_buf()).expect("migrated native database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO skills (id, scope, workspace_path, source, enabled, skill_dir, \
             skill_md_path, content_hash, metadata_json, created_at, updated_at) \
             VALUES (?1, 'global', '', 'user-created', 1, ?2, ?3, 'hash', '{}', ?4, ?4)",
            params![
                "manifest-free-skill",
                "/managed/manifest-free-skill",
                "/managed/manifest-free-skill/SKILL.md",
                "2026-08-16T00:00:00Z"
            ],
        )
        .expect("existing Skill record");
    drop(connection);

    // Reopening re-runs `migrate`; the new tables must be idempotent and must not touch `skills`.
    let reopened = database.connection().expect("reopened connection");
    let preserved: i64 = reopened
        .query_row(
            "SELECT COUNT(*) FROM skills WHERE id = 'manifest-free-skill'",
            [],
            |row| row.get(0),
        )
        .expect("preserved Skill");
    let tool_rows: i64 = reopened
        .query_row("SELECT COUNT(*) FROM skill_tool_revisions", [], |row| {
            row.get(0)
        })
        .expect("tool rows");
    let trust_rows: i64 = reopened
        .query_row("SELECT COUNT(*) FROM skill_tool_trust", [], |row| {
            row.get(0)
        })
        .expect("trust rows");
    let migration: String = reopened
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 73",
            [],
            |row| row.get(0),
        )
        .expect("Skill tool migration");

    assert_eq!(preserved, 1);
    assert_eq!(tool_rows, 0);
    assert_eq!(trust_rows, 0);
    assert_eq!(migration, "skill-tool-runtime-foundation");
}

#[test]
fn the_filesystem_source_reads_a_contained_package_and_refuses_everything_else() {
    let directory = TempDirectory::new("skill-tool-filesystem");
    let root = directory.path().join("code-review");
    std::fs::create_dir_all(root.join("scripts/modules")).expect("package directories");
    std::fs::write(root.join("scripts/tools.json"), DECLARATIVE_MANIFEST).expect("manifest");
    std::fs::write(
        root.join("scripts/modules/complexity.wasm"),
        b"module bytes",
    )
    .expect("module");
    std::fs::write(root.join("scripts/notes.md"), "notes").expect("unrelated file");

    let source = FilesystemSkillToolSource::new();
    let package = package(&root.to_string_lossy());
    assert_eq!(
        source.read_manifest(&package).expect("manifest"),
        Some(DECLARATIVE_MANIFEST.as_bytes().to_vec())
    );
    assert_eq!(
        source
            .list_tool_files(&package)
            .expect("listing")
            .into_iter()
            .map(|entry| entry.relative_path)
            .collect::<Vec<_>>(),
        vec![
            "scripts/modules/complexity.wasm".to_string(),
            "scripts/tools.json".to_string()
        ]
    );
    assert_eq!(
        source
            .read_implementation(&package, "scripts/modules/complexity.wasm")
            .expect("module bytes"),
        b"module bytes".to_vec()
    );

    for escape in [
        "scripts/modules/../../../secret.txt",
        "../secret.txt",
        "/etc/passwd",
        "C:/Windows/System32/config",
        "scripts\\modules\\complexity.wasm",
        "references/guide.md",
        "scripts/notes.md",
        "scripts/modules/.hidden.wasm",
    ] {
        assert!(
            matches!(
                source.read_implementation(&package, escape),
                Err(SkillToolApplicationError::PathEscape(_))
            ),
            "{escape} must be refused"
        );
    }
}

#[test]
fn a_package_without_a_manifest_reports_absence_rather_than_a_filesystem_error() {
    let directory = TempDirectory::new("skill-tool-no-manifest");
    let root = directory.path().join("plain-skill");
    std::fs::create_dir_all(&root).expect("package directory");
    let source = FilesystemSkillToolSource::new();
    let package = package(&root.to_string_lossy());

    assert_eq!(source.read_manifest(&package).expect("manifest"), None);
    assert!(source
        .list_tool_files(&package)
        .expect("listing")
        .is_empty());

    let outcome = SkillToolDiscoveryService::new(&source, DEFAULT_MANIFEST_LIMITS)
        .discover(
            &crate::contexts::tooling::skill_tools::application::EffectiveSkillToolPackage {
                effective: package,
                shadowed: Vec::new(),
            },
        )
        .expect("discovery");
    assert!(!outcome.manifest_present);
    assert!(outcome.is_empty());
}

#[test]
fn discovery_over_the_real_filesystem_verifies_module_integrity_end_to_end() {
    let directory = TempDirectory::new("skill-tool-filesystem-discovery");
    let root = directory.path().join("code-review");
    std::fs::create_dir_all(root.join("scripts/modules")).expect("package directories");

    let mut module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let mut exports = vec![1_u8, 5];
    exports.extend_from_slice(b"score");
    exports.extend([0, 0]);
    module.extend([7_u8, exports.len() as u8]);
    module.extend(exports);

    let manifest = MODULE_MANIFEST.replace(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        content_hash_of(&module).as_str(),
    );
    std::fs::write(root.join("scripts/tools.json"), &manifest).expect("manifest");
    std::fs::write(root.join("scripts/modules/complexity.wasm"), &module).expect("module");

    let source = FilesystemSkillToolSource::new();
    let effective = crate::contexts::tooling::skill_tools::application::EffectiveSkillToolPackage {
        effective: package(&root.to_string_lossy()),
        shadowed: Vec::new(),
    };
    let outcome = SkillToolDiscoveryService::new(&source, DEFAULT_MANIFEST_LIMITS)
        .discover(&effective)
        .expect("discovery");
    assert_eq!(outcome.discovered.len(), 1);
    assert!(outcome.discovered[0].requires_module_runtime);
    assert!(outcome.undeclared_files.is_empty());

    std::fs::write(root.join("scripts/modules/complexity.wasm"), b"swapped").expect("swap");
    let swapped = SkillToolDiscoveryService::new(&source, DEFAULT_MANIFEST_LIMITS)
        .discover(&effective)
        .expect("discovery");
    assert!(swapped.discovered.is_empty());
    assert_eq!(swapped.rejected[0].diagnostic.code, "integrity-mismatch");
}

#[test]
fn a_persisted_quarantine_reason_never_carries_a_sensitive_detail_back_out() {
    let (_directory, repository, database) = repository("skill-tool-redaction");
    let discovered = state('a', "digest-1");
    repository.record_discovered(&discovered).expect("insert");
    repository
        .save_lifecycle(
            &discovered.key.revision,
            &SkillToolLifecycle {
                quarantine: SkillToolQuarantine::Quarantined {
                    reason: "blocked".to_string(),
                },
                ..SkillToolLifecycle::default()
            },
            None,
            &SkillToolDiagnosticSummary::default(),
            "2026-08-16T07:00:00Z",
        )
        .expect("quarantine");

    let connection = database.connection().expect("connection");
    connection
        .execute(
            "UPDATE skill_tool_revisions SET diagnostic_summary = ?2 WHERE revision_witness = ?1",
            params![
                discovered.key.revision.as_str(),
                "error|limit-breach|authorization: Bearer abc123"
            ],
        )
        .expect("smuggle a sensitive detail past the write path");

    let stored = repository
        .revision_state(&discovered.key.revision)
        .expect("read")
        .expect("row");
    assert_eq!(stored.diagnostics.entries()[0].detail, "[redacted]");
}
