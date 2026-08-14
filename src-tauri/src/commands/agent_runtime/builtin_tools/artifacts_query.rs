use super::artifacts::{artifact_service, map_artifact_error};
use crate::contexts::artifacts::application::{ArtifactDescriptor, ArtifactService};
use crate::platform::database::NativeDatabase;
use serde_json::{json, Value};

pub(super) fn artifact_detail(
    database: &NativeDatabase,
    artifact_id: &str,
) -> Result<Value, String> {
    let service = artifact_service(database)?;
    let artifact = service.metadata(artifact_id).map_err(map_artifact_error)?;
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    let publication = connection
        .query_row(
            "SELECT publication_ref FROM native_tool_artifacts WHERE id = ?1",
            [artifact_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|_| "storage_unavailable")?;
    let mut statement = connection
        .prepare(
            "SELECT source_artifact_id FROM native_tool_artifact_lineage \
             WHERE artifact_id = ?1 ORDER BY ordinal",
        )
        .map_err(|_| "storage_unavailable")?;
    let lineage = statement
        .query_map([artifact_id], |row| row.get::<_, String>(0))
        .map_err(|_| "storage_unavailable")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "storage_unavailable")?;
    let mut value = artifact_summary(&service, &artifact);
    let object = value.as_object_mut().ok_or("storage_unavailable")?;
    object.insert(
        "producerOperationId".to_owned(),
        Value::String(artifact.source_operation_id),
    );
    object.insert("provenance".to_owned(), json!(lineage));
    object.insert(
        "publishedAt".to_owned(),
        publication
            .as_ref()
            .map_or(Value::Null, |_| json!(artifact.created_at)),
    );
    object.insert("publicationUrl".to_owned(), publication.into());
    object.insert("limitations".to_owned(), json!([]));
    Ok(value)
}

pub(super) fn read_all_artifact(
    database: &NativeDatabase,
    artifact_id: &str,
    maximum: usize,
) -> Result<Vec<u8>, String> {
    let service = artifact_service(database)?;
    let mut offset = 0_u64;
    let mut bytes = Vec::new();
    loop {
        let remaining = maximum.saturating_sub(bytes.len());
        if remaining == 0 {
            return Err("artifact_too_large".to_owned());
        }
        let chunk = service
            .download_chunk(artifact_id, offset, remaining.min(1_048_576))
            .map_err(map_artifact_error)?;
        bytes.extend_from_slice(&chunk.bytes);
        match chunk.next_offset {
            Some(next) => offset = next,
            None => return Ok(bytes),
        }
    }
}

pub(super) fn artifact_summary(service: &ArtifactService, artifact: &ArtifactDescriptor) -> Value {
    let integrity = if service.download_chunk(&artifact.id, 0, 1).is_ok() {
        "verified"
    } else {
        "failed"
    };
    json!({
        "id": artifact.id,
        "displayName": artifact.display_name,
        "mediaType": artifact.media_type,
        "sizeBytes": artifact.size_bytes,
        "contentHash": artifact.content_hash,
        "integrity": integrity,
        "createdAt": artifact.created_at,
        "expiresAt": artifact.expires_at,
        "simulated": false
    })
}
