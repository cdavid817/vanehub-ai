use crate::{
    contexts::skill_evolution_orchestration::domain::{
        EvolutionStageKind, EVOLUTION_STAGE_ORDER_V1,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::{CommitStageItemOutcome, OrchestrationRepository, ReserveStageItemOutcome};

#[test]
fn every_stage_dispatch_and_receipt_boundary_survives_database_reopen() {
    let directory = TempDirectory::new("orchestration-stage-crash-points");
    let mut database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    seed_run(&database);
    for (index, stage) in EVOLUTION_STAGE_ORDER_V1.into_iter().enumerate() {
        let source_id = format!("source-{index}");
        let receipt_id = format!("subsystem-receipt-{index}");
        let repository = OrchestrationRepository::new(database.clone());
        let item_id = match repository
            .reserve_stage_item("run-one", stage, &source_id, 1, index as i64)
            .expect("reserve before dispatch")
        {
            ReserveStageItemOutcome::Reserved { item_id, .. } => item_id,
            ReserveStageItemOutcome::Duplicate { .. } => panic!("unexpected initial duplicate"),
        };
        drop(repository);
        drop(database);
        database = reopen(&directory);
        let repository = OrchestrationRepository::new(database.clone());
        assert_eq!(
            repository.reserve_stage_item("run-one", stage, &source_id, 1, 100),
            Ok(ReserveStageItemOutcome::Duplicate {
                item_id: item_id.clone(),
                committed_receipt_id: None
            })
        );
        assert_eq!(
            repository.commit_stage_item(&item_id, &receipt_id),
            Ok(CommitStageItemOutcome::Committed {
                receipt_id: receipt_id.clone()
            })
        );
        drop(repository);
        drop(database);
        database = reopen(&directory);
        assert_eq!(
            OrchestrationRepository::new(database.clone())
                .reserve_stage_item("run-one", stage, &source_id, 1, 200,),
            Ok(ReserveStageItemOutcome::Duplicate {
                item_id,
                committed_receipt_id: Some(receipt_id)
            })
        );
    }
}

#[test]
fn overlay_commit_before_run_finalization_is_reconciled_without_reapplication() {
    let directory = TempDirectory::new("orchestration-overlay-commit-crash");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    seed_run(&database);
    let repository = OrchestrationRepository::new(database.clone());
    let item_id = match repository
        .reserve_stage_item(
            "run-one",
            EvolutionStageKind::EvaluateAutoApply,
            "application-one",
            1,
            100,
        )
        .expect("reserve application")
    {
        ReserveStageItemOutcome::Reserved { item_id, .. } => item_id,
        ReserveStageItemOutcome::Duplicate { .. } => panic!("unexpected duplicate"),
    };
    repository
        .commit_stage_item(&item_id, "overlay-application-one")
        .expect("overlay receipt");
    drop(repository);
    drop(database);

    let reopened = reopen(&directory);
    let repository = OrchestrationRepository::new(reopened);
    assert_eq!(
        repository.reserve_stage_item(
            "run-one",
            EvolutionStageKind::EvaluateAutoApply,
            "application-one",
            1,
            200,
        ),
        Ok(ReserveStageItemOutcome::Duplicate {
            item_id,
            committed_receipt_id: Some("overlay-application-one".into())
        })
    );
    assert_eq!(
        repository
            .run_resume_position("run-one")
            .expect("resume position")
            .next_stage,
        Some(EvolutionStageKind::Recover)
    );
}

fn reopen(directory: &TempDirectory) -> NativeDatabase {
    NativeDatabase::new(directory.path().to_path_buf()).expect("reopened database")
}

fn seed_run(database: &NativeDatabase) {
    let connection = database.connection().expect("connection");
    connection
        .execute("INSERT INTO evolution_run_requests VALUES ('request-one',1,'workspace-one','runtime_trigger','claimed','{}',0,0,'run-one',0,1,1)", [])
        .expect("request");
    connection
        .execute("INSERT INTO evolution_runs VALUES ('run-one',1,'request-one','workspace-one','running',NULL,'policy','{}','{}',NULL,'worker-one',100,NULL,0,1,1)", [])
        .expect("run");
}
