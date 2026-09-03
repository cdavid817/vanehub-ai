use crate::{
    contexts::skill_evolution_orchestration::domain::EvolutionStageKind,
    platform::database::NativeDatabase, test_support::TempDirectory,
};

use super::{
    CommitStageItemOutcome, OrchestrationPersistenceError, OrchestrationRepository,
    ReserveStageItemOutcome,
};

#[test]
fn stage_item_reservation_and_commit_are_idempotent() {
    let (repository, database, _directory) = repository();
    seed_run(&database);
    let reserved = repository
        .reserve_stage_item("run-one", EvolutionStageKind::Assess, "seed-one", 7, 100)
        .expect("reserve");
    let item_id = match reserved {
        ReserveStageItemOutcome::Reserved { item_id, .. } => item_id,
        ReserveStageItemOutcome::Duplicate { .. } => panic!("expected reservation"),
    };
    assert_eq!(
        repository.commit_stage_item(&item_id, "assessment-receipt-one"),
        Ok(CommitStageItemOutcome::Committed {
            receipt_id: "assessment-receipt-one".into()
        })
    );
    assert_eq!(
        repository.commit_stage_item(&item_id, "assessment-receipt-one"),
        Ok(CommitStageItemOutcome::Duplicate {
            receipt_id: "assessment-receipt-one".into()
        })
    );
    assert_eq!(
        repository.commit_stage_item(&item_id, "different-receipt"),
        Err(OrchestrationPersistenceError::Conflict)
    );
    assert_eq!(
        repository.reserve_stage_item("run-one", EvolutionStageKind::Assess, "seed-one", 7, 200),
        Ok(ReserveStageItemOutcome::Duplicate {
            item_id,
            committed_receipt_id: Some("assessment-receipt-one".into())
        })
    );
}

#[test]
fn stage_and_source_revision_change_the_subsystem_idempotency_key() {
    let (repository, database, _directory) = repository();
    seed_run(&database);
    let mut keys = Vec::new();
    for (stage, revision) in [
        (EvolutionStageKind::BuildSeeds, 1),
        (EvolutionStageKind::Assess, 1),
        (EvolutionStageKind::Assess, 2),
    ] {
        match repository
            .reserve_stage_item("run-one", stage, "source-one", revision, 100)
            .expect("reserve")
        {
            ReserveStageItemOutcome::Reserved {
                idempotency_key, ..
            } => keys.push(idempotency_key),
            ReserveStageItemOutcome::Duplicate { .. } => panic!("unexpected duplicate"),
        }
    }
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 3);
}

fn repository() -> (OrchestrationRepository, NativeDatabase, TempDirectory) {
    let directory = TempDirectory::new("stage-items");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (
        OrchestrationRepository::new(database.clone()),
        database,
        directory,
    )
}

fn seed_run(database: &NativeDatabase) {
    let connection = database.connection().expect("connection");
    connection
        .execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace-one','runtime_trigger','claimed','{}',0,0,'run-one',0,1,1)", [])
        .expect("request");
    connection
        .execute("INSERT INTO evolution_runs VALUES ('run-one',1,'request-one','workspace-one','running','assess','policy','{}','{}',NULL,'worker-one',100,NULL,0,1,1)", [])
        .expect("run");
}
