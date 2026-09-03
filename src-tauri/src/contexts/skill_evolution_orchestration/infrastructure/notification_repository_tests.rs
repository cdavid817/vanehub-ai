use crate::{platform::database::NativeDatabase, test_support::TempDirectory};

use super::EvolutionNotificationRepository;

#[test]
fn notification_receipts_are_safe_deduplicated_and_delivery_is_terminal() {
    let directory = TempDirectory::new("orchestration-notification-receipts");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO evolution_auto_breakers VALUES
         ('breaker-one','workspace-one','skill-one','open','integrity_failure',NULL,NULL,
          'health-v1',0,NULL,100,100,2)",
            [],
        )
        .expect("breaker");
    drop(connection);

    let repository = EvolutionNotificationRepository::new(database);
    let events = repository.pending(101).expect("pending");
    assert_eq!(events.len(), 1);
    assert_eq!(repository.pending(101).expect("deduplicated").len(), 1);
    assert_eq!(events[0]["eventKind"], "breaker_opened");
    let serialized = events[0].to_string();
    for forbidden in ["prompt", "correction", "terminal", "toolArguments", "diff"] {
        assert!(!serialized.contains(forbidden));
    }
    repository
        .finish("breaker_opened:breaker-one:2", true, 102)
        .expect("finish");
    assert!(repository.pending(103).expect("after delivery").is_empty());
}
