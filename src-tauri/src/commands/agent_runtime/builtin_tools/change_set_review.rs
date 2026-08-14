use super::artifacts::{artifact_detail, read_all_artifact};
use crate::platform::database::NativeDatabase;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::State;

const MAX_CHANGE_SET_BYTES: usize = 48 * 1024 * 1024;

#[tauri::command]
pub(crate) fn get_change_set_review(
    database: State<'_, NativeDatabase>,
    artifact_id: String,
) -> Result<Value, String> {
    review_change_set(database.inner(), artifact_id)
}

fn review_change_set(database: &NativeDatabase, artifact_id: String) -> Result<Value, String> {
    super::require_feature("delegation", "read")?;
    let connection = database.connection().map_err(|_| "storage_unavailable")?;
    let record = connection
        .query_row(
            "SELECT content_hash, repository_identity, base_commit, warnings_json \
             FROM native_tool_change_sets WHERE artifact_id = ?1",
            [&artifact_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "change_set_not_found".to_owned(),
            _ => "storage_unavailable".to_owned(),
        })?;
    let bytes = read_all_artifact(database, &artifact_id, MAX_CHANGE_SET_BYTES)?;
    if sha256(&bytes) != record.0 {
        return Err("change_set_integrity_failure".to_owned());
    }
    let manifest: Value =
        serde_json::from_slice(&bytes).map_err(|_| "change_set_schema_invalid".to_owned())?;
    let repository = required_string(&manifest, "repository_identity")?;
    let base_commit = required_string(&manifest, "base_commit")?;
    let diff_hash = required_string(&manifest, "diff_hash")?;
    if manifest.get("schema_version").and_then(Value::as_u64) != Some(1)
        || repository != record.1
        || base_commit != record.2
    {
        return Err("change_set_integrity_failure".to_owned());
    }
    let patch = STANDARD
        .decode(required_string(&manifest, "patch_base64")?)
        .map_err(|_| "change_set_schema_invalid".to_owned())?;
    if sha256(&patch) != diff_hash {
        return Err("change_set_integrity_failure".to_owned());
    }
    let files = manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "change_set_schema_invalid".to_owned())?
        .iter()
        .map(file_summary)
        .collect::<Result<Vec<_>, _>>()?;
    let diff_text = String::from_utf8(patch)
        .unwrap_or_else(|error| format!("base64:{}", STANDARD.encode(error.into_bytes())));
    let warnings = serde_json::from_str::<Vec<String>>(&record.3).unwrap_or_default();
    Ok(json!({
        "artifact": artifact_detail(database, &artifact_id)?,
        "repositoryIdentity": repository,
        "baseCommit": base_commit,
        "diffHash": diff_hash,
        "files": files,
        "diffText": diff_text,
        "riskClassification": required_string(&manifest, "risk_classification")?,
        "applyable": manifest.get("applyable").and_then(Value::as_bool) == Some(true)
            && warnings.is_empty()
    }))
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| "change_set_schema_invalid".to_owned())
}

fn file_summary(value: &Value) -> Result<String, String> {
    let path = required_string(value, "path")?;
    let kind = required_string(value, "kind")?;
    let binary = if value.get("binary").and_then(Value::as_bool) == Some(true) {
        ", binary"
    } else {
        ""
    };
    Ok(format!("{path} ({kind}{binary})"))
}

fn sha256(bytes: &[u8]) -> String {
    let hex = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}
