use crate::contexts::skill_evolution_generation::{
    application::{
        dossier_builder_tests::{build, input, snapshot, witness},
        DossierSignalSourceV1,
    },
    domain::DossierSectionPageRequest,
};
use rusqlite::Connection;

use super::{
    apply_schema, GenerationDossierQuery, GenerationDossierRepository, GenerationPersistenceError,
};

#[test]
fn section_and_source_link_pagination_is_stable_and_hash_bound() {
    let connection = setup();
    let mut source = snapshot();
    source.signals = (0..3)
        .map(|index| DossierSignalSourceV1 {
            signal_id: format!("signal-{index}"),
            category: "failure".into(),
            occurred_at_ms: index,
            witness: witness("signal", &format!("signal-{index}")),
        })
        .collect();
    let dossier = build(&input(), &source).expect("dossier");
    GenerationDossierRepository::new(&connection)
        .persist(&dossier)
        .expect("persist");
    for index in 0..3 {
        connection
            .execute(
                "INSERT INTO evolution_evidence_dossier_links
                 (dossier_id,link_kind,linked_id,linked_revision,witness_hash)
                 VALUES (?1,'evidence',?2,'r1',?3)",
                rusqlite::params![
                    dossier.dossier_id,
                    format!("signal-{index}"),
                    format!("hash-{index}")
                ],
            )
            .expect("link");
    }
    let query = GenerationDossierQuery::new(&connection);
    let first = query
        .section_page(&DossierSectionPageRequest {
            dossier_id: &dossier.dossier_id,
            ordinal: 3,
            cursor: None,
            limit: 1,
        })
        .expect("first page");
    assert_eq!(first.records.len(), 1);
    assert!(!first.page_complete);
    let cursor = first.next_cursor.expect("cursor");
    let second = query
        .section_page(&DossierSectionPageRequest {
            dossier_id: &dossier.dossier_id,
            ordinal: 3,
            cursor: Some(&cursor),
            limit: 2,
        })
        .expect("second page");
    assert_eq!(second.records.len(), 2);
    assert!(second.page_complete);
    let links = query
        .source_links(&dossier.dossier_id, None, 2)
        .expect("links");
    assert_eq!(links.links.len(), 2);
    assert!(links.next_cursor.is_some());
    connection
        .execute(
            "UPDATE evolution_evidence_dossier_sections SET section_hash='sha256:changed'
             WHERE dossier_id=?1 AND ordinal=3",
            [&dossier.dossier_id],
        )
        .expect("simulate revision drift");
    assert_eq!(
        query.section_page(&DossierSectionPageRequest {
            dossier_id: &dossier.dossier_id,
            ordinal: 3,
            cursor: Some(&cursor),
            limit: 2,
        }),
        Err(GenerationPersistenceError::Conflict)
    );
}

fn setup() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE evolution_candidate_seeds (seed_id TEXT PRIMARY KEY);
             CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);
             CREATE TABLE evolution_curator_candidates (candidate_id TEXT PRIMARY KEY);",
        )
        .expect("dependencies");
    apply_schema(&connection).expect("schema");
    connection
}
