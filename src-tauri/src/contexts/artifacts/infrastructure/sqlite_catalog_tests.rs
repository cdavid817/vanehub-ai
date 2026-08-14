use super::*;
use crate::test_support::TempDirectory;

fn descriptor(id: &str, source_ids: Vec<String>) -> ArtifactDescriptor {
    ArtifactDescriptor {
        contract_version: 1,
        id: id.to_owned(),
        content_hash: format!(
            "sha256:{}",
            if id == "artifact-source" { "a" } else { "b" }.repeat(64)
        ),
        size_bytes: 5,
        media_type: "text/plain".to_owned(),
        display_name: format!("{id}.txt"),
        creator: ArtifactCreator {
            kind: "native_tool".to_owned(),
            id: "onepiece".to_owned(),
        },
        evidence_kind: ArtifactEvidenceKind::HostVerified,
        visibility: ArtifactVisibility::Private,
        source_operation_id: "operation-not-persisted".to_owned(),
        source_artifact_ids: source_ids,
        created_at: "2026-08-13T00:00:00Z".to_owned(),
        expires_at: Some("2026-08-14T00:00:00Z".to_owned()),
    }
}

#[test]
fn sqlite_catalog_round_trips_metadata_lineage_publication_and_references() {
    let root = TempDirectory::new("artifact-sqlite-catalog");
    let database = NativeDatabase::new(root.path().to_path_buf()).expect("database");
    let catalog = SqliteArtifactCatalog::new(database);
    let source = descriptor("artifact-source", Vec::new());
    catalog.insert_immutable(&source).expect("source");
    let derived = descriptor("artifact-derived", vec![source.id.clone()]);
    catalog.insert_immutable(&derived).expect("derived");

    let restored = catalog.get(&derived.id).expect("get").expect("artifact");
    assert_eq!(restored.creator, derived.creator);
    assert_eq!(restored.source_artifact_ids, vec![source.id.clone()]);
    let candidates = catalog
        .expired_candidates("2026-08-15T00:00:00Z", 10)
        .expect("expired");
    assert!(candidates
        .iter()
        .any(|(artifact, referenced)| artifact.id == source.id && *referenced));

    catalog
        .publish(&ArtifactPublicationReference {
            contract_version: 1,
            reference: "artifact-ref-one".to_owned(),
            artifact_id: derived.id.clone(),
            content_hash: derived.content_hash.clone(),
            visibility: ArtifactVisibility::Session,
            published_at: "2026-08-13T00:01:00Z".to_owned(),
        })
        .expect("publish");
    assert_eq!(
        catalog
            .get(&derived.id)
            .expect("get")
            .expect("artifact")
            .visibility,
        ArtifactVisibility::Session
    );
    assert_eq!(catalog.count_by_hash(&derived.content_hash), Ok(1));
}
