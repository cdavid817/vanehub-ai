use super::overlay_manifest::{
    parse_overlay_manifest, serialize_overlay_manifest, FilesystemOverlayManifestRepository,
    OverlayManifestError,
};
use crate::contexts::tooling::skills::application::OverlayManifestRepository;
use crate::contexts::tooling::skills::domain::{
    OverlayBaseWitness, OverlayConflict, OverlayDocument, OverlayFile, OverlayLearnBlock,
    OverlayPatch, OverlayScope, OverlayTrust, SkillId, OVERLAY_SCHEMA_VERSION,
};
use crate::test_support::TempDirectory;

fn populated_document() -> OverlayDocument {
    let mut document = OverlayDocument::new(
        SkillId::parse("code-review").expect("valid Skill id"),
        OverlayScope::Project,
        Some("D:/work/canonical-project"),
        OverlayBaseWitness::new("system:code-review:v1", "instruction-hash", "package-hash")
            .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay document");
    document.patches.push(
        OverlayPatch::new(
            "patch-1",
            "old guidance",
            "new guidance",
            false,
            "instruction-hash",
            "2026-08-11T00:00:00Z",
        )
        .expect("patch"),
    );
    let mut guidance =
        OverlayLearnBlock::new("learn-1", "Prefer bounded retries.", "2026-08-11T00:00:00Z")
            .expect("guidance");
    guidance
        .disable("2026-08-11T00:05:00Z")
        .expect("disable guidance");
    document.learn_blocks.push(guidance);
    let mut file = OverlayFile::new(
        "file-1",
        "references/team.md",
        "text/markdown",
        18,
        "payload-content-hash",
        "sha256/payload-content-hash",
        "2026-08-11T00:00:00Z",
    )
    .expect("file");
    file.revert("2026-08-11T00:06:00Z").expect("revert file");
    document.files.push(file);
    let mut conflict = OverlayConflict::new(
        "conflict-1",
        "patch-1",
        "missing_exact_match",
        "instruction-hash",
    )
    .expect("conflict");
    conflict.resolve(2).expect("resolve conflict");
    document.conflicts.push(conflict);
    document
        .advance_revision("prior-document-hash", "2026-08-11T00:10:00Z")
        .expect("advance revision");
    document
}

#[test]
fn current_manifest_round_trips_every_governance_field() {
    let document = populated_document();

    let serialized = serialize_overlay_manifest(&document).expect("serialize manifest");
    let reparsed = parse_overlay_manifest(&serialized).expect("parse manifest");

    assert_eq!(reparsed, document);
    let json = String::from_utf8(serialized).expect("JSON is UTF-8");
    for field in [
        "\"schema_version\"",
        "\"canonical_skill_id\"",
        "\"base_instruction_hash\"",
        "\"prior_revision_hash\"",
        "\"patches\"",
        "\"learn_blocks\"",
        "\"files\"",
        "\"conflicts\"",
    ] {
        assert!(json.contains(field), "missing manifest field {field}");
    }
    assert!(!json.contains("SKILL.md"));
    assert!(!json.contains("base_instructions"));
}

#[test]
fn future_manifest_version_is_refused_before_document_decoding() {
    let future = format!(
        "{{\"schema_version\":{},\"future_only\":true}}",
        OVERLAY_SCHEMA_VERSION + 1
    );

    assert_eq!(
        parse_overlay_manifest(future.as_bytes()),
        Err(OverlayManifestError::UnsupportedFutureVersion {
            found: OVERLAY_SCHEMA_VERSION + 1,
            supported: OVERLAY_SCHEMA_VERSION,
        })
    );
}

#[test]
fn serializer_refuses_a_document_with_a_future_version() {
    let mut document = populated_document();
    document.schema_version = OVERLAY_SCHEMA_VERSION + 1;

    assert_eq!(
        serialize_overlay_manifest(&document),
        Err(OverlayManifestError::UnsupportedFutureVersion {
            found: OVERLAY_SCHEMA_VERSION + 1,
            supported: OVERLAY_SCHEMA_VERSION,
        })
    );
}

#[test]
fn current_manifest_rejects_unknown_fields_instead_of_silently_rewriting_them() {
    let serialized = serialize_overlay_manifest(&populated_document()).expect("serialize manifest");
    let mut value: serde_json::Value = serde_json::from_slice(&serialized).expect("JSON value");
    value["complete_base_skill"] = serde_json::Value::String("do not persist this".into());

    assert!(matches!(
        parse_overlay_manifest(&serde_json::to_vec(&value).expect("JSON bytes")),
        Err(OverlayManifestError::InvalidJson(_))
    ));
}

#[test]
fn repository_discovers_applicable_scopes_in_replay_order() {
    let home = TempDirectory::new("overlay-manifest-repository-home");
    let workspace = TempDirectory::new("overlay-manifest-repository-workspace");
    let workspace_identity = workspace.path().to_string_lossy().to_string();
    let skill_id = SkillId::parse("discovered-overlay").expect("Skill id");
    let home_root = home.path().join(".vanehub/skill_overlays");
    std::fs::create_dir_all(home_root.join("user")).expect("home Overlay roots");
    std::fs::create_dir_all(workspace.path().join(".vanehub/skills/.overlays"))
        .expect("Project Overlay root");
    for (scope, workspace, path) in [
        (
            OverlayScope::System,
            None,
            home_root.join("discovered-overlay.json"),
        ),
        (
            OverlayScope::User,
            None,
            home_root.join("user/discovered-overlay.json"),
        ),
        (
            OverlayScope::Project,
            Some(workspace_identity.as_str()),
            workspace
                .path()
                .join(".vanehub/skills/.overlays/discovered-overlay.json"),
        ),
    ] {
        let document = OverlayDocument::new(
            skill_id.clone(),
            scope,
            workspace,
            OverlayBaseWitness::new("base", "instructions", "package").expect("witness"),
            OverlayTrust::trusted_local(1),
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay document");
        std::fs::write(
            path,
            serialize_overlay_manifest(&document).expect("serialized manifest"),
        )
        .expect("manifest write");
    }
    let repository = FilesystemOverlayManifestRepository::with_home_root(home.path().to_path_buf());

    let applicable = repository
        .applicable(&skill_id, Some(&workspace_identity))
        .expect("applicable manifests");
    assert_eq!(
        applicable
            .iter()
            .map(|snapshot| snapshot.document.scope())
            .collect::<Vec<_>>(),
        vec![
            OverlayScope::System,
            OverlayScope::User,
            OverlayScope::Project
        ]
    );
    assert_eq!(
        repository
            .applicable(&skill_id, None)
            .expect("global manifests")
            .len(),
        2
    );
}
