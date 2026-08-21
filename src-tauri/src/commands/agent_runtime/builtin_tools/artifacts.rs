use crate::contexts::artifacts::api::{
    ArtifactBlobStorePolicy, ArtifactService, ArtifactVisibility,
};
use crate::contexts::artifacts::infrastructure::{ArtifactBlobStore, SqliteArtifactCatalog};
use crate::platform::database::NativeDatabase;
use crate::platform::filesystem::create_new_file;
use base64::Engine;
use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

pub(super) use super::artifacts_query::{artifact_detail, artifact_summary, read_all_artifact};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListArtifactsInput {
    session_id: String,
    cursor: Option<String>,
    limit: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadArtifactInput {
    artifact_id: String,
    offset: u64,
    length: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublishArtifactInput {
    artifact_id: String,
    expected_content_hash: String,
    acknowledgement: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DownloadArtifactInput {
    artifact_id: String,
    expected_content_hash: String,
}

#[tauri::command]
pub(crate) fn list_artifacts(
    database: State<'_, NativeDatabase>,
    input: ListArtifactsInput,
) -> Result<Value, String> {
    list_artifact_page(database.inner(), input)
}

fn list_artifact_page(
    database: &NativeDatabase,
    input: ListArtifactsInput,
) -> Result<Value, String> {
    super::require_feature("artifact", "read")?;
    let limit = input.limit.unwrap_or(50);
    if input.session_id.trim().is_empty() || !(1..=100).contains(&limit) {
        return Err("invalid_input".to_owned());
    }
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    let mut statement = connection
        .prepare(
            "SELECT a.id FROM native_tool_artifacts a \
             LEFT JOIN native_tool_operations o ON o.id = a.source_operation_id \
             WHERE (o.session_id = ?1 OR a.source_operation_id IS NULL) \
             AND (?2 IS NULL OR a.id < ?2) ORDER BY a.id DESC LIMIT ?3",
        )
        .map_err(|_| "storage_unavailable")?;
    let rows = statement
        .query_map(params![input.session_id, input.cursor, limit + 1], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|_| "storage_unavailable")?;
    let service = artifact_service(database)?;
    let mut items = Vec::new();
    for row in rows {
        let id = row.map_err(|_| "storage_unavailable")?;
        let descriptor = service.metadata(&id).map_err(map_artifact_error)?;
        items.push(artifact_summary(&service, &descriptor));
    }
    let next_cursor = if items.len() > limit as usize {
        items.pop().and_then(|item| item.get("id").cloned())
    } else {
        None
    };
    Ok(json!({"items": items, "nextCursor": next_cursor}))
}

#[tauri::command]
pub(crate) fn get_artifact(
    database: State<'_, NativeDatabase>,
    artifact_id: String,
) -> Result<Value, String> {
    get_artifact_detail(database.inner(), artifact_id)
}

fn get_artifact_detail(database: &NativeDatabase, artifact_id: String) -> Result<Value, String> {
    super::require_feature("artifact", "read")?;
    artifact_detail(database, &artifact_id)
}

#[tauri::command]
pub(crate) fn read_artifact(
    database: State<'_, NativeDatabase>,
    input: ReadArtifactInput,
) -> Result<Value, String> {
    read_artifact_chunk(database.inner(), input)
}

fn read_artifact_chunk(
    database: &NativeDatabase,
    input: ReadArtifactInput,
) -> Result<Value, String> {
    super::require_feature("artifact", "read")?;
    let service = artifact_service(database)?;
    let chunk = service
        .download_chunk(&input.artifact_id, input.offset, input.length)
        .map_err(map_artifact_error)?;
    Ok(json!({
        "artifactId": chunk.artifact_id,
        "offset": chunk.offset,
        "bytesBase64": base64::engine::general_purpose::STANDARD.encode(chunk.bytes),
        "nextOffset": chunk.next_offset,
        "contentHash": chunk.content_hash
    }))
}

#[tauri::command]
pub(crate) fn publish_artifact(
    database: State<'_, NativeDatabase>,
    input: PublishArtifactInput,
) -> Result<Value, String> {
    publish_artifact_reference(database.inner(), input)
}

fn publish_artifact_reference(
    database: &NativeDatabase,
    input: PublishArtifactInput,
) -> Result<Value, String> {
    super::require_feature("artifact", "publish")?;
    if !input.acknowledgement {
        return Err("explicit_acknowledgement_required".to_owned());
    }
    let service = artifact_service(database)?;
    let artifact = service
        .metadata(&input.artifact_id)
        .map_err(map_artifact_error)?;
    if artifact.content_hash != input.expected_content_hash {
        return Err("artifact_integrity_failure".to_owned());
    }
    service
        .publish(
            &input.artifact_id,
            ArtifactVisibility::Session,
            &chrono::Utc::now().to_rfc3339(),
        )
        .map_err(map_artifact_error)?;
    artifact_detail(database, &input.artifact_id)
}

#[tauri::command]
pub(crate) fn download_artifact(
    database: State<'_, NativeDatabase>,
    input: DownloadArtifactInput,
) -> Result<Value, String> {
    download_artifact_file(database.inner(), input)
}

fn download_artifact_file(
    database: &NativeDatabase,
    input: DownloadArtifactInput,
) -> Result<Value, String> {
    super::require_feature("artifact", "download")?;
    let service = artifact_service(database)?;
    let artifact = service
        .metadata(&input.artifact_id)
        .map_err(map_artifact_error)?;
    if artifact.content_hash != input.expected_content_hash {
        return Err("artifact_integrity_failure".to_owned());
    }
    let chunk = service
        .download_chunk(&artifact.id, 0, 1_048_576)
        .map_err(map_artifact_error)?;
    if chunk.next_offset.is_some() {
        return Err("artifact_download_too_large".to_owned());
    }
    let root = data_root(database)?.join("downloads");
    std::fs::create_dir_all(&root).map_err(|_| "storage_unavailable")?;
    let path = root.join(format!("{}-{}", Uuid::new_v4(), artifact.display_name));
    let mut file = create_new_file(&path).map_err(|_| "storage_unavailable")?;
    file.write_all(&chunk.bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "storage_unavailable")?;
    Ok(json!({
        "path": path.to_string_lossy(),
        "contentHash": chunk.content_hash
    }))
}

pub(super) fn artifact_service(database: &NativeDatabase) -> Result<ArtifactService, String> {
    Ok(ArtifactService::new(
        Arc::new(
            ArtifactBlobStore::new(
                data_root(database)?,
                ArtifactBlobStorePolicy {
                    max_blob_bytes: 256 * 1024 * 1024,
                    max_operation_items: 64,
                    max_operation_bytes: 512 * 1024 * 1024,
                    max_total_bytes: 4 * 1024 * 1024 * 1024,
                },
            )
            .map_err(|_| "artifact_storage_unavailable")?,
        ),
        Arc::new(SqliteArtifactCatalog::new(database.clone())),
    ))
}

fn data_root(database: &NativeDatabase) -> Result<&std::path::Path, String> {
    database
        .db_path
        .parent()
        .ok_or_else(|| "storage_unavailable".to_owned())
}

pub(super) fn map_artifact_error(error: impl std::fmt::Debug) -> String {
    let safe = format!("{error:?}");
    if safe.contains("NotFound") {
        "artifact_not_found".to_owned()
    } else if safe.contains("Integrity") {
        "artifact_integrity_failure".to_owned()
    } else if safe.contains("Invalid") {
        "invalid_input".to_owned()
    } else {
        "artifact_unavailable".to_owned()
    }
}
