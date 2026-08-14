use super::*;
use crate::contexts::agent_runtime::application::{
    CanonicalToolResource, NativeToolExecutionContext, NativeToolProgress, NativeToolProgressSink,
    ToolResourceKind, ValidatedNativeToolInput,
};
use crate::contexts::artifacts::application::{
    ArtifactBlobStorePolicy, ArtifactCreateRequest, ArtifactEvidenceKind,
};
use crate::contexts::artifacts::infrastructure::{ArtifactBlobStore, SqliteArtifactCatalog};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct NoopProgress;

impl NativeToolProgressSink for NoopProgress {
    fn publish(&self, _progress: NativeToolProgress) {}
}

fn setup() -> (TempDirectory, Arc<ArtifactService>, ArtifactDescriptor) {
    let root = TempDirectory::new("artifact-native-tool-adapter");
    let database = NativeDatabase::new(root.path().to_path_buf()).expect("database");
    let blobs = ArtifactBlobStore::new(
        root.path(),
        ArtifactBlobStorePolicy {
            max_blob_bytes: 1024,
            max_operation_items: 10,
            max_operation_bytes: 4096,
            max_total_bytes: 4096,
        },
    )
    .expect("blob store");
    let service = Arc::new(ArtifactService::new(
        Arc::new(blobs),
        Arc::new(SqliteArtifactCatalog::new(database)),
    ));
    let descriptor = service
        .create_text(
            ArtifactCreateRequest {
                operation_id: "operation-1".to_owned(),
                display_name: "report.txt".to_owned(),
                media_type: "text/plain".to_owned(),
                creator: ArtifactCreator {
                    kind: "native_tool".to_owned(),
                    id: "onepiece".to_owned(),
                },
                evidence_kind: ArtifactEvidenceKind::HostVerified,
                visibility: ArtifactVisibility::Private,
                source_artifact_ids: Vec::new(),
                created_at: "2026-08-14T00:00:00Z".to_owned(),
                expires_at: None,
            },
            "hello artifact",
        )
        .expect("artifact");
    (root, service, descriptor)
}

fn request(value: Value, operation: NativeToolOperation) -> NativeToolPortRequest {
    NativeToolPortRequest {
        input: ValidatedNativeToolInput {
            value,
            input_hash: "sha256:test".to_owned(),
            operation,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Artifact,
                canonical_id: "artifact/test".to_owned(),
                attributes: BTreeMap::new(),
            },
        },
        context: NativeToolExecutionContext {
            call_id: "call-1".to_owned(),
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
            agent_id: "onepiece".to_owned(),
            canonical_workspace: None,
            deadline: Instant::now() + Duration::from_secs(5),
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(NoopProgress),
        },
    }
}

#[test]
fn adapter_lists_reads_and_publishes_without_exposing_storage_paths() {
    let (_root, service, descriptor) = setup();
    let adapter = ArtifactNativeToolAdapter::new(service);

    let list = adapter.execute_artifact(request(
        json!({"operation": "list", "limit": 10}),
        NativeToolOperation::ArtifactRead,
    ));
    let preview = adapter.execute_artifact(request(
        json!({"operation": "read_text", "artifact_id": descriptor.id.clone(), "limit": 5}),
        NativeToolOperation::ArtifactRead,
    ));
    let publish = adapter.execute_artifact(request(
        json!({"operation": "publish", "artifact_id": descriptor.id, "visibility": "session"}),
        NativeToolOperation::ArtifactPublish,
    ));

    assert_eq!(list.status, NativeToolResultStatus::Succeeded);
    assert_eq!(preview.status, NativeToolResultStatus::Succeeded);
    assert_eq!(publish.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        preview
            .output
            .as_ref()
            .and_then(|value| value["text"].as_str()),
        Some("hello")
    );
    let serialized = serde_json::to_string(&list.output).expect("serialize");
    assert!(!serialized.contains("artifact-blobs"));
    assert!(!serialized.contains(root_path_marker()));
}

#[test]
fn adapter_maps_missing_artifact_to_a_safe_stable_failure() {
    let (_root, service, _descriptor) = setup();
    let result = ArtifactNativeToolAdapter::new(service).execute_artifact(request(
        json!({"operation": "metadata", "artifact_id": "artifact-missing"}),
        NativeToolOperation::ArtifactRead,
    ));

    assert_eq!(result.status, NativeToolResultStatus::Failed);
    assert_eq!(result.error_code, Some(NativeToolErrorCode::Unavailable));
    assert_eq!(
        result.safe_error.as_deref(),
        Some("Artifact was not found.")
    );
}

fn root_path_marker() -> &'static str {
    "artifact-native-tool-adapter"
}
