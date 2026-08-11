use super::diagnostics_cache::{
    DiagnosticsCache, DiagnosticsReadiness, MAX_DIAGNOSTICS_PER_DOCUMENT,
    MAX_DIAGNOSTIC_MESSAGE_BYTES,
};
use super::document_lease::{DocumentLeaseManager, DocumentNotificationSink};
use super::document_snapshot::DocumentAdmission;
use crate::contexts::code_intelligence::domain::models::{
    DocumentSyncMode, PositionEncoding, QueryStatus,
};
use async_trait::async_trait;
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, Location, Position, PublishDiagnosticsParams, Range,
    Uri,
};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

struct NullSink;

#[async_trait]
impl DocumentNotificationSink for NullSink {
    async fn notify(&self, _method: &'static str, _params: Value) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn publications_replace_prior_snapshots_and_current_empty_is_ready() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let cache = fixture.cache();
    cache
        .publish(
            &document,
            params(&document, vec![diagnostic("old")], Some(1)),
            10,
        )
        .expect("first publication");
    cache
        .publish(&document, params(&document, Vec::new(), Some(1)), 20)
        .expect("replacement");

    let result = cache
        .wait_for_current(
            document.uri(),
            document.version(),
            DiagnosticsReadiness::Ready,
            Duration::from_millis(5),
        )
        .await;

    assert_eq!(result.status(), QueryStatus::Ready);
    assert!(!result.stale());
    assert_eq!(result.snapshot().expect("snapshot").diagnostics().len(), 0);
}

#[tokio::test]
async fn missing_server_version_is_current_for_the_local_version() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let cache = fixture.cache();
    cache
        .publish(
            &document,
            params(&document, vec![diagnostic("current")], None),
            10,
        )
        .expect("publication");

    let result = cache
        .wait_for_current(
            document.uri(),
            document.version(),
            DiagnosticsReadiness::Ready,
            Duration::from_millis(5),
        )
        .await;

    assert_eq!(result.status(), QueryStatus::Ready);
    assert!(!result.stale());
}

#[tokio::test]
async fn stale_local_snapshot_waits_only_until_the_bounded_deadline() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let first = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("first");
    let cache = fixture.cache();
    cache
        .publish(
            &first,
            params(&first, vec![diagnostic("stale")], Some(1)),
            10,
        )
        .expect("publication");
    fixture.write("fn changed() {}\n");
    let changed = manager
        .prepare("src/main.rs", Duration::from_secs(1))
        .await
        .expect("changed");

    let result = cache
        .wait_for_current(
            changed.uri(),
            changed.version(),
            DiagnosticsReadiness::Ready,
            Duration::from_millis(5),
        )
        .await;

    assert_eq!(result.status(), QueryStatus::Timeout);
    assert!(result.stale());
    assert_eq!(
        result.snapshot().expect("stale snapshot").diagnostics()[0].message,
        "stale"
    );
}

#[tokio::test]
async fn bounded_wait_completes_when_a_current_publication_arrives() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let cache = Arc::new(fixture.cache());
    let waiting_cache = cache.clone();
    let waiting_uri = document.uri().clone();
    let version = document.version();
    let waiting = tokio::spawn(async move {
        waiting_cache
            .wait_for_current(
                &waiting_uri,
                version,
                DiagnosticsReadiness::Ready,
                Duration::from_secs(1),
            )
            .await
    });
    tokio::task::yield_now().await;

    cache
        .publish(
            &document,
            params(&document, vec![diagnostic("arrived")], Some(1)),
            10,
        )
        .expect("publication");
    let result = waiting.await.expect("waiter");

    assert_eq!(result.status(), QueryStatus::Ready);
    assert_eq!(
        result.snapshot().expect("snapshot").diagnostics()[0].message,
        "arrived"
    );
}

#[tokio::test]
async fn related_locations_outside_the_workspace_are_filtered() {
    let fixture = Fixture::new();
    let outside = tempfile::tempdir().expect("outside");
    let outside_file = outside.path().join("outside.rs");
    std::fs::write(&outside_file, "fn outside() {}\n").expect("outside file");
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let mut item = diagnostic("message");
    item.related_information = Some(vec![DiagnosticRelatedInformation {
        location: Location {
            uri: file_uri(&outside_file),
            range: unit_range(),
        },
        message: "outside".to_owned(),
    }]);
    let cache = fixture.cache();

    let summary = cache
        .publish(&document, params(&document, vec![item], Some(1)), 10)
        .expect("publication");

    assert_eq!(summary.filtered_related_count(), 1);
    let snapshot = cache.snapshot(document.uri()).expect("snapshot");
    assert!(snapshot.diagnostics()[0].related_information.is_empty());
}

#[tokio::test]
async fn warming_server_returns_warming_without_masquerading_as_empty_ready() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let cache = fixture.cache();

    let result = cache
        .wait_for_current(
            document.uri(),
            document.version(),
            DiagnosticsReadiness::Warming,
            Duration::from_secs(1),
        )
        .await;

    assert_eq!(result.status(), QueryStatus::Warming);
    assert!(!result.stale());
    assert!(result.snapshot().is_none());
}

#[tokio::test]
async fn diagnostic_count_and_message_bytes_are_hard_bounded() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let diagnostics = (0..MAX_DIAGNOSTICS_PER_DOCUMENT)
        .map(|_| diagnostic(&"😀".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES)))
        .collect();
    let cache = fixture.cache();

    let summary = cache
        .publish(&document, params(&document, diagnostics, Some(1)), 10)
        .expect("publication");
    let snapshot = cache.snapshot(document.uri()).expect("snapshot");

    assert!(summary.truncated());
    assert_eq!(snapshot.diagnostics().len(), MAX_DIAGNOSTICS_PER_DOCUMENT);
    assert!(snapshot.diagnostics()[0].message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES);
    assert!(snapshot.diagnostics()[0]
        .message
        .is_char_boundary(snapshot.diagnostics()[0].message.len()));
}

#[tokio::test]
async fn diagnostic_cache_distinguishes_exact_and_over_limit_counts() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let cache = fixture.cache();
    let exact = (0..MAX_DIAGNOSTICS_PER_DOCUMENT)
        .map(|index| diagnostic(&format!("diagnostic-{index}")))
        .collect();

    let exact_summary = cache
        .publish(&document, params(&document, exact, Some(1)), 10)
        .expect("exact publication");
    assert!(!exact_summary.truncated());
    assert_eq!(
        cache
            .snapshot(document.uri())
            .expect("exact snapshot")
            .diagnostics()
            .len(),
        MAX_DIAGNOSTICS_PER_DOCUMENT
    );

    let over = (0..=MAX_DIAGNOSTICS_PER_DOCUMENT)
        .map(|index| diagnostic(&format!("diagnostic-{index}")))
        .collect();
    let over_summary = cache
        .publish(&document, params(&document, over, Some(1)), 11)
        .expect("over-limit publication");
    let snapshot = cache.snapshot(document.uri()).expect("bounded snapshot");

    assert!(over_summary.truncated());
    assert_eq!(snapshot.diagnostics().len(), MAX_DIAGNOSTICS_PER_DOCUMENT);
    assert_eq!(snapshot.diagnostics()[0].message, "diagnostic-0");
    assert_eq!(
        snapshot.diagnostics()[MAX_DIAGNOSTICS_PER_DOCUMENT - 1].message,
        format!("diagnostic-{}", MAX_DIAGNOSTICS_PER_DOCUMENT - 1)
    );
}

#[tokio::test]
async fn process_exit_clears_snapshots_and_wakes_waiters() {
    let fixture = Fixture::new();
    let mut manager = fixture.manager();
    let document = manager
        .prepare("src/main.rs", Duration::ZERO)
        .await
        .expect("document");
    let cache = fixture.cache();
    cache
        .publish(
            &document,
            params(&document, vec![diagnostic("old")], Some(1)),
            10,
        )
        .expect("publication");

    cache.clear_after_process_exit();
    let result = cache
        .wait_for_current(
            document.uri(),
            document.version(),
            DiagnosticsReadiness::Ready,
            Duration::from_millis(2),
        )
        .await;

    assert!(cache.snapshot(document.uri()).is_none());
    assert_eq!(result.status(), QueryStatus::Timeout);
    assert!(!result.stale());
    assert!(result.snapshot().is_none());
}

fn diagnostic(message: &str) -> Diagnostic {
    Diagnostic::new_simple(unit_range(), message.to_owned())
}

fn unit_range() -> Range {
    Range::new(Position::new(0, 0), Position::new(0, 1))
}

fn params(
    document: &super::document_lease::PreparedDocument,
    diagnostics: Vec<Diagnostic>,
    version: Option<i32>,
) -> PublishDiagnosticsParams {
    PublishDiagnosticsParams::new(
        document.uri().as_str().parse().expect("document URI"),
        diagnostics,
        version,
    )
}

fn file_uri(path: &Path) -> Uri {
    url::Url::from_file_path(path)
        .expect("file URL")
        .as_str()
        .parse()
        .expect("file URI")
}

struct Fixture {
    directory: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            directory: tempfile::tempdir().expect("workspace"),
        };
        fixture.write("fn main() {}\n");
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn write(&self, content: &str) {
        let path = self.root().join("src/main.rs");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, content).expect("file");
    }

    fn manager(&self) -> DocumentLeaseManager {
        DocumentLeaseManager::new(
            DocumentAdmission::new(self.root()).expect("admission"),
            DocumentSyncMode::Full,
            PositionEncoding::Utf16,
            Arc::new(NullSink),
        )
    }

    fn cache(&self) -> DiagnosticsCache {
        DiagnosticsCache::new(self.root(), PositionEncoding::Utf16).expect("cache")
    }
}
