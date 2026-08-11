use super::document_lease::{
    DocumentLeaseManager, DocumentNotificationSink, IDLE_DOCUMENT_LEASE_TIMEOUT,
    MAX_DOCUMENT_LEASES,
};
use super::document_snapshot::DocumentAdmission;
use crate::contexts::code_intelligence::domain::models::{DocumentSyncMode, PositionEncoding};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct RecordingSink {
    notifications: Mutex<Vec<(&'static str, Value)>>,
}

impl RecordingSink {
    fn notifications(&self) -> Vec<(&'static str, Value)> {
        self.notifications.lock().expect("notifications").clone()
    }
}

#[async_trait]
impl DocumentNotificationSink for RecordingSink {
    async fn notify(&self, method: &'static str, params: Value) -> Result<(), String> {
        self.notifications
            .lock()
            .map_err(|_| "notification lock unavailable".to_owned())?
            .push((method, params));
        Ok(())
    }
}

#[tokio::test]
async fn first_use_opens_the_disk_document_with_language_and_initial_version() {
    let fixture = WorkspaceFixture::new("src/main.rs", "fn main() {}\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Incremental);

    let prepared = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("prepared");

    assert_eq!(prepared.version().value(), 1);
    assert_eq!(prepared.text(), "fn main() {}\n");
    assert_eq!(prepared.uri().scheme(), "file");
    let notifications = sink.notifications();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].0, "textDocument/didOpen");
    assert_eq!(notifications[0].1["textDocument"]["languageId"], "rust");
    assert_eq!(notifications[0].1["textDocument"]["version"], 1);
    assert_eq!(notifications[0].1["textDocument"]["text"], "fn main() {}\n");
}

#[tokio::test]
async fn unchanged_disk_content_reuses_the_open_lease_without_notification() {
    let fixture = WorkspaceFixture::new("src/main.ts", "export const value = 1;\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Full);

    manager
        .prepare("src/main.ts", Duration::ZERO)
        .await
        .expect("opened");
    let reused = manager
        .prepare("src/main.ts", Duration::from_secs(1))
        .await
        .expect("reused");

    assert_eq!(reused.version().value(), 1);
    assert_eq!(sink.notifications().len(), 1);
}

#[tokio::test]
async fn full_sync_sends_the_complete_changed_snapshot_and_increments_version() {
    let fixture = WorkspaceFixture::new("src/main.rs", "fn old() {}\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Full);
    manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("opened");
    fixture.write("src/main.rs", "fn changed() {}\n");

    let changed = manager
        .prepare("src/main.rs", Duration::from_secs(1))
        .await
        .expect("changed");

    assert_eq!(changed.version().value(), 2);
    let notifications = sink.notifications();
    assert_eq!(notifications[1].0, "textDocument/didChange");
    assert_eq!(notifications[1].1["textDocument"]["version"], 2);
    assert_eq!(
        notifications[1].1["contentChanges"],
        serde_json::json!([{ "text": "fn changed() {}\n" }])
    );
}

#[tokio::test]
async fn incremental_sync_sends_one_contiguous_utf16_replacement() {
    let fixture = WorkspaceFixture::new("src/main.ts", "const value = 'a😀z';\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Incremental);
    manager
        .prepare("src/main.ts", Duration::ZERO)
        .await
        .expect("opened");
    fixture.write("src/main.ts", "const value = 'aNEWz';\n");

    manager
        .prepare("src/main.ts", Duration::from_secs(1))
        .await
        .expect("changed");

    let notifications = sink.notifications();
    assert_eq!(
        notifications[1].1["contentChanges"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        notifications[1].1["contentChanges"][0],
        serde_json::json!({
            "range": {
                "start": { "line": 0, "character": 16 },
                "end": { "line": 0, "character": 18 }
            },
            "text": "NEW"
        })
    );
}

#[tokio::test]
async fn exact_mutation_invalidation_and_external_changes_are_synchronized() {
    let fixture = WorkspaceFixture::new("src/main.rs", "fn first() {}\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Full);
    manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("opened");

    fixture.write("src/main.rs", "fn agent_edit() {}\n");
    assert!(manager.invalidate("src/main.rs"));
    assert!(!manager.invalidate("src/other.rs"));
    manager
        .prepare("src/main.rs", Duration::from_secs(1))
        .await
        .expect("agent edit");

    fixture.write("src/main.rs", "fn external_edit() {}\n");
    let external = manager
        .prepare("src/main.rs", Duration::from_secs(2))
        .await
        .expect("external edit");

    assert_eq!(external.version().value(), 3);
    assert_eq!(sink.notifications().len(), 3);
}

#[tokio::test]
async fn exact_invalidation_forces_sync_even_when_a_write_preserves_bytes() {
    let fixture = WorkspaceFixture::new("src/main.rs", "fn main() {}\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Full);
    manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("opened");

    fixture.write("src/main.rs", "fn main() {}\n");
    assert!(manager.invalidate("src/main.rs"));
    let synchronized = manager
        .prepare("src/main.rs", Duration::from_secs(1))
        .await
        .expect("synchronized");

    assert_eq!(synchronized.version().value(), 2);
    assert_eq!(sink.notifications()[1].0, "textDocument/didChange");
}

#[tokio::test]
async fn idle_close_sends_did_close_and_releases_retained_text() {
    let fixture = WorkspaceFixture::new("src/main.rs", "fn main() {}\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Full);
    manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("opened");

    assert!(manager.retained_bytes() > 0);
    assert_eq!(
        manager
            .close_idle(IDLE_DOCUMENT_LEASE_TIMEOUT)
            .await
            .expect("closed"),
        1
    );
    assert_eq!(manager.retained_bytes(), 0);
    assert_eq!(sink.notifications()[1].0, "textDocument/didClose");
}

#[tokio::test]
async fn server_restart_discards_old_leases_and_reopens_at_initial_version() {
    let fixture = WorkspaceFixture::new("src/main.rs", "fn main() {}\n");
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Full);
    manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("opened");

    manager.server_restarted();
    assert_eq!(manager.retained_bytes(), 0);
    let reopened = manager
        .prepare("src/main.rs", Duration::from_secs(1))
        .await
        .expect("reopened");

    assert_eq!(reopened.version().value(), 1);
    let notifications = sink.notifications();
    assert_eq!(notifications.len(), 2);
    assert_eq!(notifications[1].0, "textDocument/didOpen");
}

#[tokio::test]
async fn server_stop_closes_every_lease_and_capacity_evicts_the_oldest() {
    let fixture = WorkspaceFixture::new("src/file0.rs", "fn value_0() {}\n");
    for index in 1..=MAX_DOCUMENT_LEASES {
        fixture.write(
            &format!("src/file{index}.rs"),
            &format!("fn value_{index}() {{}}\n"),
        );
    }
    let (mut manager, sink) = fixture.manager(DocumentSyncMode::Full);
    for index in 0..=MAX_DOCUMENT_LEASES {
        manager
            .prepare(
                &format!("src/file{index}.rs"),
                Duration::from_secs(index as u64),
            )
            .await
            .expect("prepared");
    }

    assert_eq!(manager.active_count(), MAX_DOCUMENT_LEASES);
    assert_eq!(
        sink.notifications()
            .iter()
            .filter(|(method, _)| *method == "textDocument/didClose")
            .count(),
        1
    );
    assert_eq!(
        manager.close_all().await.expect("closed"),
        MAX_DOCUMENT_LEASES
    );
    assert_eq!(manager.active_count(), 0);
    assert_eq!(manager.retained_bytes(), 0);
}

struct WorkspaceFixture {
    directory: tempfile::TempDir,
}

impl WorkspaceFixture {
    fn new(relative: &str, content: &str) -> Self {
        let fixture = Self {
            directory: tempfile::tempdir().expect("workspace"),
        };
        fixture.write(relative, content);
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(path, content).expect("file");
    }

    fn manager(&self, sync_mode: DocumentSyncMode) -> (DocumentLeaseManager, Arc<RecordingSink>) {
        let sink = Arc::new(RecordingSink::default());
        let manager = DocumentLeaseManager::new(
            DocumentAdmission::new(self.root()).expect("admission"),
            sync_mode,
            PositionEncoding::Utf16,
            sink.clone(),
        );
        (manager, sink)
    }
}
